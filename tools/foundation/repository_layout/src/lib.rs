//! The repository locations the tools share.
//!
//! **Role:** the location modules name each location more than one tool reads, once, as a relative
//! path a caller joins onto the checkout root `repository_root` finds (the folder's README lists
//! every module's locations).
//! **Position:** tier 1 of `tools/foundation`, over the checkout-root finder `repository_root`,
//! whose names the [`prelude`] re-exports so the tool binaries reach the root through this crate.
//! The ticket crates, the command crates, `xtask` and `ticketboard_desktop` spell these shared
//! locations through it; a location only one tool names stays in that tool's own layout module.
//! **Signals & state:** none; constants and pure path joins.
//! **Invariants:** every location is relative and uses `/` separators; the crate itself reads no
//! file and walks no folder (the root walk is `repository_root`'s alone).
mod agent_artifacts;
mod browser_gate_environment;
pub mod build_output;
mod contracts;
mod deployment;
pub mod documentation;
mod documentation_locations;
mod enfusion_mcp_node_package;
mod map_assets;
pub mod prelude;
mod ticket_registry;
pub mod tool_inputs;
mod upstream_references;
mod vanilla_reference_lanes;
pub mod workspace_folders;

#[cfg(test)]
#[path = "tests/shared_locations_tests.rs"]
mod shared_locations_tests;

pub use self::deployment::{
    CADDYFILE, DEPLOY_DIR, DEPLOY_ENV, DEPLOY_ENV_EXAMPLE, DEVELOPMENT_COMPOSE_FILE,
    SYSTEMD_UNITS_DIR, WEBSITE_API_UNIT,
};
pub use self::ticket_registry::{
    CORPUS_PINS, ESTIMATES_DIR, ESTIMATES_SCHEMA, METRICS_DIR, METRICS_SCHEMA, QUEUE_JSON, SCHEMA,
    SCOPE_VOCAB, TICKETS_DIR, WAVE_LOCK,
};
pub use agent_artifacts::{ARTIFACTS_DIR, LAST_VERIFIED_MARKER, VERDICTS_DIR, WORKTREES_DIR};
pub use browser_gate_environment::BROWSER_GATE_ENVIRONMENT;
pub use build_output::BUILD_OUTPUT_FOLDER;
pub use contracts::{
    CONTRACTS_DIR, bridge_sample_fixtures_dir, contract_catalogs_dir, contract_definitions_dir,
    contract_fixtures_dir, contract_rules_dir, contracts_dir, definition_path,
    enfusion_sample_fixtures_dir, kit_aliases_path, map_fixtures_dir, mission_fixtures_invalid_dir,
    mission_fixtures_valid_dir, prefab_classify_path, registry_compat_catalog_path,
    registry_fixtures_dir, registry_items_catalog_path,
};
pub use documentation_locations::{
    API_READINESS_EVIDENCE_PREFIX, API_READINESS_REGISTER, ARCHIVE_DIR, CURSOR_RULE_DIRS,
    FACTORY_PACK_WAVE, HISTORICAL_PATH_SPELLINGS, HOME_SERVER_RUNBOOK, MOD_DESIGN,
    PENDING_MERGE_DIR, PERMALINK_BASE, PLATFORM_FACTORY_RUNBOOK, PROJECT_INSTRUCTIONS,
    RETIRED_DOCS_ROOT, SLICE_WORKFLOW_RUNBOOK, SPAWN_DETERMINISM_RUNBOOK, STAGING_SERVER_RUNBOOK,
    TICKET_DOCUMENTS_DIR,
};
pub use enfusion_mcp_node_package::{
    ENFUSION_MCP_ENTRYPOINT, ENFUSION_MCP_NODE_PACKAGE_DIR, enfusion_mcp_entrypoint,
    enfusion_mcp_node_package_dir,
};
pub use map_assets::{
    GLYPH_ASSETS_DIR, MAP_SCRATCH_DIR, TERRAIN_ASSETS_DIR, glyph_assets_dir, glyph_manifest_path,
    map_scratch_dir, terrain_assets_dir, terrain_dir, terrain_manifest_path, terrain_registry_path,
};
pub use tool_inputs::{
    DEDICATED_SERVER_PROFILES_DIR, DEV_SERVER_PROFILE, MCP_TRANSCRIPT_FIXTURES_DIR,
};
pub use upstream_references::{
    CRF_FRAMEWORK_REFERENCE, PLAYABLE_SELECTOR_OVERRIDE_ENV, PLAYABLE_SELECTOR_REFERENCE,
    REFERENCES_DIR, VANILLA_REFERENCE,
};
pub use vanilla_reference_lanes::{
    VANILLA_EXTRACTED_SCRIPTS, VANILLA_RECONSTRUCTED_SOURCE, VANILLA_SCRIPT_API_PAGES,
    VANILLA_SOURCE_PAGES,
};
