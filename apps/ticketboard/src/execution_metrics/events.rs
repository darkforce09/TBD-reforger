use super::{
    estimated::{EstimatedSortKey, EstimatedTableKind},
    measured::{SortKey, TableKind},
};
pub(crate) enum MetricsEvent {
    SelectId(String),
    SortMetrics(TableKind, SortKey),
    SortEstimates(EstimatedTableKind, EstimatedSortKey),
}
