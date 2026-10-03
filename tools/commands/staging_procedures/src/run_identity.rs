//! A recorded run's identity: its check, its run id, where its journal lives, and the command
//! line its receipt names.
//!
//! **Role:** derives the run folder `target/staging/<check>/<run>/` and its browser inbox from the
//! recorder's run id, and the recorded command from the process arguments.
//!
//! **Position:** used by `procedure_runner/recording.rs` after
//! `RecordingSession::begin` hands out the run id; the receipt goes to [`EVIDENCE_DIRECTORY`].
//!
//! **Signals & state:** none; pure values.
//!
//! **Invariants:** the journal folder and the receipt log share one run id; the recorded command
//! is `cargo xtask <arguments>`, the command the run discipline requires from the repository root.

use std::path::{Path, PathBuf};

use crate::observation_journal::browser_inbox::INBOX_FOLDER;
use api_readiness_checks::operational_recording::StagingCheck;

/// Where receipts go, relative to the repository root: the folder `verify api-readiness` judges.
pub(crate) const EVIDENCE_DIRECTORY: &str = "target/api-readiness";
/// Where run journals go, relative to the repository root.
pub(crate) const STAGING_RUNS_DIRECTORY: &str = "target/staging";

/// One recorded run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunIdentity {
    pub(crate) check: StagingCheck,
    pub(crate) run_id: String,
    /// `<root>/target/staging/<check>/<run>/`.
    pub(crate) directory: PathBuf,
}

impl RunIdentity {
    /// The run `run_id` of `check` in the checkout at `root`.
    pub(crate) fn new(root: &Path, check: StagingCheck, run_id: &str) -> Self {
        Self {
            check,
            run_id: run_id.to_string(),
            directory: root
                .join(STAGING_RUNS_DIRECTORY)
                .join(check.id())
                .join(run_id),
        }
    }

    /// The run's browser inbox folder.
    pub(crate) fn inbox_directory(&self) -> PathBuf {
        self.directory.join(INBOX_FOLDER)
    }
}

/// `cargo xtask` and every argument after the program: the command the receipt records.
pub(crate) fn recorded_command(arguments: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut command = vec!["cargo".to_string(), "xtask".to_string()];
    command.extend(arguments.into_iter().skip(1));
    command
}
