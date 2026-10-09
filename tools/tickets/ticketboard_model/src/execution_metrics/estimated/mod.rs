//! The historical token estimates, per class and per domain.
//!
//! **Role:** reads `.ai/tickets/estimates/`, checks each file, joins the records to the corpus as
//! `EstimatesState`, sums them per class and per domain, and formats the ticket-detail cells.
//! **Position:** loaded raw by `crate::application_state::background_loading` and built by
//! `WorkspaceState::new`; the desktop application paints the Metrics tab and the detail cells.
//! **Signals & state:** none; plain data and pure functions over file text.
//! **Invariants:** estimated tokens are the `EstimatedTokens` type and never enter a measured total;
//! a malformed file is a named error row and never part of a sum.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use ticket_metrics::estimates::CohortKey;
use ticket_model::{Ticket, TicketId};

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
