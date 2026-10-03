//! The relay's arming, the executor exchanges it can withhold, and the status it reports.
//!
//! - **Role:** [`DropPolicy::decide`] looks at one forwarded exchange and either lets its answer
//!   through or spends the arming on it and returns the [`DropRecord`] of the withheld answer;
//!   [`DropPolicy::status`] reports the arming, the counts and the last drop as a [`RelayStatus`].
//! - **Position:** under [`super`]; the relay asks it once per answered exchange, and the control
//!   socket arms, disarms and reads it.
//! - **Signals & state:** one mutex over the arming, the forwarded and dropped counts and the last
//!   drop, for the life of the process.
//! - **Invariants:**
//!   - Only a `200` answer to a claim (when armed for claims) or to a result report (when armed
//!     for results) is withheld; a `204` claim answer and every other status pass through and keep
//!     the arming.
//!   - Deciding and disarming happen under one lock, so one arming withholds exactly one answer.
//!   - A record holds the command id and the fencing token only: never a header or a body.
//!
//! @contract fleet-command.schema.json#/definitions/ClaimedFleetCommand
//! @contract fleet-command.schema.json#/definitions/ExecutionResult

use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use axum::http::{Method, StatusCode};
use serde::{Deserialize, Serialize};
use time_source::{Clock, SystemClock};

/// Every executor route starts with this path.
const EXECUTOR_COMMANDS_PREFIX: &str = "/api/v1/fleet-executor/commands/";

/// The answer the next drop withholds; the words are the `control arm` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum DropTarget {
    /// The next `200` answer to a claim.
    DropNextClaimResponse,
    /// The next `200` answer to a result report.
    DropNextResultResponse,
}

impl DropTarget {
    /// The word `control arm` takes and `status` prints.
    pub fn word(self) -> &'static str {
        match self {
            Self::DropNextClaimResponse => "drop-next-claim-response",
            Self::DropNextResultResponse => "drop-next-result-response",
        }
    }

    /// The target `word` names.
    pub fn from_word(word: &str) -> Option<Self> {
        [Self::DropNextClaimResponse, Self::DropNextResultResponse]
            .into_iter()
            .find(|target| target.word() == word)
    }
}

/// The relay's arming as `status` reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Arming {
    /// Every answer passes through.
    Disarmed,
    /// The next `200` claim answer is withheld.
    DropNextClaimResponse,
    /// The next `200` result answer is withheld.
    DropNextResultResponse,
}

impl From<Option<DropTarget>> for Arming {
    fn from(armed: Option<DropTarget>) -> Self {
        match armed {
            None => Self::Disarmed,
            Some(DropTarget::DropNextClaimResponse) => Self::DropNextClaimResponse,
            Some(DropTarget::DropNextResultResponse) => Self::DropNextResultResponse,
        }
    }
}

/// Which executor answer a drop withheld.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutorResponse {
    /// The answer to `POST /api/v1/fleet-executor/commands/claim`.
    Claim,
    /// The answer to `POST /api/v1/fleet-executor/commands/{commandId}/result`.
    Result,
}

newtype_ids::string_id! {
    /// The id of a fleet command, as a claim answer or a result route names it.
    pub struct FleetCommandId;
}

/// One withheld answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DropRecord {
    /// Which answer was withheld.
    pub response: ExecutorResponse,
    /// The command: the claimed command's `command_id`, or the result route's command segment;
    /// `None` when a claim answer does not name one.
    pub command_id: Option<FleetCommandId>,
    /// The claimed command's fencing token, or the one the result report carried; `None` when
    /// the document does not hold one.
    pub fencing_token: Option<i64>,
    /// The status the upstream answered with, always `200`.
    pub upstream_status: u16,
    /// When the relay began to withhold the answer, in Unix milliseconds.
    pub withheld_at_unix_ms: u64,
}

impl DropRecord {
    /// The log line announcing this drop, for a hold of `withhold`.
    pub(super) fn announcement(&self, withhold: Duration) -> String {
        let exchange = match self.response {
            ExecutorResponse::Claim => "claim",
            ExecutorResponse::Result => "result report",
        };
        let command = self
            .command_id
            .as_ref()
            .map_or("(unnamed)", FleetCommandId::as_str);
        let token = self
            .fencing_token
            .map_or_else(|| "(none)".to_string(), |token| token.to_string());
        format!(
            "withholding the {} answer to the {exchange} of command {command} (fencing token \
             {token}) for {} ms, then closing its connection",
            self.upstream_status,
            withhold.as_millis()
        )
    }
}

/// What `control` prints: the arming, the relay's addresses, the counts and the last drop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayStatus {
    /// The current arming.
    pub arming: Arming,
    /// The address the relay listens on.
    pub listen: String,
    /// The origin the relay forwards to.
    pub upstream: String,
    /// How long a withheld answer is held, in milliseconds.
    pub withhold_milliseconds: u64,
    /// Upstream answers passed back unchanged since the relay started.
    pub forwarded_count: u64,
    /// Answers withheld since the relay started.
    pub drop_count: u64,
    /// The most recent withheld answer.
    pub last_drop: Option<DropRecord>,
}

/// The executor exchanges the relay can withhold, recognised by method and path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ExecutorExchange {
    /// A claim.
    Claim,
    /// A result report for `command_id`.
    Result { command_id: String },
}

