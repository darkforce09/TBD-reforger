//! Admin domain — user/server administration plus the [`audit`] console. T-934.15:
//! the flat files moved here unchanged; the same-named `admin.rs` is glob
//! re-exported so `handlers::admin::*` paths hold.

mod admin;
pub use self::admin::*;

pub mod audit;
