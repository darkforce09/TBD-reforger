//! Mission business logic shared by handlers across domains: the canonical mission row reads,
//! the registry phys catalog the cargo walk measures against, the strict export envelope, the
//! mod-document compile adapter, and the registry envelope ingest.

pub mod cargo_catalog;
pub mod mission_compile;
pub mod mission_document;
pub mod mission_lookup;
pub mod registry_import;
