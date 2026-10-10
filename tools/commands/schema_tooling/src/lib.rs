//! The contract tooling of `cargo xtask schema` and `cargo xtask gen`.
//!
//! **Role:** [`codegen`] renders the `contract_schema_types` crate's generated module tree from
//! the JSON Schemas in `contracts/definitions/` with `typify`, and [`verify_fresh`] proves the
//! committed tree matches; the schema gates ([`validate_all`], [`validate_file`],
//! [`map_object_enums`], [`type_inventory`], [`map_glyphs`]) hold the schemas and every fixture and
//! terrain document that follows them to one another;
//! [`flatten_orbat_slots`] turns a mission's ORBAT template into its `slots[]`; [`GenCmd`] and
//! [`run_gen_command`] are the `gen` group (the Spleen font table).
//! **Position:** tier 5 of `tools/commands`, over `repository_layout`,
//! `process_runner` (rustfmt) and `prefab_catalog`
//! (the census kind list). The xtask binary's
//! `schema`, `gen` and `ci` groups call it; the `schema` command line stays in the binary.
//! **Signals & state:** none; each call reads the checkout and prints its own report.
//! **Invariants:** a gate's verdict is its exit code (0 pass, 1 findings) and a gate that examined
//! nothing fails; everything under the generated folder is generator output; the same schemas
//! always render the same bytes.

mod error;
mod generate;
mod mission_flattening;
pub mod prelude;
mod schema_checks;

pub use error::{Error, Result};
pub use generate::cli::GenCmd;
pub use generate::dispatch::run_gen_command;
pub use generate::schema_types::{codegen, verify_fresh};
pub use mission_flattening::flatten_orbat_slots;
pub use schema_checks::{
    map_glyphs, map_object_enums, type_inventory, validate_all, validate_file,
};
