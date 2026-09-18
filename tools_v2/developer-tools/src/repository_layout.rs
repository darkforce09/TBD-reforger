//! Canonical locations of the wire-contract and map-asset trees inside a checkout.
//!
//! Every tool that reads a schema, a golden fixture, a terrain dataset or a glyph atlas resolves
//! its path through this module rather than spelling a directory literal. The trees are data, not
//! code: they are addressed by dozens of call sites across two crates, and a literal repeated that
//! widely cannot be moved without a survey. One expression per location makes the location a fact
//! the build checks instead of a string the reader has to trust.
//!
//! Every function takes the checkout root explicitly. Callers that do not already hold one get it
//! from [`crate::repository_paths::find_repo_root`], which walks up from the cwd — the running
//! worktree, never the worktree a shared-target binary happened to be compiled in.

use std::path::{Path, PathBuf};

/* ─────────────────────────────── wire contracts ─────────────────────────────── */

/// Root of the wire-contract tree: schema definitions, classification rules, live catalogs, and
/// the golden fixtures every boundary is tested against.
pub fn contracts_dir(root: &Path) -> PathBuf {
    root.join("contracts_v2")
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

/* ─────────────────────────────── golden fixtures ─────────────────────────────── */

/// Root of the committed fixture corpus.
pub fn contract_fixtures_dir(root: &Path) -> PathBuf {
    contracts_dir(root).join("fixtures")
}

/// Playable missions that must always parse, validate and compile.
pub fn mission_fixtures_valid_dir(root: &Path) -> PathBuf {
    contract_fixtures_dir(root).join("missions/valid")
}

/// Deliberately malformed scenarios, each pinning one rejection gate.
pub fn mission_fixtures_invalid_dir(root: &Path) -> PathBuf {
    contract_fixtures_dir(root).join("missions/invalid")
}

/// Spatial fixtures: object chunks, road networks, region derivations, terrain manifests.
pub fn map_fixtures_dir(root: &Path) -> PathBuf {
    contract_fixtures_dir(root).join("map")
}

/// Committed forest-density fixtures, read when re-densifying without a Workbench export.
pub fn density_fixtures_dir(root: &Path) -> PathBuf {
    map_fixtures_dir(root).join("density")
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

/* ─────────────────────────────── map assets ─────────────────────────────── */

/// Root of the built-in terrain datasets, served at `/map-assets`.
pub fn terrain_assets_dir(root: &Path) -> PathBuf {
    root.join("assets_v2/terrains")
}

/// One island's dataset directory.
pub fn terrain_dir(root: &Path, terrain: &str) -> PathBuf {
    terrain_assets_dir(root).join(terrain)
}

/// Catalog of every registered platform terrain.
pub fn terrain_registry_path(root: &Path) -> PathBuf {
    terrain_assets_dir(root).join("terrain-registry.json")
}

/// One island's manifest: world bounds, DEM scaling, tile and object paths.
pub fn terrain_manifest_path(root: &Path, terrain: &str) -> PathBuf {
    terrain_dir(root, terrain).join("manifest.json")
}

/// Tactical symbology atlases and marker sources, served at `/map-assets/glyphs`.
pub fn glyph_assets_dir(root: &Path) -> PathBuf {
    root.join("assets_v2/glyphs")
}

/// The glyph registry and its UV coordinate catalog.
pub fn glyph_manifest_path(root: &Path) -> PathBuf {
    glyph_assets_dir(root).join("manifest.json")
}

/// The pair of directories the map client reaches under a single `/map-assets` URL prefix.
///
/// Terrains and glyphs are separate on disk because glyphs are shared by every terrain, and they
/// are joined under one prefix by whatever is serving them. Carrying them as one value keeps a
/// caller from wiring the terrain mount and forgetting the glyph mount, which does not fail at
/// startup: the map renders, and every icon is missing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapAssetMounts {
    pub terrains: PathBuf,
    pub glyphs: PathBuf,
}

impl MapAssetMounts {
    /// Both directories as they sit in a checkout.
    pub fn from_root(root: &Path) -> Self {
        Self {
            terrains: terrain_assets_dir(root),
            glyphs: glyph_assets_dir(root),
        }
    }

    /// The pair implied by a terrain directory, wherever it has been placed.
    ///
    /// Glyphs are the terrain tree's sibling, so a deployment that relocates one relocates both.
    /// A terrain path with no parent yields a bare `glyphs`, which is the correct relative answer.
    pub fn beside_terrains(terrains: PathBuf) -> Self {
        let glyphs = terrains
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join("glyphs");
        Self { terrains, glyphs }
    }
}

/// Local, uncommitted export scratch for one island: stitched orthophotos, masks, spikes.
///
/// Ignored by git. Pipeline stages write intermediates here; nothing downstream of an export may
/// read from it, because a fresh clone does not have it.
pub fn map_scratch_dir(root: &Path, terrain: &str) -> PathBuf {
    root.join("assets_v2/scratch").join(terrain)
}

#[cfg(test)]
#[path = "tests/repository_layout.rs"]
mod tests;
