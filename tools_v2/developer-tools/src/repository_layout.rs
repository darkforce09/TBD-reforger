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

/* ─────────────────────────── enfusion-mcp node package ─────────────────────────── */

/// Directory holding the npm manifest, lockfile and node version that pin the `enfusion-mcp`
/// server this repository runs. It sits outside every crate root so that the installed
/// dependency tree beside it is never walked by a crate-scoped file scan.
pub const ENFUSION_MCP_NODE_PACKAGE_DIR: &str = "tools_v2/enfusion_mcp_node_package";

/// The `enfusion-mcp` server module installed by `npm ci` in that package directory. This is the
/// one spelling of that path in the workspace: `enfusion_tooling::enfusion_mcp_entrypoint`
/// resolves every caller's runner command from it, and derives from it the process pattern that
/// identifies a running server.
pub const ENFUSION_MCP_ENTRYPOINT: &str =
    "tools_v2/enfusion_mcp_node_package/node_modules/enfusion-mcp/dist/index.js";

/// Absolute path of the pinned `enfusion-mcp` server module inside a checkout.
pub fn enfusion_mcp_entrypoint(root: &Path) -> PathBuf {
    root.join(ENFUSION_MCP_ENTRYPOINT)
}

/// Absolute path of the npm package directory inside a checkout. `cargo xtask mod dev-bootstrap`
/// runs `npm ci` here.
pub fn enfusion_mcp_node_package_dir(root: &Path) -> PathBuf {
    root.join(ENFUSION_MCP_NODE_PACKAGE_DIR)
}

/* ─────────────────────────────── map assets (scratch) ─────────────────────────────── */

/// Local, uncommitted export scratch for one island: stitched orthophotos, masks, spikes.
///
/// Ignored by git. Pipeline stages write intermediates here; nothing downstream of an export may
/// read from it, because a fresh clone does not have it.
pub fn map_scratch_dir(root: &Path, terrain: &str) -> PathBuf {
    root.join("assets_v2/scratch").join(terrain)
}

/* ─────────────────────────────── checkout root ─────────────────────────────── */

/// The file whose presence marks a checkout root. [`crate::repository_paths::find_repo_root`]
/// stops its upward walk here, so a worktree nested under another checkout resolves to itself.
/// `ticket_engine::repository::ROOT_MARKER` is the same path spelled in the other foundational
/// crate, for the reason that module header gives.
pub const ROOT_MARKER: &str = ".ai/tickets/ROOT";

/* ─────────────────────────────── enfusion source index ─────────────────────────────── */

/// The symbol index built over Enfusion sources: one table per lane, read by every citation,
/// lookup and capability query. Pipeline output, not a committed input.
pub const ENF_INDEX_DIR: &str = ".ai/artifacts/enf-index";

/// The upstream-framework symbol table inside [`ENF_INDEX_DIR`], which the lookup, directory
/// census and capability join all read by default.
pub const CRF_SYMBOL_TABLE: &str = ".ai/artifacts/enf-index/crf_symbols.tsv";

/* ─────────────────────────────── export operation logs ─────────────────────────────── */

/// Where the world-export pipeline writes its per-terrain operation log and type inventory, and
/// where its verifiers read them back.
pub const OPERATIONS_LOG_DIR: &str = ".ai/artifacts";

/// One terrain's export operation log: every stage that ran, with what it produced.
pub fn export_operations_log(root: &Path, terrain: &str) -> PathBuf {
    root.join(OPERATIONS_LOG_DIR)
        .join(format!("map_export_{terrain}.json"))
}

/// One terrain's object type inventory: every world-object type the export saw, and its census
/// status.
pub fn object_type_inventory(root: &Path, terrain: &str) -> PathBuf {
    root.join(OPERATIONS_LOG_DIR)
        .join(format!("type_inventory_{terrain}.json"))
}

/* ─────────────────────────────── analysis artifacts ─────────────────────────────── */

/// Committed decision records for the inland-water classifier: the water and source spikes the
/// classifier writes and the refine spike it compares the current run against.
pub const INLAND_WATER_ARTIFACTS_DIR: &str = ".ai/artifacts/inland_water";

/// Committed decision records for the aerial orthophoto lane: the seam analysis the stitcher's
/// seam verifier writes.
pub const AERIAL_ORTHOPHOTO_ARTIFACTS_DIR: &str = ".ai/artifacts/aerial_orthophoto";

/// Committed decision records for the cartographic lane: the land-cover source spike the mask
/// builder cites in the ortho metadata it emits.
pub const CARTOGRAPHIC_RENDERING_ARTIFACTS_DIR: &str = ".ai/artifacts/cartographic_rendering";

/// [`INLAND_WATER_ARTIFACTS_DIR`] under a checkout root.
pub fn inland_water_artifacts_dir(root: &Path) -> PathBuf {
    root.join(INLAND_WATER_ARTIFACTS_DIR)
}

/// [`AERIAL_ORTHOPHOTO_ARTIFACTS_DIR`] under a checkout root.
pub fn aerial_orthophoto_artifacts_dir(root: &Path) -> PathBuf {
    root.join(AERIAL_ORTHOPHOTO_ARTIFACTS_DIR)
}

/// [`CARTOGRAPHIC_RENDERING_ARTIFACTS_DIR`] under a checkout root.
pub fn cartographic_rendering_artifacts_dir(root: &Path) -> PathBuf {
    root.join(CARTOGRAPHIC_RENDERING_ARTIFACTS_DIR)
}

/// Documents the tools read or name in what they print.
///
/// Relocating the documentation tree rewrites this module and nothing else in the crate.
pub mod documentation {
    /// Game-mod documentation. `enf citations` walks it and resolves every `@idx lane#Symbol`
    /// citation in it against the symbol index.
    pub const MOD_DOCS_DIR: &str = "docs/mod";

    /// The hand-authored verdict table `enf capability` joins the upstream symbol index against,
    /// so a framework file nobody has triaged is a build error rather than an oversight.
    pub const CAPABILITY_VERDICTS: &str = "docs/mod/capability_verdicts.tsv";

    /// Known wedge modes of the headless editor gate, and the recipe for each — named by the
    /// font-cache diagnostic when it cannot explain what it found.
    pub const EDITOR_GATE_RUNBOOK: &str = "docs/website/EDITOR_GATE_RUNBOOK.md";
}

#[cfg(test)]
#[path = "tests/repository_layout.rs"]
mod tests;
