//! The debounced file watch.
//!
//! **Role:** declares `file_watch`: the watch on `.ai/tickets/`, the repository root and the roadmap
//! folder, and its debouncer.
//! **Position:** armed and polled by the desktop application.
//! **Signals & state:** none here; `file_watch` owns the `notify` watchers and a channel.
//! **Invariants:** a ticket command's own writes never start a strict check.

pub mod file_watch;
