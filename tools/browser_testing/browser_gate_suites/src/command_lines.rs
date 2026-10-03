//! The `gate` and `capture` command lines.
//!
//! **Role:** parses the `gate` and `capture` arguments, runs each subcommand's gate or capture
//! driver of this crate, and turns its outcome into the process exit code.
//! **Position:** the library half of the `gate` and `capture` binaries of `developer_tools`
//! (`tools/developer_tools/src/bin/gate.rs` and `capture.rs`), which call [`gate::run`] and
//! [`capture::run`] and nothing else.
//! **Signals & state:** none at this level; each command line builds the tokio runtime its
//! command runs on, and each gate owns its own server and browser.
//! **Invariants:** the command lines alone decide the exit codes; an [`crate::Error`] a gate
//! returns is printed with every cause ([`crate::Error::with_causes`]), never swallowed.

pub mod capture;
pub mod gate;
