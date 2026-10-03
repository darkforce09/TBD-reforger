//! A child that shares this process's terminal.
//!
//! **Role:** [`Run::terminal`]: stdin, stdout and stderr inherited, the child awaited, its raw
//! exit code returned.
//! **Position:** a mode of `crate::run_modes`; recipe steps, test suites, `db logs -f` and the
//! interactive `psql` of the xtask command crates run through it.
//! **Signals & state:** the call owns its child until it is reaped.
//! **Invariants:** the child stays in this process's process group and session, so the
//! terminal's Ctrl-C and job control reach it as they reach a shell's foreground job; a deadline
//! therefore kills the child alone, never the group it shares with this process; a signal is
//! [`NotRun::Signalled`].
//!
//! Every other mode of the crate gives its child a new session. A terminal child must not have
//! one: the operator's Ctrl-C goes to the terminal's foreground process group, and a child moved
//! out of it would keep running, still writing to the terminal, after this process died.

use std::process::Stdio;

use verification_core::NotRun;

use crate::Run;
use crate::runner::{KillScope, exit_code, feed_stdin, spawn, wait_within};

impl Run {
    /// Run with stdin, stdout and stderr inherited from this process and return the raw exit
    /// code once the child exits.
    ///
    /// An unset stdin is inherited; [`Run::stdin_null`], [`Run::stdin_file`] and a body replace
    /// it. A timeout kills the child (the direct child only: see the module documentation).
    pub fn terminal(mut self) -> Result<i32, NotRun> {
        let label = self.display();
        let mut cmd = self.base_command();
        cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());
        let body = self.attach_stdin(&mut cmd, Stdio::inherit());
        let mut child = spawn(&mut cmd, &self.program, &label)?;
        feed_stdin(&mut child, body);
        let status = wait_within(&mut child, KillScope::ChildOnly, self.timeout, &label)?;
        exit_code(status, &label)
    }
}

#[cfg(test)]
#[path = "tests/terminal_tests.rs"]
mod tests;
