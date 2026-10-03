# Repository layout

The `repository_layout` crate: the one walk that finds a checkout root, and the repository
locations more than one tool names, spelled once each. A tool joins these relative paths onto the
root the walk found; a location only one tool names stays in that tool's own layout module
(`tools/developer_tools/src/map_pipeline_layout.rs`,
`tools/enfusion/enfusion_script_index/src/script_index_layout.rs`,
`tools/browser_testing/browser_gate_suites/src/gate_layout.rs`,
`tools/tickets/ticket_model/src/repository.rs`).

## Contents

```text
tools/foundation/repository_layout/
├── Cargo.toml  the `repository_layout` library package: `thiserror` only, layout tier 0
└── src/        the root walk, its error, and the ticket registry, artifact, documentation, reference-lane, build-output, contract, map-asset, npm package and browser gate pin locations
```

## How it works

```text
find_repository_root()            current directory ──┐
find_repository_root_from(start)  any folder ─────────┴─► walk up to the nearest folder holding .ai/tickets/ROOT
                                                            ├─ Ok(root)   ──► root.join(TICKETS_DIR | ARTIFACTS_DIR | …)
                                                            └─ Err(RootMarkerNotFound { start })
```

The walk stops at the nearest marker, so a slice worktree nested under another checkout resolves
to itself. A command answers from its working directory; a tool that must read the checkout it
was compiled from walks from its own `CARGO_MANIFEST_DIR`. `src/README.md` describes each module.

## Getting started

Run from the repository root:

```bash
cargo test -p repository_layout   # the walk over scratch checkouts, and the shared locations in this checkout
```

## Configuration

No feature and no environment variable; the walk reads the working directory and the
filesystem only.

## Public surface

- At the crate root: `find_repository_root`, `find_repository_root_from`, `is_repository_root`,
  `ROOT_MARKER`, `Error` and `Result`; the ticket registry paths (`TICKETS_DIR`, `SCHEMA`,
  `SCOPE_VOCAB`, `CORPUS_PINS`, `WAVE_LOCK`, `QUEUE_JSON`, `METRICS_DIR`, `METRICS_SCHEMA`,
  `ESTIMATES_DIR`, `ESTIMATES_SCHEMA`); the artifact tree (`ARTIFACTS_DIR`, `WORKTREES_DIR`,
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
  `API_READINESS_EVIDENCE_PREFIX`, `API_READINESS_REGISTER`, `ARCHIVE_DIR`, `TICKET_DOCUMENTS_DIR`,
  `PENDING_MERGE_DIR`, `CURSOR_RULE_DIRS`, `PROJECT_INSTRUCTIONS`, `RETIRED_DOCS_ROOT`,
  `HISTORICAL_PATH_SPELLINGS`, `PERMALINK_BASE`).
- `build_output`: `BUILD_OUTPUT_FOLDER`, the purpose subfolder names (`DEV_API_SUBFOLDER`, the
  `GATE_*` folders and prefixes, `CONTINUOUS_INTEGRATION_SUBFOLDER`, `MCP_DAEMON_SUBFOLDER`,
  `DATABASE_SELFTEST_SUBFOLDER`, `RUN_TARGET_SUBDIR`, `PURPOSE_SUBFOLDERS`),
  `build_output_subfolder`, and the retired root-level names with
  `is_retired_root_level_build_folder`.
- `documentation`: `DOCUMENTATION_ROOT`, `ROADMAP` and `GAP_ANALYSIS`.
- `prelude`: the walk, the probe and `ROOT_MARKER`.

## Boundaries

- Depends on: `thiserror` only.
- Used by: the ticket crates in `tools/tickets/`, `developer_tools`, `xtask` and `ticketboard`, for every checkout-root
  lookup and every location listed above; `deploy_settings` (the settings file and its example)
  and `tool_test_support` (the test checkout root).
- Rules: tier 0 of `tools/foundation` with no workspace dependency (`cargo xtask verify
  crate-tiers`; `foundation_crates_depend_only_on_lower_foundation_crates` in
  `tools/checks/repository_checks/src/tests/tooling_dependency_boundaries.rs`); no other tool defines a root walk.

## Related documentation

- [Tooling foundation crates](/tools/foundation/README.md) — the foundation crates and their
  tiers.
- [Tooling architecture](/documentation/tools/tooling_architecture.md) — how the tooling crates
  fit together.
