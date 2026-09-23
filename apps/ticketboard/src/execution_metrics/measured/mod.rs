use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

mod models;
pub use models::*;
mod aggregation;
pub use aggregation::*;
mod services;
pub use services::*;
mod formatting;
pub use formatting::*;

#[cfg(test)]
#[path = "tests/measured.rs"]
mod tests;
