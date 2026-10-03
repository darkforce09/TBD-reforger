//! Running external programs without losing the reason they stopped.
//!
//! **Role:** [`Run`] spawns a child in its own process group, drains both pipes for its whole
//! life, enforces a deadline by killing the group, and reports a signal death, a timeout or a
//! missing program as a [`verification_core::NotRun`] cause, never as an exit code. Beside the
//! captures, a run can share this terminal ([`Run::terminal`]), exchange bytes
//! ([`Run::binary_output`], [`BinaryOutput`]), write to files ([`Run::output_to_files`]), outlive
//! this process ([`Run::spawn_detached`]), stream lines to a caller that may kill it
//! ([`Run::stream_lines`], [`StreamingChild`]) or replace this process
//! ([`Run::replace_process`]). [`which`],
//! [`retry`] and [`wait_for`] look programs up and wait without pretending; [`PathGuard`] puts
//! one more folder first on `PATH` for a scope. [`host_execution`]
//! runs host-linked binaries from inside the development container, and
//! [`secure_shell_transport`] builds the `ssh` argv of one remote command.
//! **Position:** tier 1 of `tools/foundation`, over `verification_core` (the outcome vocabulary)
//! and `libc`. Every xtask command that runs a program, and the xtask gates that read a child's
//! status, call it.
//! **Signals & state:** none; a [`Run`] is a value its spawning call consumes, and each run owns
//! its child and the threads that drain it until it returns, except a detached child, which a
//! reaper thread waits on, and a streamed child, which its [`StreamingChild`] owns.
//! **Invariants:** a signal is never an exit code; a deadline kills the whole process group, not
//! the direct child alone, except for a terminal child, which shares this process's group; a
//! full pipe never blocks a child; a status is passed through raw.

mod error;
pub mod host_execution;
mod lookup;
pub mod prelude;
mod run;
mod run_modes;
mod runner;
mod search_path;
pub mod secure_shell_transport;
mod stream;

pub use error::{Error, Result};
pub use lookup::{retry, wait_for, which};
pub use run::{Merged, Output, Run};
pub use run_modes::{BinaryOutput, StreamingChild};
pub use search_path::PathGuard;
