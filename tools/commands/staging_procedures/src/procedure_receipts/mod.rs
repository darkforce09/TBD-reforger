//! The receipt of one recorded staging procedure run: its check, its log, its acceptance
//! thresholds and the recording that writes them.
//!
//! **Role:** declares [`staging_check`] (the three recorded checks and their limits),
//! [`receipt_log`] (the values a run logs and the log grammar), [`acceptance_thresholds`] (the
//! measurements a passing run must meet) and [`recording_session`] (the recording that judges a
//! run and writes its receipt), and re-exports the values the procedures build.
//!
//! **Position:** `procedure_runner/recording.rs` opens and finishes a recording around every
//! `fleet --record`, `discord --record` and `load --record`; the procedures build the case,
//! environment, observation and measurement values; the receipts land in
//! `target/staging/receipts/`.
//!
//! **Signals & state:** none here; a recording owns its start snapshot until it finishes.
//!
//! **Invariants:** only a run whose every declared case passed and whose measurements meet the
//! thresholds carries the success marker; a run that stopped early still ends in a failing
//! receipt.

pub(crate) mod acceptance_thresholds;
pub(crate) mod receipt_log;
pub(crate) mod recording_session;
pub(crate) mod staging_check;

pub(crate) use acceptance_thresholds::Observations;
pub(crate) use receipt_log::{
    CaseName, CaseStatus, EnvironmentEntry, ObservationRecord, RecordedCase,
};
pub(crate) use recording_session::{FixtureManifest, RecordedOutcome, RecordingSession};
pub(crate) use staging_check::StagingCheck;
