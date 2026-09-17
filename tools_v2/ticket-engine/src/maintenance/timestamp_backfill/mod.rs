//! Timestamp backfill for the ticket domain.

use anyhow::{Result, bail};

use crate::store::{is_parent_id, parent_numeric_id};
use crate::{Corpus, Status, StatusName, Ticket, validate_rfc3339_utc};
use regex::Regex;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::LazyLock;

use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

/// 7–40 lowercase hex — the repo's stamp/estimate SHA shape. Moved to `tbd-tickets`
/// at T-917.6 (the `ops::stamp_sha` refusal and the ship gate judge the same shape);
/// re-exported here so the miner, the T-917.5 estimates check and the gate keep one
/// authority.
pub(crate) use crate::is_sha_shaped;

mod history;

pub use history::{SubjectCommit, mine_subjects};

use history::{
    day_floor, estimated_of, is_date_shaped, parse_utc, shipped_sha_of, short_sha, stamps_of,
};
#[cfg(test)]
use history::{subject_ids, to_utc_z};

mod interpolation;

#[cfg(test)]
use interpolation::Anchor;
use interpolation::{build_anchors, method2_dates};

mod planning;

pub use planning::{BackfillReport, backfill};

use planning::{Plan, ShippedAction};

mod application;

pub use application::cmd_backfill_stamps;

use application::apply_plan;

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
