//! The ticket-free foundations: streamed subprocesses, bounded logs, cargo discovery, opening a
//! path externally and wall-clock labels.
//!
//! **Role:** runs programs on worker threads and streams their output ([`process`]), and labels
//! the wall clock ([`time`]).
//! **Position:** the bottom of the crate; every feature and the application may use it.
//! **Signals & state:** [`process::ProcessHandle`] owns worker threads and a channel.
//! **Invariants:** imports no feature module and nothing from `ticket_model`.

pub mod process;
pub mod time;
