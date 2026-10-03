//! The names a caller of the compiler imports with `use mission_compiler::prelude::*;`.

pub use crate::{
    COMPILE_DIAGNOSTIC_RULE_IDS, COMPILER_PACKAGE_VERSION, CompiledOutput, Error, KitSubstitution,
    KitSubstitutionReport, MissionMeta, ModMissionDocument, Result, apply_authored_environment,
    flatten_mod_document_json, flatten_mod_document_json_full,
    flatten_mod_document_json_with_diagnostics, flatten_mod_document_json_with_substitutions,
    flatten_to_mod_document, mission_terrain_key, scan_editor_payload_types,
    unsupported_authored_data,
};
