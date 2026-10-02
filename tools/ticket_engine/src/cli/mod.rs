//! Cli for the ticket domain.

use anyhow::{Result, bail};

use serde_json::{Value, json};

use crate::{Corpus, Ticket, ops};
use std::fs;
use std::path::Path;

use crate::cli::prompt::extract_prompt;
use crate::registry::*;
use crate::sync::gap_analysis::test_gap_analysis_round_trip;
use crate::sync::{cmd_sync, generate_queue_json, refuse_empty_write};
use crate::validation::require_check_ok;
pub mod prompt;

mod brief;

pub use brief::cmd_brief;

use brief::VALID_TICKET_STATUSES;

mod queries;

pub use queries::{
    cmd_gap_round_trip, cmd_list, cmd_next, cmd_plan_batch, cmd_prompt, cmd_scope_histogram,
    cmd_show, cmd_sparse_paths, require_ticket, unknown_ticket,
};

mod mutation_support;

use mutation_support::{load_corpus, refresh_wave_lock, refuse_verbatim, reload_registry};

mod shipping;

pub use shipping::commit_subjects;
pub use shipping::{cmd_ship, cmd_ship_opt, cmd_stamp_sha, stamp_sha_with_inputs};

mod readiness;

pub use readiness::cmd_mark_ready;

mod mutations;

pub use mutations::{cmd_add, cmd_add_child, cmd_advance_slice, cmd_remove, cmd_reorder};

mod status;

pub use status::{cmd_ready_ids, cmd_set_status};

mod batch;

pub use batch::{CleanupTargets, cleanup_targets, cmd_config, cmd_get, cmd_run};

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
