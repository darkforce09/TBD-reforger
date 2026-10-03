//! A recorded run's observation journal: one JSONL line per observation and every raw artifact,
//! stored by its SHA-256.
//!
//! **Role:** archives each observation's raw bytes under `artifacts/<sha256>.txt` and appends a
//! line naming its step, observer, time, verdict, summary and digest to `journal.jsonl`.
//!
//! **Position:** created by `procedure_runner/recording.rs` in the run folder
//! `target/staging/<check>/<run>/`; written by `procedure_runner/runner.rs`; the digests it
//! returns are the `sha256=` of the log's `observation:` lines.
//!
//! **Signals & state:** the open journal file and the run's observation sequence number.
//!
//! **Invariants:** the journal file is created new, so two runs never share one; a line is
//! flushed before `archive` returns; an artifact file holds exactly the bytes its name digests,
//! and identical observations share one file.

use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use crate::error::{Result, ResultExt};
use content_digest::sha256_hex;
use serde_json::json;

/// The journal's file name in the run folder.
pub(crate) const JOURNAL_FILE: &str = "journal.jsonl";
/// The folder of raw artifacts in the run folder.
pub(crate) const ARTIFACT_FOLDER: &str = "artifacts";

/// One observation to archive.
#[derive(Debug, Clone, Copy)]
pub(crate) struct JournalEntry<'a> {
    /// `<step>.<effect>`, `<step>.request` or `<step>.action`.
    pub step: &'a str,
    /// The observer label, e.g. `database` or `chrome page read`.
    pub observer: &'a str,
    pub observed_unix_ms: u64,
    /// What the judge made of it.
    pub summary: &'a str,
    /// `pending`, `satisfied`, `contradicted`, `host action`, …
    pub verdict: &'a str,
    /// The raw bytes observed.
    pub artifact: &'a [u8],
}

/// The journal of one run.
#[derive(Debug)]
pub(crate) struct ObservationJournal {
    directory: PathBuf,
    journal: File,
    sequence: u64,
}

impl ObservationJournal {
    /// Creates the journal in `directory`, which must not hold one yet.
    pub(crate) fn create(directory: &Path) -> Result<Self> {
        fs::create_dir_all(directory.join(ARTIFACT_FOLDER))
            .with_context(|| format!("create {}", directory.display()))?;
        let path = directory.join(JOURNAL_FILE);
        let journal = OpenOptions::new()
            .create_new(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("create {}", path.display()))?;
        Ok(Self {
            directory: directory.to_path_buf(),
            journal,
            sequence: 0,
        })
    }

    /// Stores `entry`'s artifact, appends its journal line, and returns the artifact's SHA-256.
    pub(crate) fn archive(&mut self, entry: &JournalEntry<'_>) -> Result<String> {
        let sha256 = sha256_hex(entry.artifact);
        let relative = format!("{ARTIFACT_FOLDER}/{sha256}.txt");
        let path = self.directory.join(&relative);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => file
                .write_all(entry.artifact)
                .with_context(|| format!("write {}", path.display()))?,
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(error).with_context(|| format!("create {}", path.display()));
            }
        }
        self.sequence += 1;
        let line = json!({
            "sequence": self.sequence,
            "step": entry.step,
            "observer": entry.observer,
            "observed_unix_ms": entry.observed_unix_ms,
            "verdict": entry.verdict,
            "summary": entry.summary,
            "sha256": sha256,
            "artifact": relative,
            "bytes": entry.artifact.len(),
        });
        writeln!(self.journal, "{line}")?;
        self.journal.flush()?;
        Ok(sha256)
    }
}
