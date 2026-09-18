//! Admin domain — user/server administration plus the [`audit`] console. T-934.15:
//! the flat files moved here unchanged; the same-named `admin.rs` is glob
//! re-exported so `handlers::admin::*` paths hold.

// Deliberate inception: the domain keeps its same-named root handler file so every
// pre-T-934.15 `handlers::admin::…` path resolves through the glob re-export below.
#[allow(clippy::module_inception)]
mod admin;
pub use self::admin::*;

pub mod audit;
