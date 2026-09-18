//! Missions domain handlers that have not yet moved into [`crate::missions::handlers`]: the
//! export / compiled / ingest / instrumentation surfaces in the same-named `missions.rs`, plus
//! the [`approvals`] queue and the asset [`registry`].

// Deliberate inception: the domain keeps its same-named root handler file, glob re-exported so
// `handlers::missions::…` resolves the handlers it still owns.
#[allow(clippy::module_inception)]
mod missions;
pub use self::missions::*;

pub mod approvals;
pub mod registry;
