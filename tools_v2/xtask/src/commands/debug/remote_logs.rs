//! T-855 — port of `scripts/mod/remote-log-grep.sh` → `cargo xtask mod remote-logs`.
//!
//! Four outcomes (preserved exactly): 0 HEALTHY · 1 FAIL · 2 PARTIAL · 3 ENVIRONMENT.
//! Usage / bad flags go to 3 (not 2) so a mistype cannot read as "booted, nobody joined".
//!
//! Fail-opens closed vs bash:
//! - `grep -c PAT 2>/dev/null || true` on the tagged-line count collapsed a read/pattern
//!   error into `0` (STALE BUILD). We count after a successful read; an unreadable log is
//!   ENVIRONMENT (3), same as a missing file.
//! - Display / error extract used `2>/dev/null`; we print matching lines from the same
//!   in-memory text used for the verdict (no silent empty extract on a read error).
//!
//! Preserved oddity: unset `TBD_SSH_HOST` after optional `deploy.env` source exits **1**
//! (bash `${VAR:?…}`), not ENVIRONMENT 3 — pin that, do not "fix" it to 3.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use verification_core::NotRun;
use verification_core::gate::probe_str;
use verification_core::pattern::Pattern;
use verification_core::proc::{self, Run};

use crate::core::repository_root::find_repo_root;

const PAT_TAGGED: &str = r"\[TBD\]\[";
const PAT_MISSION: &str = r"\[TBD\]\[Mission\] loaded id=";
const PAT_SLOTS: &str = r"\[TBD\]\[Slots\] Slot-";
const PAT_LOBBY: &str = r"\[TBD\]\[Stage\].*LOBBY|\[TBD\] Stage .*LOBBY";
const PAT_LOADOUT: &str = r"\[TBD\]\[Loadout\]\[Slot\]";
const PAT_ASSIGNED: &str = r"\[TBD\]\[Spawn\].*assigned|\[TBD\] SpawnManager: assigned";
const PAT_ERRORS: &str = r"Can.t compile|Unknown class .TBD_|RequestSpawn failed";
const PAT_EXTRACT: &str = r"\[TBD\]|assigned slot|Can.t compile|RequestSpawn failed|Unknown class";

struct SshOut {
    code: i32,
    stdout: String,
}

#[cfg(test)]
#[path = "tests/remote_logs/tests.rs"]
mod tests;

mod execution;
pub use execution::run;

mod shell_quote;
use shell_quote::append_line;
use shell_quote::parse_deploy_env;
use shell_quote::shell_quote;
use shell_quote::tempfile_dir;
use shell_quote::write_log;

#[cfg(test)]
use execution::{check_log, check_log_quiet, cmd_selftest};
