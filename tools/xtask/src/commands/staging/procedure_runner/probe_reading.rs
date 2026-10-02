//! One observation of a probe: the read (a host command or the step's browser inbox entry), the
//! judgement, and the journal entry that archives what was seen.
//!
//! **Role:** [`observe`] takes one observation for the runner and returns the verdict, the
//! observer's label and the archived artifact's digest.
//!
//! **Position:** called by `runner.rs` for every poll of a request or effect probe, and by the
//! load procedure's run, which polls its steps between its own workstation actions.
//!
//! **Signals & state:** none held; reads through the run's host and inbox, writes to its journal.
//!
//! **Invariants:** a probe whose read would change the host is contradicted without reaching the
//! host; a failed or non-zero read is pending, never satisfied; a browser entry is judged only
//! inside its window and a refused one is never archived; a measurement is judged only once the
//! run holds it; every observed artifact is journaled.

use anyhow::Result;

use super::runner::RunContext;
use super::step::{Measurements, Probe, ProbeSource, ProbeVerdict, Step, StepContext};
use crate::commands::staging::observation_journal::browser_inbox::{
    BROWSER_OBSERVER, InboxRead, InboxWindow,
};
use crate::commands::staging::observation_journal::journal::JournalEntry;
use crate::commands::staging::remote_observers::remote_command::CommandPurpose;

/// The observer label of a measurement taken on the workstation.
pub(crate) const MEASUREMENT_OBSERVER: &str = "workstation";

/// One observation, judged and journaled.
pub(crate) struct Observed {
    pub verdict: ProbeVerdict,
    pub observer: String,
    /// The archived artifact's SHA-256, when something was observed.
    pub sha256: Option<String>,
}

/// When a probe observes: its step's start, the request row's time once seen, and the end of
/// the window its browser entry must fall in.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ProbeWindow {
    pub started: u64,
    pub request_unix_ms: Option<u64>,
    pub window_end: u64,
}

/// Takes one observation through `probe` inside `window`, judges it and journals it.
pub(crate) fn observe(
    context: &mut RunContext<'_>,
    measurements: &Measurements,
    step: &Step,
    probe_id: &str,
    probe: &Probe,
    window: ProbeWindow,
) -> Result<Observed> {
    let ProbeWindow {
        started,
        request_unix_ms,
        window_end,
    } = window;
    let now = context.clock.now_unix_ms();
    let step_context = StepContext {
        step_started_unix_ms: started,
        request_unix_ms,
        observed_unix_ms: now,
        measurements,
    };
    // (observer, the text the judge reads, the raw bytes the journal keeps, a verdict that
    // replaces the judge's)
    let (observer, judged, artifact, verdict) = match &probe.source {
        ProbeSource::Host(read) => match read(&step_context) {
            Err(error) => {
                let why = format!("its read cannot be built: {error:#}");
                (
                    "probe".to_string(),
                    None,
                    None,
                    Some(ProbeVerdict::Contradicted(why)),
                )
            }
            Ok(command) if command.purpose != CommandPurpose::Read => {
                let why = "its read would change the host".to_string();
                (
                    "probe".to_string(),
                    None,
                    None,
                    Some(ProbeVerdict::Contradicted(why)),
                )
            }
            Ok(command) => match context.host.run(&command) {
                Ok(output) if output.exit_code == 0 => {
                    let observer = command.observer.to_string();
                    (
                        observer,
                        Some(output.stdout.clone()),
                        Some(output.stdout),
                        None,
                    )
                }
                Ok(output) => {
                    let seen = format!("{} exited {}", command.observer, output.exit_code);
                    let observer = command.observer.to_string();
                    (
                        observer,
                        None,
                        Some(output.stdout),
                        Some(ProbeVerdict::Pending(seen)),
                    )
                }
                Err(error) => {
                    let seen = format!("{error:#}");
                    let observer = command.observer.to_string();
                    (observer, None, None, Some(ProbeVerdict::Pending(seen)))
                }
            },
        },
        ProbeSource::BrowserInbox => {
            let window = InboxWindow {
                opens_unix_ms: started,
                closes_unix_ms: window_end,
            };
            let observer = BROWSER_OBSERVER.to_string();
            match context.inbox.read(step.id.as_str(), window)? {
                InboxRead::Absent => {
                    let seen = "no browser inbox entry yet".to_string();
                    (observer, None, None, Some(ProbeVerdict::Pending(seen)))
                }
                InboxRead::Outside {
                    captured_at_unix_ms,
                } => {
                    let seen = format!(
                        "the inbox entry was captured at {captured_at_unix_ms}, outside the \
                         step's window"
                    );
                    (observer, None, None, Some(ProbeVerdict::Pending(seen)))
                }
                InboxRead::Refused(why) => {
                    (observer, None, None, Some(ProbeVerdict::Contradicted(why)))
                }
                InboxRead::Accepted { text, raw, .. } => (observer, Some(text), Some(raw), None),
            }
        }
        ProbeSource::Measurement(key) => {
            let observer = MEASUREMENT_OBSERVER.to_string();
            match measurements.get(key) {
                Some(value) => {
                    let text = value.to_string();
                    (observer, Some(text.clone()), Some(text), None)
                }
                None => {
                    let seen = format!("{key} is not measured yet");
                    (observer, None, None, Some(ProbeVerdict::Pending(seen)))
                }
            }
        }
    };
    let verdict = match (verdict, &judged) {
        (Some(verdict), _) => verdict,
        (None, Some(text)) => (probe.judge)(text, &step_context),
        (None, None) => ProbeVerdict::Pending("nothing observed".to_string()),
    };
    let sha256 = match &artifact {
        Some(artifact) => {
            let (kind, summary) = match &verdict {
                ProbeVerdict::Pending(seen) => ("pending", seen.clone()),
                ProbeVerdict::Satisfied(satisfaction) => {
                    ("satisfied", satisfaction.summary.clone())
                }
                ProbeVerdict::Contradicted(why) => ("contradicted", why.clone()),
            };
            let label = format!("{}.{probe_id}", step.id.as_str());
            Some(context.journal.archive(&JournalEntry {
                step: &label,
                observer: &observer,
                observed_unix_ms: now,
                summary: &summary,
                verdict: kind,
                artifact: artifact.as_bytes(),
            })?)
        }
        None => None,
    };
    Ok(Observed {
        verdict,
        observer,
        sha256,
    })
}
