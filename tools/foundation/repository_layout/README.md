# Repository layout

The `repository_layout` crate: the repository locations more than one tool names, spelled once
each. A tool joins these relative paths onto the checkout root that
[`repository_root`](/crates/foundation/repository_root/README.md) finds; a location only one tool
names stays in that tool's own layout module
(`tools/map_assets/map_raster_pipeline/src/decision_record_locations.rs`,
`tools/enfusion/enfusion_script_index/src/script_index_layout.rs`,
`tools/browser_testing/browser_gate_suites/src/gate_layout.rs`).

## Contents

```text
tools/foundation/repository_layout/
├── Cargo.toml  the `repository_layout` library package: `repository_root` (the finder its prelude re-exports), layout tier 1
└── src/        the legacy ticket data, artifact, documentation, reference-lane, build-output, contract, map-asset, npm package, browser gate pin, workspace folder and Enfusion mod addon folder locations
```

## How it works

```text
prelude::find_repository_root()?  ──► root ──► root.join(ARTIFACTS_DIR | REFERENCES_DIR | …)
(re-exported from repository_root)          └──► contracts_dir(&root), terrain_dir(&root, "everon"), …
```

Every location is a constant relative to the checkout root, and the contract, map-asset and npm
package modules add functions that join one onto a root the caller passes. The crate's own code
walks no folder and reads no file: the caller finds the root with `repository_root`'s finder (from
its working directory, or from its own `CARGO_MANIFEST_DIR` when it must read the checkout it was
compiled from). The prelude re-exports the finder's names, so a tool binary, which depends only on
tool crates, reaches the checkout root through this crate.
`src/README.md` describes each module.

## Getting started

Run from the repository root:

```bash
cargo test -p repository_layout   # the shared locations by shape, and the committed ones in this checkout
```

## Configuration

No feature and no environment variable; the crate holds constants and pure path joins.

## Public surface

- At the crate root: the legacy ticket data folder `LEGACY_TICKETS_DIR` (`.ai/tickets`, records
  awaiting import into the central ticket manager, which the link check and the relocation tool
  leave alone); the artifact tree (`ARTIFACTS_DIR`, `WORKTREES_DIR`,
  `LAST_VERIFIED_MARKER`, `VERDICTS_DIR`); the reference lanes (`REFERENCES_DIR`,
  `CRF_FRAMEWORK_REFERENCE`, `VANILLA_REFERENCE`) and the vanilla lane's folders
  (`VANILLA_EXTRACTED_SCRIPTS`, `VANILLA_SCRIPT_API_PAGES`, `VANILLA_SOURCE_PAGES`,
  `VANILLA_RECONSTRUCTED_SOURCE`); `BUILD_OUTPUT_FOLDER`; `BROWSER_GATE_ENVIRONMENT`.
- At the crate root, each with a function joining it onto a given root: the contract tree
  (`CONTRACTS_DIR`, `contracts_dir`, `contract_definitions_dir`, `definition_path`,
  `contract_rules_dir`, `prefab_classify_path`, `kit_aliases_path`, `contract_catalogs_dir`,
  `registry_items_catalog_path`, `registry_compat_catalog_path`, `contract_fixtures_dir`,
  `mission_fixtures_valid_dir`, `mission_fixtures_invalid_dir`, `map_fixtures_dir`,
  `registry_fixtures_dir`, `enfusion_sample_fixtures_dir`, `bridge_sample_fixtures_dir`); the map
  assets (`TERRAIN_ASSETS_DIR`, `GLYPH_ASSETS_DIR`, `MAP_SCRATCH_DIR`, `terrain_assets_dir`,
  `terrain_dir`, `terrain_registry_path`, `terrain_manifest_path`, `glyph_assets_dir`,
  `glyph_manifest_path`, `map_scratch_dir`); the npm package (`ENFUSION_MCP_NODE_PACKAGE_DIR`,
  `ENFUSION_MCP_ENTRYPOINT`, `enfusion_mcp_node_package_dir`, `enfusion_mcp_entrypoint`).
