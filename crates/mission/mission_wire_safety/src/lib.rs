//! The two code-expressed safety scans of a mission editor payload.
//!
//! **Role:** finds the authored names that would carry a control character into the compiled
//! mission document, which the schema's `wireSafeString` forbids ([`scan_editor_payload`]), and
//! the slot cargo heavier or bulkier than the garment that holds it ([`scan_cargo_capacity`]),
//! each as one readable line per finding.
//! **Position:** mission tier 0, depending on `serde_json` only. The API runs both scans on every
//! save and refuses a compile on a capacity line; the mission compiler and validator crates call
//! the byte rule, the reporting cap and the capacity scan.
//! **Signals & state:** none; pure functions over a parsed payload and a catalog the caller builds.
//! **Invariants:** the byte rule catches exactly the characters `wireSafeString` forbids; reports
//! stay bounded by [`MAX_REPORTED`] distinct values; an empty catalog never invents a limit.

pub mod prelude;
mod scan;

/// The sentence every over-capacity line ends with: why the finding refuses rather than predicts.
pub use scan::CARGO_CAPACITY_CAVEAT;
/// The catalogued weight, volume and capacity of one registry item.
pub use scan::CargoPhys;
/// The registry items by resource name, as [`scan_cargo_capacity`] reads them.
pub use scan::CargoPhysCatalog;
/// The number of distinct bad values a scan reports before one tail line counts the rest.
pub use scan::MAX_REPORTED;
/// The name of a control byte an author can act on, such as `TAB (U+0009)`.
pub use scan::describe;
/// The first byte of a string that `wireSafeString` forbids, if any.
pub use scan::first_unsafe_byte;
/// Whether `wireSafeString` forbids a byte: 0x00 to 0x1F and 0x7F.
pub use scan::is_wire_unsafe;
/// A value quoted for a log line or an error body, control characters escaped, long values elided.
pub use scan::quote_value;
/// The over-capacity lines of every slot's cargo against the garments' catalogued maximums.
pub use scan::scan_cargo_capacity;
/// The control-character lines of every authored name that lands in the compiled document.
pub use scan::scan_editor_payload;

#[cfg(test)]
mod tests;
