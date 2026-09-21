//! `cargo xtask mod remote-logs` — read a staging server's console log and say what happened.
//!
//! Four outcomes: 0 HEALTHY · 1 FAIL · 2 PARTIAL · 3 ENVIRONMENT. A usage error or a bad flag is
//! ENVIRONMENT, not PARTIAL, so a mistype can never read as "the server booted and nobody joined".
//!
//! The log is read once, into memory, and every count and extract comes from that same text. A
//! log that cannot be read is ENVIRONMENT, the same as a log that is not there — never a zero
//! count, which would read as the STALE BUILD verdict.
//!
//! An unset `TBD_SSH_HOST` exits 1 rather than ENVIRONMENT 3: it is a missing required value, the
//! same class as the deploy's own required-variable refusal, and it is reported the same way.

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
