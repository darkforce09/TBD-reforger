//! `cargo xtask mod remote-logs` — read a staging server's console log and say what happened.
//!
//! **Role:** the log patterns of the verdict and the module tree of `mod remote-logs`: the
//! verdict and the self-test (`execution`), the remote fetch (`remote_fetch`) and the shell
//! quoting and temp files (`shell_quote`); [`run`] is the entry. Four outcomes: 0 HEALTHY · 1
//! FAIL · 2 PARTIAL · 3 ENVIRONMENT.
//!
//! **Position:** called by `xtask`'s `mod` dispatch and run last by `cargo xtask deploy staging`.
//! The host and the profile folder come from `deploy.env` under the precedence rule of
//! [`deploy_settings`]; `--instance N` reads fleet instance N's log under
//! `~/tbd/fleet/instance-N/profile`, and without it the single server's profile folder is read.
//!
//! **Signals & state:** none held; a remote run writes one temp copy of the log and removes it.
//!
//! **Invariants:** a usage error or a bad flag is ENVIRONMENT, not PARTIAL, so a mistype can
//! never read as "the server booted and nobody joined". The log is read once, into memory, and
//! every count and extract comes from that same text; a log that cannot be read is ENVIRONMENT,
//! the same as a log that is not there — never a zero count, which would read as the STALE BUILD
//! verdict. A host that runs a fleet, read without `--instance`, is ENVIRONMENT with a message
//! naming `--instance`. A missing or malformed setting exits 1 rather than ENVIRONMENT 3: it is a
//! refused setting, the same class as the deploy's own refusals, and it is reported the same way.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use process_runner::Run;
use verification_core::NotRun;
use verification_core::gate::probe_str;
use verification_core::pattern::Pattern;

use repository_root::find_repository_root;

use crate::error::{Result, ResultExt};

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

mod execution;
pub use execution::run;

mod remote_fetch;

mod shell_quote;
use shell_quote::append_line;
use shell_quote::shell_quote;
use shell_quote::tempfile_dir;
use shell_quote::write_log;
