//! What the Metrics tab asks the application to do.
//!
//! **Role:** `MetricsEvent`: select a ticket, or sort a measured or an estimated table.
//! **Position:** emitted by the desktop application's Metrics tab; converted into an `Action` by
//! `crate::application_state::events`.
//! **Signals & state:** none; a plain enum.
//! **Invariants:** measured and estimated sorts stay separate variants, like their tables.

use crate::execution_metrics::{
    estimated::{EstimatedSortKey, EstimatedTableKind},
    measured::{SortKey, TableKind},
};
/// What the metrics tab asks the application to do.
pub enum MetricsEvent {
    /// Select the ticket with this id.
    SelectId(String),
    /// Sort a measured table by this column.
    SortMetrics(TableKind, SortKey),
    /// Sort an estimated table by this column.
    SortEstimates(EstimatedTableKind, EstimatedSortKey),
}
