//! `wave.lock` mirror (T-915.2 §Data layer) — pure, no egui types.
//!
//! Parses `.ai/tickets/wave.lock` into a LOCAL mirror of xtask's
//! `wave_lock::WaveLock`. The app cannot link xtask (it is the heavy bin — clap,
//! jsonschema, map-engine-core — and a long-running GUI must not freeze a fast-moving
//! rule set into itself), so the struct is redeclared here with `deny_unknown_fields`
//! OFF: a newer lock with extra keys must still render (forward-compat read).
//!
//! The lock is rendered VERBATIM — this module never recomputes packing and never
//! re-derives membership. Drift between the lock and the ticket files is the trust
//! banner's job (T-915.3), not this viewer's.
//!
//! A missing lock is a REFUSAL, never an empty plan: `missing_lock_message` mirrors
//! `ticket_engine::wave_lock::missing_lock_error` byte-for-byte. `paths_collide` /
//! `collides` mirror `wave_lock::collides` (the prefix-containment rule) for the
//! owns-collision explainer, unit-tested against the same cases.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

pub fn lock_path(repo_root: &Path) -> PathBuf {
    repo_root.join(ticket_engine::repository::WAVE_LOCK)
}

/// Mirror of xtask `LockWave` — no `deny_unknown_fields` (see module docs).
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct LockWave {
    pub n: u32,
    pub tickets: Vec<String>,
}

/// Mirror of xtask `WaveLock`. `wave_base` keeps the upstream serde default so
/// pre-T-914 locks (no `wave_base` line) still read.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct WaveLock {
    pub version: u32,
    pub max_concurrent: usize,
    #[serde(default)]
    pub wave_base: u32,
    pub pack_last: Vec<String>,
    pub waves: Vec<LockWave>,
    pub owns: BTreeMap<String, Vec<String>>,
    pub depends_on: BTreeMap<String, Vec<String>>,
}

/// Lock load outcome. `Missing` and `Refused` are DISTINCT render states — both are
/// refusals (never empty lanes), but only `Missing` carries the DidNotRun text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockState {
    Loaded(WaveLock),
    /// wave.lock absent — the exact DidNotRun refusal message.
    Missing {
        message: String,
    },
    /// Present but unreadable or unparsable — the VERBATIM error, never paraphrased.
    Refused {
        path: PathBuf,
        error: String,
    },
}

/// Byte-for-byte the `ticket_engine::wave_lock::missing_lock_error` text.
pub fn missing_lock_message(path: &Path) -> String {
    format!(
        "{} missing — DidNotRun: run `cargo xtask wave repack`. A missing lock is a refusal, never an empty plan.",
        path.display()
    )
}

pub fn parse_lock(text: &str) -> Result<WaveLock, String> {
    toml::from_str(text).map_err(|e| e.to_string())
}

/// Load `repo_root/.ai/tickets/wave.lock`. Never panics, never invents an empty plan.
pub fn load_lock(repo_root: &Path) -> LockState {
    let path = lock_path(repo_root);
    if !path.is_file() {
        return LockState::Missing {
            message: missing_lock_message(&path),
        };
    }
    match fs::read_to_string(&path) {
        Err(e) => LockState::Refused {
            path,
            error: e.to_string(),
        },
        Ok(text) => match parse_lock(&text) {
            Ok(lock) => LockState::Loaded(lock),
            Err(error) => LockState::Refused { path, error },
        },
    }
}

/// One path pair under the prefix-containment rule: equal, or one prefix-contains the
/// other with a '/' boundary. Mirrors the inner comparison of `wave_lock::collides` —
/// `"a/bc"` does NOT collide with `"a/b"` (no boundary).
pub fn paths_collide(x: &str, y: &str) -> bool {
    x == y
        || x.starts_with(&format!("{}/", y.trim_end_matches('/')))
        || y.starts_with(&format!("{}/", x.trim_end_matches('/')))
}

/// Mirror of `wave_lock::collides`: two owns sets collide when ANY path pair does.
pub fn collides(a: &[String], b: &[String]) -> bool {
    a.iter().any(|x| b.iter().any(|y| paths_collide(x, y)))
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

#[cfg(test)]
#[path = "tests/wavelock_tests.rs"]
mod tests;
