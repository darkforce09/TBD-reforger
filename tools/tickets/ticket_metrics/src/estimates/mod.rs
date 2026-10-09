//! Token estimates for shipped tickets that have no measured run receipt.
//!
//! **Role:** plans, writes, loads and checks the files under `.ai/tickets/estimates/`: one
//! [`EstimateRecord`] per shipped ticket without a receipt, from the lines its commits changed
//! ([`collect_numstat`], `diff_loc`) or from the median of similar tickets (`cohort_median`),
//! for every shipped ticket at once ([`plan_estimates`], [`run_estimates`]) or for one ticket at
//! `stamp-sha` time ([`plan_estimate_for_id`]); and the `ticket check` rules over the tree
//! ([`check_as_errors`]).
//! **Position:** over `ticket_model` (the corpus, the subject commits, the factor document's
//! path and the excluded paths), `repository_layout` and the receipt lookup of this crate;
//! `ticket_registry`'s `stamp-sha` verb and `ticket check` call it.
//! **Signals & state:** none; functions read git history, the ticket corpus and the estimate
//! tree of the checkout they are given, and the writing pass rewrites ticket files.
//! **Invariants:** an estimate is derived from git history and the corpus alone and validates
//! against `.ai/tickets/estimates.schema.json` and [`validate_estimate`] before it is written;
//! a ticket has a receipt or an estimate, never both; `factor` is always [`TOKENS_PER_LOC`];
//! estimate files never sit inside the receipt tree, so one can never pass for a measurement.

use serde::{Deserialize, Serialize};

use process_runner::Run;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use ticket_model::{Corpus, StatusName, Ticket, TicketId};
use time_source::validate_rfc3339_utc;

use ticket_model::commit_subjects::SubjectCommit;
use ticket_model::is_sha_shaped;
use walkdir::WalkDir;

use repository_layout::{ESTIMATES_DIR, ESTIMATES_SCHEMA};
use ticket_model::repository::documentation::TOKEN_ESTIMATE_FACTOR_DOC;

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

pub use storage::{load_existing, run_estimates, write_estimate_file};

mod verification;

pub use verification::check_as_errors;