- At the crate root, the locations the commands read: the deployment files (`DEPLOY_DIR`,
  `DEPLOY_ENV`, `DEPLOY_ENV_EXAMPLE`, `CADDYFILE`, `DEVELOPMENT_COMPOSE_FILE`, `SYSTEMD_UNITS_DIR`,
  `WEBSITE_API_UNIT`); the tool inputs (`DEDICATED_SERVER_PROFILES_DIR`, `DEV_SERVER_PROFILE`,
  `MCP_TRANSCRIPT_FIXTURES_DIR`); the PlayableSelector lane (`PLAYABLE_SELECTOR_REFERENCE`,
  `PLAYABLE_SELECTOR_OVERRIDE_ENV`); and the documents and documentation areas (`FACTORY_PACK_WAVE`,
  `HOME_SERVER_RUNBOOK`, `STAGING_SERVER_RUNBOOK`, `SLICE_WORKFLOW_RUNBOOK`,
  `PLATFORM_FACTORY_RUNBOOK`, `MOD_DESIGN`, `SPAWN_DETERMINISM_RUNBOOK`,
  `ARCHIVE_DIR`, `TICKET_DOCUMENTS_DIR`,
  `PENDING_MERGE_DIR`, `CURSOR_RULE_DIRS`, `PROJECT_INSTRUCTIONS`, `RETIRED_TOP_LEVEL_FOLDERS` with
  `is_retired_top_level_folder`, `HISTORICAL_PATH_SPELLINGS`, `PERMALINK_BASE`).
- `build_output`: `BUILD_OUTPUT_FOLDER`, the purpose subfolder names (`DEV_API_SUBFOLDER`, the
  `GATE_*` folders and prefixes, `CONTINUOUS_INTEGRATION_SUBFOLDER`, `MCP_DAEMON_SUBFOLDER`,
  `DATABASE_SELFTEST_SUBFOLDER`, `RUN_TARGET_SUBDIR`, `PURPOSE_SUBFOLDERS`),
  `build_output_subfolder`, and the retired root-level names with
  `is_retired_root_level_build_folder`.
- `documentation`: `DOCUMENTATION_ROOT`.
- `workspace_folders`: the workspace's top-level folders (`ENFUSION_MOD_DIR`, `LIBRARY_CRATES_DIR`,
  `TOOLS_DIR`), the API server crate and the `.env` its binaries read (`API_SERVER_CRATE_DIR`,
  `API_SERVER_ENVIRONMENT_FILE`), and the API database crate with its SQL folders
  (`API_DATABASE_CRATE_DIR`, `API_DATABASE_MIGRATIONS_DIR`, `API_DATABASE_SEEDS_DIR`), which the
  `db` commands and the staging, mod and fixture tools read.
- `enfusion_mod_folders`: each Enfusion mod addon once, as its folder name
  (`FRAMEWORK_ADDON_FOLDER_NAME`, `EXPORT_ADDON_FOLDER_NAME`, `MCP_BRIDGE_ADDON_FOLDER_NAME`), which
  is also the name a dedicated server's `addons/` folder links it under, and as its folder under
  `ENFUSION_MOD_DIR` (`FRAMEWORK_ADDON_DIR`, `EXPORT_ADDON_DIR`, `MCP_BRIDGE_ADDON_DIR`); read by the
  mod commands, the deploys, the workstation setup and the schema checks' mission validation.
- `prelude`: the top-level trees (`ARTIFACTS_DIR`, `CONTRACTS_DIR`, `DEPLOY_DIR`,
  `REFERENCES_DIR`, `TERRAIN_ASSETS_DIR`, `BUILD_OUTPUT_FOLDER`) and the checkout-root finder's
  names re-exported from `repository_root` (`find_repository_root`, `find_repository_root_from`,
  `is_repository_root`, `ROOT_MARKER`).

## Boundaries

- Depends on: `repository_root`, whose finder the prelude re-exports and whose root the tests
  check the committed locations against.
- Used by: the check and command crates, for every location listed above; `deploy_settings` (the settings file and its example); `xtask`,
  for the checkout root through the prelude.
- Rules: tier 1 of `tools/foundation`, over `repository_root` alone (`cargo xtask verify
  crate-tiers`); the root walk is `repository_root`'s alone, and the tool binaries reach it only
  through this crate.

## Related documentation

- [Tooling foundation crates](/tools/foundation/README.md) — the foundation crates and their
  tiers.
- [Tooling architecture](/documentation/tools/tooling_architecture.md) — how the tooling crates
  fit together.
