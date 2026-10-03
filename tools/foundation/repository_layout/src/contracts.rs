//! The wire-contract tree.
//!
//! **Role:** the schema definitions, classification rules, live Workbench catalogs and golden
//! fixtures under [`CONTRACTS_DIR`], each as a function that joins its location onto a checkout
//! root the caller passes.
//! **Position:** `xtask` validates schemas, catalogs and fixtures here and generates code from the
//! definitions; `developer_tools` validates what the map pipelines and the blueprint compiler
//! emit against the definitions and classifies prefabs by the rules. A contract location only one
//! tool names stays in that tool's own layout module.
//! **Signals & state:** none; pure path joins.
//! **Invariants:** every location lies under `<root>/`[`CONTRACTS_DIR`]; nothing here touches the
//! filesystem.

use std::path::{Path, PathBuf};

/// Root of the wire-contract tree, relative to a checkout root: schema definitions,
/// classification rules, live catalogs, and the golden fixtures every boundary is tested against.
pub const CONTRACTS_DIR: &str = "contracts";

/// [`CONTRACTS_DIR`] under a checkout root.
pub fn contracts_dir(root: &Path) -> PathBuf {
    root.join(CONTRACTS_DIR)
}

/// Authoritative JSON Schema definitions. The codegen pipeline's input directory.
pub fn contract_definitions_dir(root: &Path) -> PathBuf {
    contracts_dir(root).join("definitions")
}

/// One schema definition by file name, e.g. `mission.schema.json`.
pub fn definition_path(root: &Path, file_name: &str) -> PathBuf {
    contract_definitions_dir(root).join(file_name)
}

/// Deterministic classification and mapping tables applied to Enfusion assets.
pub fn contract_rules_dir(root: &Path) -> PathBuf {
    contracts_dir(root).join("rules")
}

/// Maps Enfusion `{GUID}` prefab paths to functional categories and density tiers.
pub fn prefab_classify_path(root: &Path) -> PathBuf {
    contract_rules_dir(root).join("prefab-classify.json")
}

/// Approved mission kit aliases, keyed by Enfusion resource name.
pub fn kit_aliases_path(root: &Path) -> PathBuf {
    contract_rules_dir(root).join("kit-aliases.json")
}

/// Live Workbench exports: production data the platform ingests, not test fixtures.
pub fn contract_catalogs_dir(root: &Path) -> PathBuf {
    contracts_dir(root).join("catalogs")
}

/// Flat item catalog exported from Workbench; drives the Virtual Arsenal.
pub fn registry_items_catalog_path(root: &Path) -> PathBuf {
    contract_catalogs_dir(root).join("registry-items.workbench.json")
}

/// Directed weapon-to-attachment compatibility graph exported from Workbench.
pub fn registry_compat_catalog_path(root: &Path) -> PathBuf {
    contract_catalogs_dir(root).join("registry-compat.workbench.json")
}

/// Root of the committed fixture corpus.
pub fn contract_fixtures_dir(root: &Path) -> PathBuf {
    contracts_dir(root).join("fixtures")
}

/// Playable missions that must always parse, validate and compile.
pub fn mission_fixtures_valid_dir(root: &Path) -> PathBuf {
    contract_fixtures_dir(root).join("missions/valid")
}

/// Deliberately malformed missions, each pinning one rejection gate.
pub fn mission_fixtures_invalid_dir(root: &Path) -> PathBuf {
    contract_fixtures_dir(root).join("missions/invalid")
}

/// Spatial fixtures: object chunks, road networks, region derivations, terrain manifests.
pub fn map_fixtures_dir(root: &Path) -> PathBuf {
    contract_fixtures_dir(root).join("map")
}

/// Item, loadout, faction and alias samples used by round-trip and validation tests.
pub fn registry_fixtures_dir(root: &Path) -> PathBuf {
    contract_fixtures_dir(root).join("registry")
}

/// Raw JSON payload samples as the game mod emits them.
pub fn enfusion_sample_fixtures_dir(root: &Path) -> PathBuf {
    contract_fixtures_dir(root).join("enfusion_samples")
}

/// Canonical voice-bridge IPC message samples.
pub fn bridge_sample_fixtures_dir(root: &Path) -> PathBuf {
    contract_fixtures_dir(root).join("bridge_samples")
}

#[cfg(test)]
#[path = "tests/contracts_tests.rs"]
mod tests;
