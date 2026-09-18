//! Business-logic services shared across domains.

pub mod mission_compile;
pub mod registry_import;
pub mod user_stats;

pub use mission_compile::{
    CompileError, ModMissionDocument, ModSlot, flatten_to_mod_document,
    flatten_to_mod_document_with_catalog, mission_terrain_key,
};
// T-690 — the compile's structured diagnostics, and the two `/compiled` response headers that carry
// them ALONGSIDE the body (the body itself is unchanged: `mission.schema.json` closes the document
// root, so a findings key in the JSON would 500 the route for every mission).
pub use mission_compile::{
    COMPILE_DIAGNOSTICS_COUNT_HEADER, COMPILE_DIAGNOSTICS_RULES_HEADER, CompileFinding,
    compile_diagnostics_rules_header,
};
pub use registry_import::{ImportCounts, ImportError, ensure_modpack, import_compat, import_items};
pub use website_map_engine::data::scenario::orbat::{
    OrbatSlotTemplate, OrbatSquadTemplate, parse_orbat_template,
};
// T-336 — `users.total_deployments` / `attendance_rate` have exactly one writer and two callers.
// That makes it a service; it used to be `pub(super)` inside `handlers/telemetry.rs`.
pub use user_stats::{
    recompute_user_stats, recompute_user_stats_best_effort, refresh_leaderboard_best_effort,
};
