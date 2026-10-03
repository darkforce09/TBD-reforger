//! A child whose stdout and stderr go straight to files.
//!
//! **Role:** [`Run::output_to_files`]: the child writes into two files the caller opened, and the
//! call returns its raw exit code.
//! **Position:** a mode of `crate::run_modes`; the database backup writes a binary `pg_dump -Fc`
//! stream through it.
//! **Signals & state:** the call owns its child until it is reaped; the files close with it.
//! **Invariants:** nothing the child writes passes through this process's memory; a deadline
//! kills the child's process group; a signal is [`NotRun::Signalled`].

use std::fs::File;
use std::process::Stdio;

use verification_core::NotRun;

use crate::Run;
use crate::runner::{KillScope, exit_code, feed_stdin, spawn, wait_within};

impl Run {
    /// Run to completion with stdout written to `stdout` and stderr to `stderr`, returning the
    /// raw exit code.
    ///
    /// The caller opens both files, so it chooses truncation or appending and names the file in
    /// its own error when one cannot be opened. An unset stdin is `/dev/null`.
    pub fn output_to_files(mut self, stdout: File, stderr: File) -> Result<i32, NotRun> {
        let label = self.display();
        let (mut cmd, body) = self.command(Stdio::from(stdout), Stdio::from(stderr));
        let mut child = spawn(&mut cmd, &self.program, &label)?;
        let pgid = child.id() as i32;
        // `cmd` holds the parent's copies of both files; the child has its own.
        drop(cmd);
        feed_stdin(&mut child, body);
        let status = wait_within(
            &mut child,
            KillScope::ProcessGroup(pgid),
            self.timeout,
            &label,
        )?;
        exit_code(status, &label)
    }
}

#[cfg(test)]
#[path = "tests/file_output_tests.rs"]
mod tests;
