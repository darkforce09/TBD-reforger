//! The wave lock: which ready tickets run together, in which wave.
//!
//! **Role:** compiles `.ai/tickets/wave.lock` from the ticket files ([`compile`], [`cmd_repack`]),
//! reads and writes it ([`load`], [`parse`], [`render`], [`write()`]), checks it against the ticket
//! files ([`check_as_errors`], [`cmd_check`]), reads the wave-close history from git
//! ([`history`], [`archived_wave_plans`]) and reports colliding slices ([`collisions`]).
//! **Position:** tier 3 of `tools/tickets`, over `ticket_model`, `repository_layout` and
//! `process_runner`; `ticket_registry` repacks it after a status change and folds its check into
//! `ticket check`, and xtask's `wave` command group, platform wave driver and mod wave driver
//! read and repack it.
//! **Signals & state:** none in memory; every function takes the checkout root and reads or
//! writes files there or runs `git` in it.
//! **Invariants:** the repack is the lock's one writer; two tickets whose scopes collide never
//! share a wave; a shipped ticket stays in the wave it shipped from until that wave closes.

pub mod archived_wave_plans;
pub mod collisions;
mod compiler;
mod error;
pub mod history;
mod model;
mod packing;
mod parking;
mod persistence;
pub mod prelude;
mod ticket_views;
mod verification;

pub use compiler::{compile, compile_reserving, compile_with_cap};
pub use error::{Error, Result};
pub use model::{LOCK_VERSION, LockWave, TicketView, WaveLock, lock_path};
pub use persistence::{
    cmd_repack, load, missing_lock_error, parse, render, repack_quiet, repack_reserving, write,
};
pub use ticket_views::{collides, load_views, max_concurrent};
pub use verification::{check_as_errors, cmd_check};
