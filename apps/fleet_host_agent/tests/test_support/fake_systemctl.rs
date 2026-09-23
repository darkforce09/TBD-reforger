//! A stand-in systemctl program for the process-control tests. Each test writes one into its
//! own temporary directory: a small POSIX sh script that records every argument vector it is
//! run with and answers `start`, `stop`, `restart` and `show --property=... --value` from state
//! files the test controls. It uses shell builtins only, because the agent runs it with an
//! empty environment (no PATH). A verb changes the unit's state before the invocation is
//! recorded, so a test that sees the invocation also sees its effect.
//!
//! A stand-in holds a lock of its test binary for its whole life, so the tests that use one run
//! one at a time: a script that another thread's forked child still holds open for writing
//! cannot be executed ("Text file busy").

use std::fs::{self, Permissions};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use tempfile::TempDir;
use tokio::sync::{Mutex, MutexGuard};

static ONE_SCRIPT_AT_A_TIME: Mutex<()> = Mutex::const_new(());

const SCRIPT: &str = r#"#!/bin/sh
state='STATE_DIRECTORY'
case "$2" in
start|stop|restart)
    read -r next_state < "$state/active_state_after_verb"
    printf '%s\n' "$next_state" > "$state/active_state"
    ;;
esac
printf '%s\n' "$*" >> "$state/invocations"
case "$2" in
start|stop|restart)
    read -r verb_exit < "$state/verb_exit"
    if [ "$verb_exit" = hang ]; then
        while :; do :; done
    fi
    exit "$verb_exit"
    ;;
show)
    case "$3" in
    --property=LoadState) read -r value < "$state/load_state" ;;
    --property=ActiveState) read -r value < "$state/active_state" ;;
    *) exit 64 ;;
    esac
    read -r show_exit < "$state/show_exit"
    printf '%s\n' "$value"
    exit "$show_exit"
    ;;
esac
exit 64
"#;

/// How the stand-in unit behaves.
pub struct UnitScenario {
    pub load_state: &'static str,
    /// The ActiveState before any verb runs.
    pub active_state: &'static str,
    /// The ActiveState every start, stop or restart leaves behind.
    pub active_state_after_verb: &'static str,
    /// The verb's exit status, or `"hang"` for a verb that never returns.
    pub verb_exit: &'static str,
    /// The exit status of `show`.
    pub show_exit: u8,
}

pub struct FakeSystemctl {
    directory: TempDir,
    _one_at_a_time: MutexGuard<'static, ()>,
}

impl FakeSystemctl {
    pub async fn install(scenario: &UnitScenario) -> Self {
        let one_at_a_time = ONE_SCRIPT_AT_A_TIME.lock().await;
        let directory = tempfile::tempdir().expect("a temporary directory");
        let state = directory.path().to_str().expect("a UTF-8 path").to_owned();
        assert!(!state.contains('\''), "the script quotes its directory");
        let fake = Self {
            directory,
            _one_at_a_time: one_at_a_time,
        };
        fake.write("load_state", scenario.load_state);
        fake.write("active_state", scenario.active_state);
        fake.write("active_state_after_verb", scenario.active_state_after_verb);
        fake.write("verb_exit", scenario.verb_exit);
        fake.write("show_exit", &scenario.show_exit.to_string());
        let program = fake.program();
        fs::write(&program, SCRIPT.replace("STATE_DIRECTORY", &state)).expect("the script");
        fs::set_permissions(program, Permissions::from_mode(0o700)).expect("an executable script");
        fake
    }

    pub fn program(&self) -> PathBuf {
        self.directory.path().join("systemctl")
    }

    /// Every argument vector the program was run with, one line each.
    pub fn invocations(&self) -> Vec<String> {
        fs::read_to_string(self.directory.path().join("invocations"))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// Changes the unit's ActiveState, as a unit that dies or recovers on its own.
    #[allow(
        dead_code,
        reason = "the ledger tests share this stand-in but never change the unit from outside"
    )]
    pub fn set_active_state(&self, state: &str) {
        self.write("active_state", state);
    }

    fn write(&self, name: &str, value: &str) {
        fs::write(self.directory.path().join(name), format!("{value}\n")).expect("a state file");
    }
}
