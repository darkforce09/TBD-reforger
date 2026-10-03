//! A child whose output a caller reads line by line while it runs, and may kill.
//!
//! **Role:** [`Run::stream_lines`] and its handle [`StreamingChild`]: each line of stdout and
//! stderr arrives on a channel as the child writes it; the handle polls, waits on or kills the
//! child.
//! **Position:** a mode of `crate::run_modes`, re-exported at the crate root; the ticketboard's
//! desktop application streams its strict check, `git status` and ticket commands through it.
//! **Signals & state:** one reader thread per pipe sends into the channel and ends at its pipe's
//! EOF or when the receiver is dropped; the [`StreamingChild`] owns the child.
//! **Invariants:** the child leads its own process group, so [`StreamingChild::kill`] and a
//! deadline reach every grandchild (a `cargo run` binary as well as `cargo`); the channel closes
//! once both pipes reach EOF; lines are decoded lossily without their `\n` or `\r\n`; a signal is
//! [`NotRun::Signalled`].

use std::process::{Child, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Instant;

use verification_core::NotRun;

use crate::Run;
use crate::runner::{KillScope, POLL, exit_code, feed_stdin, kill, spawn};
use crate::stream::start_line_drains;

/// A running child whose output streams over a channel — see [`Run::stream_lines`].
#[derive(Debug)]
pub struct StreamingChild {
    child: Child,
    label: String,
    deadline: Option<(Instant, u64)>,
    reaped: bool,
}

impl Run {
    /// Start the child with stdout and stderr each read line by line, returning its handle and
    /// the channel the lines arrive on, both streams in arrival order.
    ///
    /// An unset stdin is `/dev/null`. A timeout is enforced by [`StreamingChild::try_wait`] and
    /// [`StreamingChild::wait`]: past it they kill the group and answer
    /// [`NotRun::Timeout`].
    pub fn stream_lines(mut self) -> Result<(StreamingChild, Receiver<String>), NotRun> {
        let label = self.display();
        let (mut cmd, body) = self.command(Stdio::piped(), Stdio::piped());
        let mut child = spawn(&mut cmd, &self.program, &label)?;
        feed_stdin(&mut child, body);
        let (sender, lines) = mpsc::channel();
        start_line_drains(&mut child, &sender);
        let deadline = self
            .timeout
            .map(|limit| (Instant::now() + limit, limit.as_secs()));
        Ok((
            StreamingChild {
                child,
                label,
                deadline,
                reaped: false,
            },
            lines,
        ))
    }
}

impl StreamingChild {
    /// The child's pid, which is also its process group id.
    pub fn id(&self) -> u32 {
        self.child.id()
    }

    /// SIGKILL to the child's whole process group; a group already gone is no error, and once
    /// the child has been reaped this does nothing, since its pid may since have been reused.
    pub fn kill(&mut self) {
        if self.reaped {
            return;
        }
        // `setsid` made the child a group leader, so its pgid equals its pid.
        let scope = KillScope::ProcessGroup(self.child.id() as i32);
        kill(&mut self.child, scope);
    }

    /// The raw exit code once the child has exited, `None` while it runs; past the deadline the
    /// group is killed and the answer is [`NotRun::Timeout`].
    pub fn try_wait(&mut self) -> Result<Option<i32>, NotRun> {
        match self.child.try_wait() {
            Ok(Some(status)) => {
                self.reaped = true;
                exit_code(status, &self.label).map(Some)
            }
            Ok(None) => match self.deadline {
                Some((at, secs)) if Instant::now() >= at => {
                    self.kill();
                    let _ = self.child.wait();
                    self.reaped = true;
                    Err(NotRun::Timeout {
                        tool: self.label.clone(),
                        secs,
                    })
                }
                _ => Ok(None),
            },
            Err(e) => Err(NotRun::ToolError {
                tool: self.label.clone(),
                status: -1,
                stderr: format!("try_wait failed: {e}"),
            }),
        }
    }

    /// Wait for the child to exit and return its raw exit code, enforcing the deadline.
    pub fn wait(mut self) -> Result<i32, NotRun> {
        if self.deadline.is_none() {
            let status = self.child.wait().map_err(|e| NotRun::ToolError {
                tool: self.label.clone(),
                status: -1,
                stderr: format!("wait failed: {e}"),
            })?;
            return exit_code(status, &self.label);
        }
        loop {
            if let Some(code) = self.try_wait()? {
                return Ok(code);
            }
            std::thread::sleep(POLL);
        }
    }
}

#[cfg(test)]
#[path = "tests/line_stream_tests.rs"]
mod tests;
