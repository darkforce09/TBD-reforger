//! The names a gate imports with `use verification_core::prelude::*;`.

pub use crate::gate;
pub use crate::lock::{GateLock, flock_exclusive};
pub use crate::pattern::Pattern;
pub use crate::report::Report;
pub use crate::scan;
pub use crate::verdict::{Finding, Kind, NotRun, Verdict};
