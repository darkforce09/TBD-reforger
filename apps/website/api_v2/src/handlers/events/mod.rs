//! Events domain — event / ORBAT scheduling.

// Deliberate inception: the domain keeps its same-named root handler file, and the glob
// re-export below is what makes every `handlers::events::…` path resolve through it.
#[allow(clippy::module_inception)]
mod events;
pub use self::events::*;
