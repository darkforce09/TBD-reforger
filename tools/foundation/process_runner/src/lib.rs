//! Running external programs without losing the reason they stopped.
//!
//! **Role:** [`Run`] spawns a child in its own process group, drains both pipes for its whole
//! life, enforces a deadline by killing the group, and reports a signal death, a timeout or a
//! missing program as a [`verification_core::NotRun`] cause, never as an exit code; [`which`],
//! [`retry`] and [`wait_for`] look programs up and wait without pretending. [`host_execution`]
//! runs host-linked binaries from inside the development container, and
//! [`secure_shell_transport`] builds the `ssh` argv of one remote command.
//! **Position:** tier 1 of `tools/foundation`, over `verification_core` (the outcome vocabulary)
//! and `libc`. Every xtask command that runs a program, and the xtask gates that read a child's
//! status, call it.
//! **Signals & state:** none; a [`Run`] is a value its spawning call consumes, and each run owns
//! its child and the threads that drain it until it returns.
//! **Invariants:** a signal is never an exit code; a deadline kills the whole process group, not
//! the direct child alone; a full pipe never blocks a child; a status is passed through raw.

mod error;
pub mod host_execution;
mod lookup;
pub mod prelude;
mod run;
mod runner;
pub mod secure_shell_transport;
mod stream;

pub use error::{Error, Result};
pub use lookup::{retry, wait_for, which};
pub use run::{Merged, Output, Run};
