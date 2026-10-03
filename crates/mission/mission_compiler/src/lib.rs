//! The mission game-document compiler.
//!
//! **Role:** compiles a stored editor payload and its mission metadata into the game document a
//! server loads ([`flatten_to_mod_document`] and the JSON entry points beside it), with the
//! findings of everything the compile drops or reshapes and the kit substitutions it makes; scans
//! a payload for type errors before a save ([`scan_editor_payload_types`]) and lists the authored
//! gameplay data a document cannot carry ([`unsupported_authored_data`]); holds the compiler
//! identity an artifact records ([`COMPILER_PACKAGE_VERSION`]).
//! **Position:** mission tier 5, over `mission_model` (the compiled rows and authored blocks),
//! `mission_payload` (terrain bounds, kit aliases), `mission_validation` (the finding vocabulary),
//! `mission_wire_safety` and `orbat_slot_ids` (the slot ids a kit substitution names). The API
//! compiles on submit and preview; the Mission Creator compiles its export and shows the findings
//! in its validation panel.
//! **Signals & state:** none beyond the kit alias table `mission_payload` parses once; pure
//! functions.
//! **Invariants:** the compiled document keeps the authored faction, squad and slot order and
//! matches `mission.schema.json`; a compile never fails on authored data it can drop, it reports
//! a finding instead; the editor-input structs it parses stay private to the crate.

mod authoring;
mod compiler_identity;
mod error;
mod game_document;
pub mod prelude;

/// The compiler identity every compiled artifact records.
pub use compiler_identity::COMPILER_PACKAGE_VERSION;
/// Why a compile fails, and its result.
pub use error::{Error, Result};
/// The rule ids of the compile findings, all of them and one by one.
pub use game_document::{
    COMPILE_DIAGNOSTIC_RULE_IDS, DIAG_DROP_SLOT_CALLSIGN, DIAG_DROP_SLOT_RANK,
    DIAG_DROP_SLOT_STANCE, DIAG_DROP_SLOT_TAG, DIAG_DROP_SLOT_UNIT_NAME, DIAG_DROP_SQUAD_LEADER,
    DIAG_DROP_VEHICLE_ROSTER, DIAG_WIN_CONDITIONS,
};
/// The compile entry points: the typed document, the JSON forms and the authored environment.
pub use game_document::{
    CompiledOutput, apply_authored_environment, flatten_mod_document_json,
    flatten_mod_document_json_full, flatten_mod_document_json_with_diagnostics,
    flatten_mod_document_json_with_substitutions, flatten_to_mod_document,
};
/// The flow defaults a mission that authors no flow compiles to.
pub use game_document::{
    FLOW_DEFAULT_BRIEFING_S, FLOW_DEFAULT_JIP, FLOW_DEFAULT_SAFESTART_S, FLOW_DEFAULT_TIMELIMIT_S,
};
/// The kit substitutions a compile made, one by one and as a report.
pub use game_document::{KitSubstitution, KitSubstitutionReport};
/// The mission metadata a compile reads, the compiled document, and the terrain key it derives.
pub use game_document::{MissionMeta, ModMissionDocument, mission_terrain_key};
/// The save-time type scan and the authored data a document cannot carry.
pub use game_document::{scan_editor_payload_types, unsupported_authored_data};
