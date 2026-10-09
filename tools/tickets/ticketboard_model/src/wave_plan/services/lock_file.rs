//! Read-only wave-lock parsing and ownership-path collision rules.
//!
//! **Role:** reads `.ai/tickets/wave.lock` into [`WaveLock`] ([`load_lock`]) as a [`LockState`],
//! and explains why two tickets can never share a wave ([`colliding_pairs`]).
//! **Position:** the Waves tab's input; `crate::application_state::background_loading` loads it,
//! [`crate::wave_plan::models::wave_projection`] projects it, and the comparison view of
//! `tools/tickets/ticketboard_desktop` lists the colliding pairs.
//! **Signals & state:** none; pure functions over the file text.
//! **Invariants:** wave membership is rendered exactly as recorded. Missing and malformed locks
//! are local refusals; the registry board remains usable. The board keeps its own lock shape,
//! because `ticket_wave_lock::WaveLock` differs in two ways: it refuses unknown keys
//! (`deny_unknown_fields`) where the board accepts them, since the CLI owns the format and its
//! strict validation, and it carries the pending-close `emptied` waves, which the board does not
//! render. The path, the missing-lock text and the collision rule are `ticket_wave_lock`'s own.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::{Error, Result};

/// Mirror of xtask `LockWave` — no `deny_unknown_fields` (see module docs).
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct LockWave {
    /// The wave number.
    pub n: u32,
    /// The wave's ticket ids, in packing order.
    pub tickets: Vec<String>,
}

/// Locks without wave_base deserialize it as zero, matching the writer's default.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct WaveLock {
    /// The format version.
    pub version: u32,
    /// The most tickets one open wave may hold.
    pub max_concurrent: usize,
    #[serde(default)]
    /// The newest closed wave number when the lock was packed.
    pub wave_base: u32,
    /// The tickets that pack last.
    pub pack_last: Vec<String>,
    /// Wave 0 and the open waves.
    pub waves: Vec<LockWave>,
    /// Every ticket's `owns` list, by id.
    pub owns: BTreeMap<String, Vec<String>>,
    /// Every ticket's `depends_on` list, by id.
    pub depends_on: BTreeMap<String, Vec<String>>,
}

/// Lock load outcome. `Missing` and `Refused` are DISTINCT render states — both are
/// refusals (never empty lanes), but only `Missing` carries the DidNotRun text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockState {
    /// The parsed lock.
    Loaded(WaveLock),
    /// wave.lock absent — the exact DidNotRun refusal message.
    Missing {
        /// The exact `DidNotRun` refusal text.
        message: String,
    },
    /// Present but unreadable or unparsable — the VERBATIM error, never paraphrased.
    Refused {
        /// The lock file.
        path: PathBuf,
        /// The verbatim read or parse error.
        error: String,
    },
}

/// Parses lock text; unknown keys are accepted, a missing required key refuses.
pub fn parse_lock(text: &str) -> Result<WaveLock> {
    toml::from_str(text).map_err(|e| Error::WaveLockUnparsable(e.to_string()))
}

/// Load `repo_root/.ai/tickets/wave.lock`. Never panics, never invents an empty plan.
pub fn load_lock(repo_root: &Path) -> LockState {
    let path = ticket_wave_lock::lock_path(repo_root);
    if !path.is_file() {
        return LockState::Missing {
            message: ticket_wave_lock::missing_lock_error(repo_root),
        };
    }
    match fs::read_to_string(&path) {
        Err(e) => LockState::Refused {
            path,
            error: e.to_string(),
        },
        Ok(text) => match parse_lock(&text) {
            Ok(lock) => LockState::Loaded(lock),
            Err(error) => LockState::Refused {
                path,
                error: error.to_string(),
            },
        },
    }
}

/// One path pair under the prefix-containment rule: equal, or one prefix-contains the
/// other with a '/' boundary. Mirrors the inner comparison of `ticket_wave_lock::collides` —
/// `"a/bc"` does NOT collide with `"a/b"` (no boundary).
pub fn paths_collide(x: &str, y: &str) -> bool {
    x == y
        || x.starts_with(&format!("{}/", y.trim_end_matches('/')))
        || y.starts_with(&format!("{}/", x.trim_end_matches('/')))
}

/// EVERY colliding `(a-side, b-side)` pair — the explainer surface ("why these two
/// can never share a wave"), not just the boolean.
pub fn colliding_pairs(a: &[String], b: &[String]) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for x in a {
        for y in b {
            if paths_collide(x, y) {
                pairs.push((x.clone(), y.clone()));
            }
        }
    }
    pairs
}
