//! The names a caller imports with `use schema_tooling::prelude::*;`.

pub use crate::generate::cli::GenCmd;
pub use crate::generate::dispatch::run_gen_command;
pub use crate::generate::schema_types::{codegen, verify_fresh};
pub use crate::mission_flattening::flatten_orbat_slots;
pub use crate::schema_checks::{
    map_glyphs, map_object_enums, type_inventory, validate_all, validate_file,
};
