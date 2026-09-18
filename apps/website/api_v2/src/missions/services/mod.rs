//! Mission business logic shared by handlers across domains: the canonical mission row reads,
//! the strict export envelope, the mod-document compile adapter, and the registry envelope ingest.

pub mod mission_compile;
pub mod mission_document;
pub mod mission_lookup;
pub mod registry_import;
