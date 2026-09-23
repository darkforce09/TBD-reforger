use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use ticket_engine::Ticket;

use crate::execution_metrics::measured::{ErrorRow, format_tokens, valid_git_sha, valid_ticket_id};
use crate::ticket_registry::models::corpus::Corpus;
use crate::ticket_registry::models::projection as board;

mod models;
pub use models::*;
mod validation;
pub use validation::*;
mod services;
pub use services::*;
mod detail_projection;
pub use detail_projection::*;
mod aggregation;
pub use aggregation::*;

#[cfg(test)]
#[path = "tests/estimated.rs"]
mod tests;
