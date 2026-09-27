//! Shared native value and provenance rendering for cards and original documents.
mod provenance;
mod value_renderer;
pub use provenance::metadata;
pub use value_renderer::{DocumentInspector, display, summary};
