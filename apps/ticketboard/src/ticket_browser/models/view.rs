use super::{
    program_tree::{FlatRow, TreeModel},
    status_board::BoardModel,
};
use crate::{
    execution_metrics::estimated::EstimatesState, ticket_registry::models::corpus::Corpus,
};
/// Borrowed browser data; rendering cannot reach application jobs or mutate the registry.
pub(crate) struct BrowserView<'a> {
    pub corpus: &'a Corpus,
    pub board: &'a BoardModel,
    pub expanded: &'a [bool; 8],
    pub visible: &'a [Vec<usize>; 8],
    pub selected: Option<usize>,
    pub compare: Option<usize>,
    pub tree: &'a TreeModel,
    pub tree_flat: &'a [FlatRow],
    pub legacy_expanded: bool,
    pub estimates: &'a EstimatesState,
}
pub(crate) struct DraggedTicket(pub usize);
