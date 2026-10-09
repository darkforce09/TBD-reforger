//! Streamed subprocesses, bounded output logs, cargo discovery and opening a path externally.
//!
//! **Role:** re-exports `spawn_streaming`, `ProcessHandle` and `ProcessEvent`, `resolve_cargo`, and
//! `BoundedLog`; declares `external_open`.
//! **Position:** `crate::core`; the desktop application runs the strict check, `git status` and
//! every ticket command through it, and `crate::ticket_actions::models` and
//! `crate::repository_status::models` hold its handles and logs.
//! **Signals & state:** a `ProcessHandle` owns its reader and watcher threads and a channel.
//! **Invariants:** the UI thread never blocks on a child, its pipes or its spawn; each process ends
//! in exactly one terminal event.

use process_runner::{Run, StreamingChild};
use std::{
    collections::VecDeque,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::Duration,
};
use verification_core::NotRun;
mod streaming;
pub use streaming::*;
mod cargo_discovery;
pub use cargo_discovery::*;
mod bounded_log;
pub use bounded_log::*;
pub mod external_open;
