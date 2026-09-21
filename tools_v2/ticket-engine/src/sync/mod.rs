//! Sync for the ticket domain.

use anyhow::{Result, bail};

use serde_json::{Value, json};

use crate::registry::*;
use crate::sync::gap_analysis::sync_gap_analysis_ticket_column;
use crate::validation::constants::*;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;
pub mod gap_analysis;

mod runner;

pub use runner::{cmd_sync, refuse_empty_write};

mod registry_views;

use registry_views::{generate_ticket_lead_md, generate_ticket_registry_md};

mod queue_views;

use queue_views::{
    generate_milestones_md, generate_ticket_brainstorm_md, generate_ticket_dev_queue_md,
    generate_ticket_mod_queue_md,
};

mod queue_json;

pub use queue_json::generate_queue_json;

mod markers;

use markers::inject_next_block;
#[cfg(test)]
use markers::marker_inner_is_vacuous;

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
