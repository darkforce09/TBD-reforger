//! Replacing this process with the child, as a shell's `exec` does.
//!
//! **Role:** [`Run::replace_process`]: the program takes over this process's pid, process
//! group, terminal and stdio, and this process's code never runs again.
//! **Position:** a mode of `crate::run_modes`; `cargo xtask mod spawn-verify --selftest` hands
//! itself over to `cargo run … mcp wb-logs --selftest` through it.
//! **Signals & state:** none; on success there is no caller left to hold any.
//! **Invariants:** the call returns only when the replacement failed, and then with the cause;
//! no new session is made, so the replacement keeps this process's place in the terminal's job
//! control.

use std::process::Stdio;

use verification_core::NotRun;

use crate::Run;

impl Run {
    /// Replace this process with the child; returns only the reason the replacement failed.
    ///
    /// The working directory, the environment and an explicit [`Run::stdin_null`] or
    /// [`Run::stdin_file`] apply; an unset stdin, stdout and stderr stay this process's own. A
    /// stdin body cannot be fed, since no writer survives the replacement, and is refused, as is
    /// a timeout, since nothing remains to enforce it.
    pub fn replace_process(mut self) -> NotRun {
        use std::os::unix::process::CommandExt;

        let label = self.display();
        if self.timeout.is_some() {
            return refused(label, "a replaced process cannot carry a timeout");
        }
        let mut cmd = self.base_command();
        if self.attach_stdin(&mut cmd, Stdio::inherit()).is_some() {
            return refused(label, "a replaced process cannot be fed a stdin body");
        }
        let error = cmd.exec();
        if error.kind() == std::io::ErrorKind::NotFound {
            NotRun::ToolAbsent(self.program)
        } else {
            NotRun::ToolError {
                tool: label,
                status: -1,
                stderr: format!("exec failed: {error}"),
            }
        }
    }
}

fn refused(label: String, reason: &str) -> NotRun {
    NotRun::ToolError {
        tool: label,
        status: -1,
        stderr: reason.to_string(),
    }
}

#[cfg(test)]
#[path = "tests/process_replacement_tests.rs"]
mod tests;
