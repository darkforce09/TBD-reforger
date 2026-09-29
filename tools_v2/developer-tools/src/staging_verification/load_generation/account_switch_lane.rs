//! The account side of a virtual client: its sign-in, its prefetched account switches, and the
//! handover of each signed-in account to its member requests.
//!
//! - **Role:** signs the client into its first account at its sign-in instant, refreshes the next
//!   account half a hold before each switch, and hands the member-request lane the account to
//!   send as: at once after the sign-in or a renewal of the same account, and at the switch
//!   instant after a prefetch.
//! - **Position:** one of the two lanes [`super::virtual_client::VirtualClient::run`] drives side
//!   by side; publishes through a `watch` channel that
//!   [`super::member_request_lane::MemberRequestLane`] reads at every slot; builds its refreshes
//!   through [`super::account_rotation`] and sends them through [`super::guarded_exchange`].
//! - **Signals & state:** owns the client's [`AccountRing`], the sending half of the handover,
//!   its session records and its late-switch count.
//! - **Invariants:**
//!   - A refresh never holds up a member request: it runs beside the member-request lane, its
//!     latency joins the `session` class only, and its auth ceiling never stalls the address's
//!     queue.
//!   - The member-request lane never loses its account to a switch: a switch hands over only an
//!     account whose refresh succeeded, and until then, or when every candidate fails, the
//!     current account stays.
//!   - A switch whose prefetch has not finished at the switch instant is late: the next account
//!     takes over as soon as its refresh ends, and the switch is counted.
//!   - At most one refresh is in flight; a failed refresh retires its account and the next
//!     candidate is refreshed at once. No refresh starts once the run's window has closed.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;
use tokio::time::Instant;

use super::account_rotation::{self, AccountRing, SecretToken};
use super::guarded_exchange::{BodyUse, ClientConnection, RequestHeaders};
use super::latency_recording::{RequestOutcome, RequestRecord};
use super::pacing::AccountSwitchSchedule;
use super::virtual_client::RunContext;
use super::workload_plan::RequestClass;

/// The account the member-request lane sends as: its ring position and its access token.
#[derive(Debug, Clone)]
pub(crate) struct SignedInAccount {
    pub(crate) position: usize,
    pub(crate) access_token: SecretToken,
}

/// The receiving half of a client's handover: `None` while the client holds no usable account.
pub(crate) type SignedInReceiver = watch::Receiver<Option<SignedInAccount>>;

/// What the account-switch lane leaves behind when the run ends.
#[derive(Debug, Default)]
pub(crate) struct AccountSwitchOutcome {
    pub(crate) records: Vec<RequestRecord>,
    pub(crate) guard_delayed: u64,
    pub(crate) late_switches: u64,
}

/// The account side of one client for the length of a run.
pub(crate) struct AccountSwitchLane {
    connection: ClientConnection,
    schedule: AccountSwitchSchedule,
    ring: AccountRing,
    handover: watch::Sender<Option<SignedInAccount>>,
    outcome: AccountSwitchOutcome,
}

impl AccountSwitchLane {
    /// The lane and the receiving half its member-request lane reads.
    pub(crate) fn new(
        connection: ClientConnection,
        schedule: AccountSwitchSchedule,
        ring: AccountRing,
    ) -> (Self, SignedInReceiver) {
        let (handover, signed_in) = watch::channel(None);
        let lane = Self {
            connection,
            schedule,
            ring,
            handover,
            outcome: AccountSwitchOutcome::default(),
        };
        (lane, signed_in)
    }

    /// Sign in, then prefetch and switch at every switch instant inside the run's window.
    pub(crate) async fn run(mut self, context: Arc<RunContext>) -> AccountSwitchOutcome {
        let window_end = context.settings.ramp + context.settings.measured;
        let sign_in = self.schedule.sign_in();
        if sign_in >= window_end {
            return self.outcome;
        }
        tokio::time::sleep_until(context.start + sign_in).await;
        let Some(first) = self.refresh_next(&context, sign_in).await else {
            return self.outcome;
        };
        self.hand_over(first);
        for number in 1u32.. {
            let switch = self.schedule.switch(number);
            if switch >= window_end {
                break;
            }
            let prefetch = self.schedule.prefetch(number);
            tokio::time::sleep_until(context.start + prefetch).await;
            if Instant::now() >= context.end {
                break;
            }
            let refreshed = self.refresh_next(&context, prefetch).await;
            let switch_at = context.start + switch;
            if Instant::now() > switch_at {
                self.outcome.late_switches += 1;
            }
            match refreshed {
                Some(next) if Some(next.position) == self.ring.current() => self.hand_over(next),
                Some(next) => {
                    tokio::time::sleep_until(switch_at).await;
                    self.hand_over(next);
                }
                None if self.ring.current().is_none() => {
                    self.handover.send_replace(None);
                }
                None => {}
            }
        }
        self.outcome
    }

    /// Make `account` the one the member-request lane sends as.
    fn hand_over(&mut self, account: SignedInAccount) {
        self.ring.switch_to(account.position);
        self.handover.send_replace(Some(account));
    }

    /// Refresh the ring's candidates in turn until one hands over a new pair; `None` once every
    /// account is retired or the window has closed. The first attempt counts from `scheduled`,
    /// each retry from its own start.
    async fn refresh_next(
        &mut self,
        context: &RunContext,
        scheduled: Duration,
    ) -> Option<SignedInAccount> {
        let mut scheduled = scheduled;
        while let Some(position) = self.ring.next_candidate() {
            if Instant::now() >= context.end {
                return None;
            }
            let account = self.ring.account(position);
            let account_index = account.index;
            let request = account_rotation::refresh_request(account);
            let auth = request.auth;
            let headers = RequestHeaders {
                bearer: None,
                if_none_match: None,
            };
            let exchange = self
                .connection
                .exchange(context, scheduled, request, headers, BodyUse::Keep)
                .await;
            self.outcome.guard_delayed += u64::from(exchange.guard_delayed);
            let refreshed = match (exchange.outcome, exchange.body.as_deref()) {
                (RequestOutcome::Expected { status }, Some(body)) => {
                    account_rotation::decode_refresh_answer(body)
                        .ok_or(RequestOutcome::UndecodableSessionAnswer { status })
                }
                (outcome, _) => Err(outcome),
            };
            let (outcome, signed_in) = match refreshed {
                Ok(pair) => {
                    let access_token = self.ring.take_refreshed(position, pair);
                    let account = SignedInAccount {
                        position,
                        access_token,
                    };
                    (exchange.outcome, Some(account))
                }
                Err(outcome) => {
                    self.ring.retire(position);
                    (outcome, None)
                }
            };
            self.outcome.records.push(RequestRecord {
                client: self.connection.client,
                address: self.connection.address,
                account: account_index,
                class: RequestClass::Session,
                template: None,
                scheduled,
                sent: exchange.sent,
                finished: exchange.finished,
                outcome,
                auth,
            });
            if signed_in.is_some() {
                return signed_in;
            }
            scheduled = Instant::now().saturating_duration_since(context.start);
        }
        None
    }
}
