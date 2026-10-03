//! A child that reads and writes bytes, not text.
//!
//! **Role:** [`Run::binary_output`] and its result [`BinaryOutput`]: stdout captured as the bytes
//! the child wrote, stderr decoded lossily for diagnostics.
//! **Position:** a mode of `crate::run_modes`, re-exported at the crate root; the database
//! restore and dump verification feed a binary archive through it.
//! **Signals & state:** the call owns its child, its stdin writer and its two drain threads until
//! it returns.
//! **Invariants:** stdout is never decoded; the stdin body is written on its own thread while
//! both pipes drain, so a child that answers while it reads cannot deadlock; a deadline kills the
//! child's process group; a signal is [`NotRun::Signalled`].

use std::process::Stdio;
use std::time::{Duration, Instant};

use verification_core::NotRun;

use crate::Run;
use crate::runner::{KillScope, exit_code, feed_stdin, spawn, wait_within};
use crate::stream::SeparateDrains;

/// What a finished process produced when its stdout is bytes — see [`Run::binary_output`].
#[derive(Debug)]
pub struct BinaryOutput {
    /// The raw exit code.
    pub code: i32,
    /// Everything the child wrote to stdout, byte for byte.
    pub stdout: Vec<u8>,
    /// Everything the child wrote to stderr, decoded lossily.
    pub stderr: String,
    /// The wall time from spawn to exit.
    pub duration: Duration,
}

impl Run {
    /// Run to completion, capturing stdout as raw bytes and stderr as text.
    ///
    /// Pair it with [`Run::stdin_bytes`] for a child that takes a binary stream in.
    pub fn binary_output(mut self) -> Result<BinaryOutput, NotRun> {
        let label = self.display();
        let started = Instant::now();

        let (mut cmd, body) = self.command(Stdio::piped(), Stdio::piped());
        let mut child = spawn(&mut cmd, &self.program, &label)?;
        let pgid = child.id() as i32;
        feed_stdin(&mut child, body);
        let drains = SeparateDrains::start(&mut child);

        let status = match wait_within(
            &mut child,
            KillScope::ProcessGroup(pgid),
            self.timeout,
            &label,
        ) {
            Ok(status) => status,
            // As in `Run::output`: only a killed group guarantees the drains reach EOF.
            Err(cause) => {
                if matches!(cause, NotRun::Timeout { .. }) {
                    drains.join_bytes();
                }
                return Err(cause);
            }
        };
        let (stdout, stderr) = drains.join_bytes();
        let code = exit_code(status, &label)?;
        Ok(BinaryOutput {
            code,
            stdout,
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
            duration: started.elapsed(),
        })
    }
}

#[cfg(test)]
#[path = "tests/binary_pipes_tests.rs"]
mod tests;
