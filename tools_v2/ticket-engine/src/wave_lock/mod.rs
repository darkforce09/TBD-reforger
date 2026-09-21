//! Wave lock for the ticket domain.

use anyhow::{Result, bail};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::repository::WAVE_LOCK;
use crate::{StatusName, Ticket};

pub mod archived_wave_plans;
pub mod collisions;
pub mod history;

mod model;

pub use model::{LOCK_VERSION, LockWave, TicketView, WaveLock, lock_path};

use model::HEADER;

mod ticket_views;

pub use ticket_views::{collides, load_views, max_concurrent};

mod parking;

use parking::{carry_emptied, snapshots, wave_zero};

mod packing;

use packing::{greedy_waves, reserved_entry};

mod compiler;

pub use compiler::{compile, compile_reserving, compile_with_cap};

use compiler::{assemble, ledger_base, ledger_floor};

mod persistence;

pub use persistence::{
    cmd_repack, load, missing_lock_error, parse, render, repack_quiet, repack_reserving, write,
};

use persistence::summary;

mod verification;

pub use verification::{check_as_errors, cmd_check};

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
