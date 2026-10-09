//! The playtest's check that the game runtime's telemetry reaches the platform.
//!
//! **Role:** after the deployment is confirmed, wait for the runtime's first telemetry queue
//! reading and note the match the server runs; on a requested stop, while the server is still up
//! and heartbeating, wait for the queue's backlog to drain; once the server has stopped, require
//! that every match seen holds acknowledged events and that the last reading's backlog is zero.
//! With `--require-telemetry` a failed check turns a successful run's exit code into 1; without it
//! the observations are only printed and the stop does not wait for the drain.
//!
//! **Position:** driven by [`super::platform_deployment::confirm`],
//! [`super::platform_deployment::drain_telemetry`] (the boot's pre-stop hook) and
//! [`super::platform_deployment::release`] with the development administrator's bearer the
//! playtest already holds; reads through
//! [`crate::website_api_client::server_telemetry_status`] and
//! [`crate::website_api_client::match_has_acknowledged_events`].
//!
//! **Signals & state:** [`TelemetryWatch`] keeps the observations across the two phases in a
//! `RefCell` (the boot's ready hook is a shared `Fn`); everything runs on the playtest's one
//! thread.
//!
//! **Invariants:** the verdict and the exit-code fold are pure functions ([`release_verdict`],
//! [`fold_exit_code`]); a non-zero run code is never replaced; a run whose server never became
//! ready is not checked; a missing reading or an unreadable event page is a failure, never a pass;
//! the backlog is judged only when it was read with the server still up (a crash cannot drain).

use std::cell::{Cell, RefCell};
use std::thread::sleep;
use std::time::{Duration, Instant};

use crate::website_api_client::{
    ApiClient, ServerTelemetryStatus, TelemetryQueueReading, match_has_acknowledged_events,
    server_telemetry_status,
};

/// How long the playtest waits for the runtime's first queue reading after the confirmation.
const FIRST_READING_PATIENCE: Duration = Duration::from_secs(90);
/// How long a requested stop waits, with the server still up, for the backlog to drain.
const DRAIN_PATIENCE: Duration = Duration::from_secs(60);
/// The pause between two status reads.
const POLL_INTERVAL: Duration = Duration::from_secs(5);

/// What the status reads showed over the run.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct TelemetryObservations {
    /// Every non-empty `current_match_id` seen, in the order first seen.
    pub matches_seen: Vec<String>,
    /// The most recent queue reading.
    pub last_reading: Option<TelemetryQueueReading>,
    /// The status was read on a requested stop or an expired deadline, with the server still up.
    /// A server that exited on its own (a crash) could not drain, so its backlog is not judged.
    pub read_before_stop: bool,
}

impl TelemetryObservations {
    /// Fold one status read in: a new match id is remembered, a reading replaces the last one.
    pub(super) fn record(&mut self, status: &ServerTelemetryStatus) {
        if let Some(match_id) = &status.current_match_id
            && !self.matches_seen.contains(match_id)
        {
            self.matches_seen.push(match_id.clone());
        }
        if let Some(reading) = &status.telemetry_queue {
            self.last_reading = Some(reading.clone());
        }
    }
}

/// The outcome of the release check.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum TelemetryVerdict {
    /// A reading arrived and no match was seen, so there was nothing to require.
    NoMatchObserved,
    /// Every match seen holds events and the last reading's backlog is zero.
    Passed,
    /// Why the check failed, one line per reason.
    Failed(Vec<String>),
}

/// The release verdict over the observations and each seen match's event read (`Ok(true)` when
/// the match holds at least one acknowledged event).
pub(super) fn release_verdict(
    observations: &TelemetryObservations,
    event_reads: &[(String, Result<bool, String>)],
) -> TelemetryVerdict {
    let mut reasons = Vec::new();
    match &observations.last_reading {
        None => reasons.push("the runtime never reported a telemetry_queue reading".to_string()),
        Some(reading)
            if observations.read_before_stop
                && !observations.matches_seen.is_empty()
                && reading.backlog > 0 =>
        {
            reasons.push(format!(
                "the last reading ({}) still holds a backlog of {}",
                reading.reported_at, reading.backlog
            ))
        }
        Some(_) => {}
    }
    for match_id in &observations.matches_seen {
        match event_reads.iter().find(|(id, _)| id == match_id) {
            Some((_, Ok(true))) => {}
            Some((_, Ok(false))) => {
                reasons.push(format!("match {match_id} holds no acknowledged event"))
            }
            Some((_, Err(error))) => reasons.push(format!(
                "the events of match {match_id} are unreadable: {error}"
            )),
            None => reasons.push(format!("the events of match {match_id} were not read")),
        }
    }
    if !reasons.is_empty() {
        TelemetryVerdict::Failed(reasons)
    } else if observations.matches_seen.is_empty() {
        TelemetryVerdict::NoMatchObserved
    } else {
        TelemetryVerdict::Passed
    }
}

/// The run's exit code once the telemetry verdict is folded in: a failed check turns a
/// successful run into 1 only when the check is required.
pub(super) fn fold_exit_code(run_code: u8, verdict: &TelemetryVerdict, required: bool) -> u8 {
    match verdict {
        TelemetryVerdict::Failed(_) if required && run_code == 0 => 1,
        _ => run_code,
    }
}

