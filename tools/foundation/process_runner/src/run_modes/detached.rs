//! A child that outlives the process that started it.
//!
//! **Role:** [`Run::spawn_detached`] and [`Run::spawn_detached_to_files`]: the child starts in
//! its own session and the call returns its pid without waiting.
//! **Position:** a mode of `crate::run_modes`; the `mcpd` broker, the API server the editor
//! gates boot and the desktop "open externally" handler start through it.
//! **Signals & state:** one reaper thread per child waits on it, so a child that exits before
//! this process does is never left a zombie; when this process exits first, the thread ends with
//! it and init adopts the child.
//! **Invariants:** the child leads a new session, so a process-group kill aimed at this process
//! (a CI step teardown, a terminal's Ctrl-C) does not reach it; stdin is `/dev/null` unless the
//! run chose another; a timeout does not apply, since nothing waits.

use std::fs::File;
use std::process::Stdio;

use verification_core::NotRun;

use crate::Run;
use crate::runner::{feed_stdin, spawn};

impl Run {
    /// Start the child in its own session with stdout and stderr inherited, and return its pid
    /// at once.
    pub fn spawn_detached(self) -> Result<u32, NotRun> {
        self.detach(Stdio::inherit(), Stdio::inherit())
    }

    /// Start the child in its own session with stdout written to `stdout` and stderr to
    /// `stderr`, and return its pid at once. The caller opens both files (a shared log is one
    /// file and its `try_clone`).
    pub fn spawn_detached_to_files(self, stdout: File, stderr: File) -> Result<u32, NotRun> {
        self.detach(Stdio::from(stdout), Stdio::from(stderr))
    }

    fn detach(mut self, stdout: Stdio, stderr: Stdio) -> Result<u32, NotRun> {
        let label = self.display();
        let (mut cmd, body) = self.command(stdout, stderr);
        let mut child = spawn(&mut cmd, &self.program, &label)?;
        let pid = child.id();
        feed_stdin(&mut child, body);
        std::thread::spawn(move || {
            let _ = child.wait();
        });
        Ok(pid)
    }
}

#[cfg(test)]
#[path = "tests/detached_tests.rs"]
mod tests;
