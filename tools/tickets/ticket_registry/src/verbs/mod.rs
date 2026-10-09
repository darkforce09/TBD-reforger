//! The bodies of the `cargo xtask ticket` verbs.
//!
//! **Role:** one function per verb: the read verbs (`show`, `get`, `list`, `next`, `brief`,
//! `prompt`, `config`, `ready-ids`, `plan-batch`, `sparse-paths`, `scope-histogram`,
//! `gap-round-trip`), the writing verbs (`add`, `add-child`, `remove`, `reorder`,
//! `advance-slice`, `set-status`, `mark-ready`, `ship`, `stamp-sha`) and the batch `run`.
//! **Position:** the xtask `ticket` command group parses the arguments and calls these with the
//! checkout root and the loaded registry value; they use `crate::validation`, `crate::ops`,
//! `crate::sync` and `ticket_wave_lock`.
//! **Signals & state:** none; prints the command output and writes ticket files, `queue.json`,
//! the roadmap block, the gap-analysis column and the wave lock.
//! **Invariants:** a writing verb runs the `ticket check` preflight before it changes a file
//! (`stamp-sha` alone excepted), writes only through a typed operation, and reloads the registry
//! before it syncs; a verb never starts an agent or removes a worktree itself — `run` takes the
//! executor as a callback and [`cleanup_targets`] only resolves paths.

use crate::error::{Error, Result};

use serde_json::{Value, json};

use crate::ops;
use std::fs;
use std::path::Path;
use ticket_model::{Corpus, Ticket, TicketId};

use crate::registry::*;
use crate::sync::gap_analysis::test_gap_analysis_round_trip;
use crate::sync::{cmd_sync, generate_queue_json, refuse_empty_write};
use crate::validation::require_check_ok;
use crate::verbs::prompt::extract_prompt;
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

pub use shipping::{cmd_ship, cmd_ship_opt, cmd_stamp_sha, stamp_sha_with_inputs};

mod readiness;

pub use readiness::cmd_mark_ready;

mod mutations;

pub use mutations::{cmd_add, cmd_add_child, cmd_advance_slice, cmd_remove, cmd_reorder};

mod status;

pub use status::{cmd_ready_ids, cmd_set_status};

mod batch;

pub use batch::{CleanupTargets, ExecutorResult, cleanup_targets, cmd_config, cmd_get, cmd_run};
