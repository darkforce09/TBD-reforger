//! The borrowed view the browser paints.
//!
//! **Role:** `BrowserView`, the corpus, board, tree, rows, selection, comparison and estimates for
//! one frame, and `DraggedTicket`, the drag payload of a card.
//! **Position:** lent by the desktop application to its Board and Tree tabs and the detail column.
//! **Signals & state:** none; the view only borrows.
//! **Invariants:** rendering can neither reach application jobs nor change the registry.

use super::{
    program_tree::{FlatRow, TreeModel},
    status_board::BoardModel,
};
use crate::{
    execution_metrics::estimated::EstimatesState, ticket_registry::models::corpus::Corpus,
};
/// Borrowed browser data; rendering cannot reach application jobs or mutate the registry.
pub struct BrowserView<'a> {
    /// The loaded corpus.
    pub corpus: &'a Corpus,
    /// The status board.
    pub board: &'a BoardModel,
    /// Which columns are expanded.
    pub expanded: &'a [bool; 8],
    /// Per column, the card rows the filters let through.
    pub visible: &'a [Vec<usize>; 8],
    /// The selected ticket's corpus index.
    pub selected: Option<usize>,
    /// The comparison ticket's corpus index.
    pub compare: Option<usize>,
    /// The program tree.
    pub tree: &'a TreeModel,
    /// The flattened tree rows.
    pub tree_flat: &'a [FlatRow],
    /// True when the quarantined parked lines are expanded.
    pub quarantine_expanded: bool,
    /// The historical estimates, for the stamp cells.
    pub estimates: &'a EstimatesState,
}
/// The drag payload of a card: the dragged ticket's corpus index.
pub struct DraggedTicket(pub usize);
