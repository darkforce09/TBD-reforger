//! One step of a staging procedure: what happens, what the harness waits for, until when, and
//! which declared case each awaited effect decides.
//!
//! **Role:** the step vocabulary the procedures fill and the runner executes: [`Step`] with its
//! [`StepKind`] and `AWAIT` instruction, the optional [`RequestPredicate`] (the row that marks
//! the orchestrator's request), the [`EffectPredicate`]s with their [`Deadline`]s, and the case
//! each effect maps to.
//!
//! **Position:** built by `fleet_procedure/`, `discord_procedure/` and `load_procedure/`;
//! executed by `runner.rs`; serialized into the fixture manifest by `procedure.rs`.
//!
//! **Signals & state:** none; values and boxed predicate functions.
//!
//! **Invariants:** step and effect ids match `[a-z0-9_]+`; a deadline counts from the observed
//! request row or from the step's start, never from when an approval happened; a probe only
//! reads, and a judge is a pure function of the observed text and the step context.

use std::collections::BTreeMap;

use anyhow::{Result, ensure};
use serde_json::Value;

use crate::commands::staging::remote_observers::remote_command::RemoteCommand;
use crate::verifications::api_readiness::operational_recording::CaseName;

/// Values a probe measured, by name, visible to every later step (a session generation, a PID,
/// a player count).
pub(crate) type Measurements = BTreeMap<String, Value>;

/// A step or effect id: `[a-z0-9_]+`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct StepId(String);

impl StepId {
    /// Accepts `id` only when it matches `[a-z0-9_]+`.
    pub(crate) fn new(id: &str) -> Result<Self> {
        ensure!(identifier(id), "step id {id:?} must match [a-z0-9_]+");
        Ok(Self(id.to_string()))
    }

    /// The validated id.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// Who acts in a step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StepKind {
    /// The orchestrator acts in the operator's browser; the harness prints `AWAIT` and polls.
    ChromeAction,
    /// The harness runs this host-changing command at the step's start, then polls.
    HostAction(RemoteCommand),
    /// Nobody acts; the harness only observes.
    Observation,
}

impl StepKind {
    /// `chrome_action`, `host_action` or `observation`.
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Self::ChromeAction => "chrome_action",
            Self::HostAction(_) => "host_action",
            Self::Observation => "observation",
        }
    }
}

/// Where an effect's deadline counts from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeadlineAnchor {
    /// The time the request row carries (for example `fleet_commands.requested_at`).
    RequestRow,
    /// The moment the harness printed the step's `AWAIT` line.
    StepStart,
}

/// How long an effect may take, from its anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Deadline {
    pub anchor: DeadlineAnchor,
    pub seconds: u64,
}

impl Deadline {
    /// `seconds` after the observed request row.
    pub(crate) fn from_request_row(seconds: u64) -> Self {
        Self {
            anchor: DeadlineAnchor::RequestRow,
            seconds,
        }
    }

    /// `seconds` after the step started.
    pub(crate) fn from_step_start(seconds: u64) -> Self {
        Self {
            anchor: DeadlineAnchor::StepStart,
            seconds,
        }
    }
}

/// What a probe's judge sees besides the observed text.
#[derive(Debug, Clone, Copy)]
pub(crate) struct StepContext<'a> {
    /// When the step printed its `AWAIT` line.
    pub step_started_unix_ms: u64,
    /// The request row's time, once observed.
    pub request_unix_ms: Option<u64>,
    /// When this observation was taken.
    pub observed_unix_ms: u64,
    /// Everything measured so far in the run.
    pub measurements: &'a Measurements,
}