impl ExecutorExchange {
    /// The executor exchange a request with `method` and `path` is, if it is one.
    ///
    /// @route POST /api/v1/fleet-executor/commands/claim
    /// @route POST /api/v1/fleet-executor/commands/:commandId/result
    pub(super) fn classify(method: &Method, path: &str) -> Option<Self> {
        if method != Method::POST {
            return None;
        }
        let rest = path.strip_prefix(EXECUTOR_COMMANDS_PREFIX)?;
        if rest == "claim" {
            return Some(Self::Claim);
        }
        let command_id = rest.strip_suffix("/result")?;
        (!command_id.is_empty() && !command_id.contains('/')).then(|| Self::Result {
            command_id: command_id.to_string(),
        })
    }
}

/// The identity fields of a claim answer.
#[derive(Deserialize)]
struct ClaimedCommandIdentity {
    command_id: String,
    fencing_token: i64,
}

/// The fencing token of a result report.
#[derive(Deserialize)]
struct ReportedFencingToken {
    fencing_token: i64,
}

/// What the relay does with one upstream answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Decision {
    /// Pass the answer back unchanged.
    Forward,
    /// Withhold the answer; the record describes it.
    Withhold(DropRecord),
}

/// The relay's facts `status` repeats: its listen address, its upstream and its hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RelayIdentity {
    pub(super) listen: String,
    pub(super) upstream: String,
    pub(super) withhold: Duration,
}

#[derive(Debug, Default)]
struct PolicyState {
    armed: Option<DropTarget>,
    forwarded_count: u64,
    drop_count: u64,
    last_drop: Option<DropRecord>,
}

/// The arming and the record of what the relay forwarded and withheld.
#[derive(Debug)]
pub(super) struct DropPolicy {
    identity: RelayIdentity,
    state: Mutex<PolicyState>,
}

impl DropPolicy {
    /// A disarmed policy for the relay `identity` describes.
    pub(super) fn new(identity: RelayIdentity) -> Self {
        Self {
            identity,
            state: Mutex::new(PolicyState::default()),
        }
    }

    /// Withhold the next `200` answer `target` names, replacing any earlier arming.
    pub(super) fn arm(&self, target: DropTarget) -> RelayStatus {
        let mut state = self.lock();
        state.armed = Some(target);
        self.status_of(&state)
    }

    /// Pass every answer through.
    pub(super) fn disarm(&self) -> RelayStatus {
        let mut state = self.lock();
        state.armed = None;
        self.status_of(&state)
    }

    /// The arming, the counts and the last drop.
    pub(super) fn status(&self) -> RelayStatus {
        self.status_of(&self.lock())
    }

    /// Decide the fate of the upstream's `answer_status` answer to `exchange`, whose request body
    /// was `request_body` and whose answer body is `answer_body`.
    pub(super) fn decide(
        &self,
        exchange: Option<&ExecutorExchange>,
        answer_status: StatusCode,
        request_body: &[u8],
        answer_body: &[u8],
    ) -> Decision {
        let mut state = self.lock();
        let withheld = match (state.armed, exchange) {
            (Some(DropTarget::DropNextClaimResponse), Some(ExecutorExchange::Claim))
                if answer_status == StatusCode::OK =>
            {
                let claimed = serde_json::from_slice::<ClaimedCommandIdentity>(answer_body).ok();
                Some(DropRecord {
                    response: ExecutorResponse::Claim,
                    command_id: claimed
                        .as_ref()
                        .map(|claimed| FleetCommandId::new(claimed.command_id.clone())),
                    fencing_token: claimed.map(|claimed| claimed.fencing_token),
                    upstream_status: answer_status.as_u16(),
                    withheld_at_unix_ms: SystemClock.now_unix_ms(),
                })
            }
            (
                Some(DropTarget::DropNextResultResponse),
                Some(ExecutorExchange::Result { command_id }),
            ) if answer_status == StatusCode::OK => Some(DropRecord {
                response: ExecutorResponse::Result,
                command_id: Some(FleetCommandId::new(command_id.clone())),
                fencing_token: serde_json::from_slice::<ReportedFencingToken>(request_body)
                    .ok()
                    .map(|reported| reported.fencing_token),
                upstream_status: answer_status.as_u16(),
                withheld_at_unix_ms: SystemClock.now_unix_ms(),
            }),
            _ => None,
        };
        match withheld {
            Some(record) => {
                state.armed = None;
                state.drop_count += 1;
                state.last_drop = Some(record.clone());
                Decision::Withhold(record)
            }
            None => {
                state.forwarded_count += 1;
                Decision::Forward
            }
        }
    }

    fn status_of(&self, state: &PolicyState) -> RelayStatus {
        RelayStatus {
            arming: Arming::from(state.armed),
            listen: self.identity.listen.clone(),
            upstream: self.identity.upstream.clone(),
            withhold_milliseconds: u64::try_from(self.identity.withhold.as_millis())
                .unwrap_or(u64::MAX),
            forwarded_count: state.forwarded_count,
            drop_count: state.drop_count,
            last_drop: state.last_drop.clone(),
        }
    }

    /// The state; a panic elsewhere while holding it leaves plain counters, still consistent.
    fn lock(&self) -> MutexGuard<'_, PolicyState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
#[path = "tests/drop_policy_tests.rs"]
mod tests;
