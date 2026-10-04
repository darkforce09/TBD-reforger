//! Shared native value and provenance rendering for cards and original documents.
mod provenance;
pub mod value_renderer;
pub(crate) use provenance::metadata;
pub(crate) use value_renderer::{DocumentInspector, display, summary};
