//! The API's equipment datasets: read-only browsing and verified local imports of published
//! Workbench data.
//!
//! **Role:** imports the equipment datasets the Workbench export publishes into a local data
//! directory with a navigation index, and answers the generation-pinned read queries behind the
//! equipment data viewer routes.
//! **Position:** above `api_identifiers` and `content_digest`; the API's application state holds
//! one [`EquipmentDatasets`], the equipment data viewer handlers of `api_community_content` read
//! through it and the `equipment_export_watcher` background worker drives its imports; names no
//! other API crate and no domain.
//! **Signals & state:** each [`EquipmentDataService`] holds its loaded generations, the document
//! cache and the import progress behind locks; see `service_state`.
//! **Invariants:** a generation becomes current only after every file matched its manifest and its
//! index was sealed; a query answers from one pinned generation; every answer stays under
//! [`PAGE_BYTES`].

mod dataset_catalogs;
pub mod error;
pub mod importing;
pub mod index;
pub mod prelude;
pub mod queries;
mod service_state;
pub mod source;

pub use dataset_catalogs::EquipmentDatasets;
pub use error::{Error, Result};
pub use index::INDEX_VERSION;
pub use queries::PAGE_BYTES;
pub use service_state::{Dataset, EquipmentDataService, ImportProgress};

#[cfg(test)]
#[path = "tests/import_recovery.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/gameplay_import.rs"]
mod gameplay_tests;
