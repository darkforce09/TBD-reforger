use super::{
    estimated::{EstimatedSortPair, EstimatesState},
    measured::{MetricsState, SortPair},
};
use std::collections::HashMap;
pub(crate) struct MetricsView<'a> {
    pub metrics: &'a MetricsState,
    pub estimates: &'a EstimatesState,
    pub metrics_sort: SortPair,
    pub est_sort: EstimatedSortPair,
    pub id_to_index: &'a HashMap<String, usize>,
}
