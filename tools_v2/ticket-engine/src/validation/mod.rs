//! Validation for the ticket domain.

use crate::registry::*;
use crate::repository::gap_analysis_path;
use crate::sync::gap_analysis::test_gap_analysis_round_trip;
use crate::validation::constants::*;
use anyhow::Result;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use walkdir::WalkDir;
pub mod constants;
pub mod vocabulary;

mod schema;

pub use schema::validate_registry_schema;

#[cfg(test)]
use schema::ticket_schema_path;
use schema::{PRIORITY_P, STRICT_RE, validate_registry};

mod scope;

use scope::{check_live_work_surface, check_open_work_owns, check_work_class};

mod body;

#[cfg(test)]
use body::body_findings;
use body::check_body_rules;

mod shipping;

use shipping::{check_estimated_stamp_coherence, check_ship_gate};

mod readiness;

use readiness::{check_plan_ready_gate, check_ready_tier_body, check_work_title_nonempty};

mod debt;

#[cfg(test)]
use debt::pin_growth_finding;
use debt::{check_debt_pins, debt_counter_lines};

mod references;

use references::{check_children_integrity, fossil_paths_check, scan_legacy_ids};

mod runner;

pub use runner::check;

mod command;

pub use command::{cmd_check, require_check_ok, require_check_ok_deferring_repack};

#[cfg(test)]
use command::strict_honesty_counters;

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
