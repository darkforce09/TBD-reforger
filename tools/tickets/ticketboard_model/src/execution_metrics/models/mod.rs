//! The borrowed view the Metrics tab paints.
//!
//! **Role:** `MetricsView`: both states, both sort pairs and the ticket id index for one frame.
//! **Position:** built by the desktop application from the loaded board and painted by its Metrics
//! tab.
//! **Signals & state:** none; the view only borrows.
//! **Invariants:** painting cannot change the loaded data; measured and estimated states stay
//! separate fields with separate sorts.

use crate::execution_metrics::{
    estimated::{EstimatedSortPair, EstimatesState},
    measured::{MetricsState, SortPair},
};
use std::collections::HashMap;
/// What the metrics tab reads each frame, borrowed from the workspace.
pub struct MetricsView<'a> {
    /// The measured receipts.
    pub metrics: &'a MetricsState,
    /// The historical estimates.
    pub estimates: &'a EstimatesState,
    /// The measured tables' sorts.
    pub metrics_sort: SortPair,
    /// The estimated tables' sorts.
    pub est_sort: EstimatedSortPair,
    /// Ticket id to corpus index, for the row links.
    pub id_to_index: &'a HashMap<String, usize>,
}
