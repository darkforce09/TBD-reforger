//! Role: Module boundary for mission/validation/wire_safety.
//! Position: `mission/validation/wire_safety` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use serde_json::Value;
use std::collections::HashMap;
mod scan;
/// Expose scan :: cargo capacity caveat at this domain boundary.
pub use scan::CARGO_CAPACITY_CAVEAT;
/// Expose scan ::  cargo phys at this domain boundary.
pub use scan::CargoPhys;
/// Expose scan ::  cargo phys catalog at this domain boundary.
pub use scan::CargoPhysCatalog;
/// Expose scan :: max reported at this domain boundary.
pub use scan::MAX_REPORTED;
/// Expose scan :: describe at this domain boundary.
pub use scan::describe;
/// Expose scan :: first unsafe byte at this domain boundary.
pub use scan::first_unsafe_byte;
/// Expose scan :: is wire unsafe at this domain boundary.
pub use scan::is_wire_unsafe;
/// Expose scan :: quote value at this domain boundary.
pub use scan::quote_value;
/// Expose scan :: scan cargo capacity at this domain boundary.
pub use scan::scan_cargo_capacity;
/// Expose scan :: scan editor payload at this domain boundary.
pub use scan::scan_editor_payload;
#[cfg(test)]
mod tests;