/// Builds the host read a probe polls.
pub(crate) type HostRead = Box<dyn Fn(&StepContext<'_>) -> Result<RemoteCommand>>;
/// Judges one observed text.
pub(crate) type Judge = Box<dyn Fn(&str, &StepContext<'_>) -> ProbeVerdict>;

/// Where a probe's observation comes from.
pub(crate) enum ProbeSource {
    /// A read of the staging host.
    Host(HostRead),
    /// The step's browser inbox entry, accepted only inside the step's window.
    BrowserInbox,
    /// A value the run measured on the workstation (the load engine's report, a census), read
    /// from the run's measurements under this name once it is there.
    Measurement(String),
}

/// One observation and its judgement.
pub(crate) struct Probe {
    pub source: ProbeSource,
    pub judge: Judge,
}

impl Probe {
    /// A probe that polls a host read.
    pub(crate) fn host(
        read: impl Fn(&StepContext<'_>) -> Result<RemoteCommand> + 'static,
        judge: impl Fn(&str, &StepContext<'_>) -> ProbeVerdict + 'static,
    ) -> Self {
        Self {
            source: ProbeSource::Host(Box::new(read)),
            judge: Box::new(judge),
        }
    }

    /// A probe that reads the step's browser inbox entry.
    pub(crate) fn browser_inbox(
        judge: impl Fn(&str, &StepContext<'_>) -> ProbeVerdict + 'static,
    ) -> Self {
        Self {
            source: ProbeSource::BrowserInbox,
            judge: Box::new(judge),
        }
    }

    /// A probe that judges the run's measurement `key`, as JSON text, once it is measured.
    pub(crate) fn measurement(
        key: &str,
        judge: impl Fn(&str, &StepContext<'_>) -> ProbeVerdict + 'static,
    ) -> Self {
        Self {
            source: ProbeSource::Measurement(key.to_string()),
            judge: Box::new(judge),
        }
    }

    /// Whether the probe reads the browser inbox.
    pub(crate) fn reads_browser(&self) -> bool {
        matches!(self.source, ProbeSource::BrowserInbox)
    }

    /// `host`, `browser_inbox` or `measurement:<name>`, as the fixture manifest records it.
    pub(crate) fn source_name(&self) -> String {
        match &self.source {
            ProbeSource::Host(_) => "host".to_string(),
            ProbeSource::BrowserInbox => "browser_inbox".to_string(),
            ProbeSource::Measurement(key) => format!("measurement:{key}"),
        }
    }
}

/// A judged effect that holds.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Satisfaction {
    /// What was observed, for the log.
    pub summary: String,
    /// The time the observed row carries (a request row's `requested_at`), when it has one.
    pub observed_unix_ms: Option<u64>,
    /// Values later steps and the observations may use.
    pub measurements: Vec<(String, Value)>,
}

impl Satisfaction {
    /// This satisfaction, carrying the row time `unix_ms`.
    pub(crate) fn at(mut self, unix_ms: u64) -> Self {
        self.observed_unix_ms = Some(unix_ms);
        self
    }

    /// This satisfaction, also measuring `key`.
    pub(crate) fn measure(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.measurements.push((key.into(), value.into()));
        self
    }
}

/// A judge's answer.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ProbeVerdict {
    /// Not yet; poll again. The text says what was seen.
    Pending(String),
    /// The effect holds.
    Satisfied(Satisfaction),
    /// The effect was observed wrong; the case fails at once.
    Contradicted(String),
}

impl ProbeVerdict {
    /// A satisfaction with `summary` and nothing else.
    pub(crate) fn satisfied(summary: impl Into<String>) -> Satisfaction {
        Satisfaction {
            summary: summary.into(),
            observed_unix_ms: None,
            measurements: Vec::new(),
        }
    }
}

/// The row whose appearance marks the orchestrator's request; it may take up to
/// [`super::runner::REQUEST_APPEARANCE_SECONDS`] to appear.
pub(crate) struct RequestPredicate {
    pub description: String,
    pub probe: Probe,
}

/// An awaited effect: what must be observed, by when, and which case it decides.
pub(crate) struct EffectPredicate {
    /// `[a-z0-9_]+`, unique within the step.
    pub id: String,
    pub description: String,
    pub probe: Probe,
    pub deadline: Deadline,
    /// The declared case this effect decides; a case holds only when every effect mapped to it
    /// was satisfied in time.
    pub case: CaseName,
}

/// One step of a procedure.
pub(crate) struct Step {
    pub id: StepId,
    pub kind: StepKind,
    /// The `AWAIT` instruction: what the orchestrator or operator does now.
    pub instruction: String,
    pub request: Option<RequestPredicate>,
    pub effects: Vec<EffectPredicate>,
}

/// A case the procedure declares, in declaration order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeclaredCase {
    pub name: CaseName,
    /// A dependency the staging environment lacks; the case is recorded `NOT RUN` naming it.
    pub unavailable_dependency: Option<String>,
}

impl DeclaredCase {
    /// A case the procedure runs.
    pub(crate) fn runs(name: &str) -> Result<Self> {
        Ok(Self {
            name: CaseName::new(name)?,
            unavailable_dependency: None,
        })
    }

    /// A case recorded `NOT RUN (missing: <dependency>)`.
    pub(crate) fn not_run(name: &str, dependency: &str) -> Result<Self> {
        Ok(Self {
            name: CaseName::new(name)?,
            unavailable_dependency: Some(dependency.to_string()),
        })
    }
}

pub(super) fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}
