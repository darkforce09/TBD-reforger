//! Read-only browsing and verified local imports of published Workbench data.
mod dataset_catalogs;
pub mod importing;
pub mod index;
pub mod queries;
mod service_state;
pub mod source;

pub use dataset_catalogs::EquipmentDatasets;
pub use service_state::{Dataset, EquipmentDataService, ImportProgress};

pub const INDEX_VERSION: &str = "1";
pub const PAGE_BYTES: usize = 256 * 1024;

#[cfg(test)]
#[path = "tests/import_recovery.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/gameplay_import.rs"]
mod gameplay_tests;
