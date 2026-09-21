//! Estimates for the ticket domain.

use anyhow::{Result, bail};

use serde::{Deserialize, Serialize};

use crate::{Corpus, StatusName, Ticket, validate_rfc3339_utc};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cli::commit_subjects::SubjectCommit;
use crate::is_sha_shaped;
use walkdir::WalkDir;

use crate::repository::documentation::TOKEN_ESTIMATE_FACTOR_DOC;
use crate::repository::{ESTIMATES_DIR, ESTIMATES_SCHEMA};

mod model;

pub use model::{CohortKey, EstimateRecord, TOKENS_PER_LOC, estimates_root, validate_estimate};

mod git_changes;

pub use git_changes::{collect_numstat, is_excluded_path, parse_numstat};

mod cohorts;

use cohorts::{Member, attrs_of, cohort_for, estimated_mut, estimated_of, median};

mod planning;

pub use planning::{EstimateReport, plan_estimates};

mod incremental;

pub use incremental::{derivation_shas, plan_estimate_for_id};

use incremental::members_from_existing;

mod storage;

pub use storage::run_estimates;

#[cfg(test)]
use storage::render_estimate;

mod verification;

pub use verification::check_as_errors;

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;

pub(crate) use storage::{load_existing, write_estimate_file};