/// Whether the drain wait continues: the last reading still holds a backlog.
pub(super) fn backlog_remains(observations: &TelemetryObservations) -> bool {
    observations
        .last_reading
        .as_ref()
        .is_some_and(|reading| reading.backlog > 0)
}

/// One line describing a queue reading.
pub(super) fn describe_reading(reading: &TelemetryQueueReading) -> String {
    format!(
        "backlog {}/{}, dropped {}, oldest {} s (reported {})",
        reading.backlog,
        reading.capacity,
        reading.dropped_total,
        reading.oldest_age_seconds,
        reading.reported_at
    )
}

/// The observations of one playtest run, kept between the ready hook and the release.
pub(super) struct TelemetryWatch {
    required: bool,
    armed: Cell<bool>,
    observations: RefCell<TelemetryObservations>,
}

impl TelemetryWatch {
    /// A watch; `required` is `--require-telemetry`.
    pub(super) fn new(required: bool) -> Self {
        Self {
            required,
            armed: Cell::new(false),
            observations: RefCell::default(),
        }
    }

    /// After the confirmation: poll up to 90 s for the first queue reading and print it.
    pub(super) fn observe_after_confirmation(&self, client: &ApiClient<'_>, server_id: &str) {
        self.armed.set(true);
        println!(
            "==> waiting up to {} s for the runtime's telemetry queue reading",
            FIRST_READING_PATIENCE.as_secs()
        );
        let started = Instant::now();
        loop {
            self.read_status(client, server_id);
            if let Some(reading) = &self.observations.borrow().last_reading {
                println!("    telemetry queue: {}", describe_reading(reading));
                break;
            }
            if started.elapsed() >= FIRST_READING_PATIENCE {
                println!(
                    "    no telemetry_queue reading within {} s",
                    FIRST_READING_PATIENCE.as_secs()
                );
                break;
            }
            sleep(POLL_INTERVAL);
        }
        self.print_matches();
    }

    /// On a requested stop, while the server still heartbeats: read the status and, under
    /// `--require-telemetry`, poll up to 60 s until the backlog is zero.
    pub(super) fn drain_before_stop(&self, client: &ApiClient<'_>, server_id: &str) {
        if !self.armed.get() {
            return;
        }
        self.observations.borrow_mut().read_before_stop = true;
        self.read_status(client, server_id);
        if self.required {
            let backlog_remains_now = || backlog_remains(&self.observations.borrow());
            if backlog_remains_now() {
                println!(
                    "==> waiting up to {} s for the telemetry queue to drain before the stop",
                    DRAIN_PATIENCE.as_secs()
                );
            }
            let started = Instant::now();
            while backlog_remains_now() && started.elapsed() < DRAIN_PATIENCE {
                sleep(POLL_INTERVAL);
                self.read_status(client, server_id);
            }
        }
        if let Some(reading) = &self.observations.borrow().last_reading {
            println!(
                "    telemetry queue before the stop: {}",
                describe_reading(reading)
            );
        }
    }

    /// When the server has stopped: the release check, folded into `run_code`.
    pub(super) fn check_on_release(
        &self,
        client: &ApiClient<'_>,
        server_id: &str,
        run_code: u8,
    ) -> u8 {
        if !self.armed.get() {
            return run_code;
        }
        println!("==> telemetry check");
        if !self.observations.borrow().read_before_stop {
            // The server stopped on its own; its last stored reading stands.
            self.read_status(client, server_id);
        }
        let observations = self.observations.borrow().clone();
        if let Some(reading) = &observations.last_reading {
            println!(
                "    last telemetry queue reading: {}",
                describe_reading(reading)
            );
        }
        self.print_matches();
        let event_reads: Vec<(String, Result<bool, String>)> = observations
            .matches_seen
            .iter()
            .map(|match_id| {
                let read = match_has_acknowledged_events(client, match_id)
                    .map_err(|error| format!("{error:#}"));
                (match_id.clone(), read)
            })
            .collect();
        let verdict = release_verdict(&observations, &event_reads);
        self.print_verdict(&verdict);
        fold_exit_code(run_code, &verdict, self.required)
    }

    /// One status read folded into the observations; a failed read is printed and skipped.
    fn read_status(&self, client: &ApiClient<'_>, server_id: &str) {
        match server_telemetry_status(client, server_id) {
            Ok(status) => self.observations.borrow_mut().record(&status),
            Err(error) => println!("    could not read the server status: {error:#}"),
        }
    }

    fn print_matches(&self) {
        let observations = self.observations.borrow();
        if observations.matches_seen.is_empty() {
            println!("    no current match seen yet");
        } else {
            println!("    matches seen: {}", observations.matches_seen.join(", "));
        }
    }

    fn print_verdict(&self, verdict: &TelemetryVerdict) {
        match verdict {
            TelemetryVerdict::NoMatchObserved => {
                println!("    no match was seen, so no events or drain were required")
            }
            TelemetryVerdict::Passed => println!(
                "    telemetry PASSED: every match seen holds acknowledged events, backlog 0"
            ),
            TelemetryVerdict::Failed(reasons) => {
                for reason in reasons {
                    println!("    telemetry FAILED: {reason}");
                }
                if self.required {
                    println!("    --require-telemetry: the run exits non-zero");
                } else {
                    println!("    (reported only; --require-telemetry makes this fail the run)");
                }
            }
        }
    }
}
