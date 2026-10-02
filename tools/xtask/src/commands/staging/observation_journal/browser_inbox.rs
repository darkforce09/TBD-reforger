//! The browser inbox: page reads the orchestrator saves from the operator's Chrome for the
//! harness to judge.
//!
//! **Role:** reads `browser_inbox/<step>.json` (`{"captured_at_unix_ms": …, "output": …}`, the
//! Chrome tool's raw output unchanged), accepts it only when it was captured inside the step's
//! window, and refuses an entry that carries a credential.
//!
//! **Position:** the folder sits in the run folder `target/staging/<check>/<run>/`; the runner
//! polls it for probes whose source is the browser inbox and archives an accepted entry in the
//! journal as a `chrome page read`.
//!
//! **Signals & state:** none held; each read opens the file afresh, so a later capture replaces
//! an earlier one.
//!
//! **Invariants:** an entry captured before the step started or after its deadline is never
//! accepted; an entry whose text names an authorization header, a bearer, an access or refresh
//! token, or a set-cookie header is refused and never archived, so no token reaches the evidence.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

/// The observer label of an accepted entry.
pub(crate) const BROWSER_OBSERVER: &str = "chrome page read";
/// The inbox's folder name in the run folder.
pub(crate) const INBOX_FOLDER: &str = "browser_inbox";

/// Lowercase fragments that mark an entry as carrying a credential.
const CREDENTIAL_MARKERS: [&str; 6] = [
    "authorization",
    "bearer ",
    "access_token",
    "refresh_token",
    "set-cookie",
    "\"token\"",
];

/// The file the orchestrator writes.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InboxEntry {
    captured_at_unix_ms: u64,
    output: serde_json::Value,
}

/// When an entry counts for a step: from the step's start to its deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InboxWindow {
    pub opens_unix_ms: u64,
    pub closes_unix_ms: u64,
}

/// What one read of a step's entry found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InboxRead {
    /// No entry yet.
    Absent,
    /// An entry captured outside the window.
    Outside { captured_at_unix_ms: u64 },
    /// An entry that must not be used: malformed, or carrying a credential.
    Refused(String),
    /// An entry inside the window: the tool output (a string output as is, anything else as
    /// JSON text) and the file's raw text for the journal.
    Accepted {
        captured_at_unix_ms: u64,
        text: String,
        raw: String,
    },
}

/// The inbox of one run.
#[derive(Debug, Clone)]
pub(crate) struct BrowserInbox {
    directory: PathBuf,
}

impl BrowserInbox {
    /// The inbox in `directory`.
    pub(crate) fn new(directory: &Path) -> Self {
        Self {
            directory: directory.to_path_buf(),
        }
    }

    /// Where the orchestrator saves `step`'s page read.
    pub(crate) fn entry_path(&self, step: &str) -> PathBuf {
        self.directory.join(format!("{step}.json"))
    }

    /// Reads `step`'s entry and judges it against `window`.
    pub(crate) fn read(&self, step: &str, window: InboxWindow) -> Result<InboxRead> {
        let path = self.entry_path(step);
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(InboxRead::Absent),
            Err(error) => return Err(error).with_context(|| format!("read {}", path.display())),
        };
        let Ok(raw) = String::from_utf8(bytes) else {
            return Ok(InboxRead::Refused(format!(
                "{} is not UTF-8",
                path.display()
            )));
        };
        let lowercase = raw.to_ascii_lowercase();
        if let Some(marker) = CREDENTIAL_MARKERS
            .iter()
            .find(|marker| lowercase.contains(**marker))
        {
            return Ok(InboxRead::Refused(format!(
                "the entry names {marker:?}; the harness never reads a credential, so it is not \
                 archived"
            )));
        }
        let entry: InboxEntry = match serde_json::from_str(&raw) {
            Ok(entry) => entry,
            Err(error) => {
                return Ok(InboxRead::Refused(format!(
                    "the entry is not {{\"captured_at_unix_ms\", \"output\"}} JSON: {error}"
                )));
            }
        };
        let captured_at_unix_ms = entry.captured_at_unix_ms;
        if captured_at_unix_ms < window.opens_unix_ms || captured_at_unix_ms > window.closes_unix_ms
        {
            return Ok(InboxRead::Outside {
                captured_at_unix_ms,
            });
        }
        let text = match entry.output {
            serde_json::Value::String(text) => text,
            other => other.to_string(),
        };
        Ok(InboxRead::Accepted {
            captured_at_unix_ms,
            text,
            raw,
        })
    }
}
