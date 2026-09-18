# Legacy Tooling Analysis & Inventory Catalog

Exhaustive forensic analysis of the existing tooling layout (`xtask/`, `tools/`, and `crates/`), capturing exact line counts, functional responsibilities, architectural violations, and target refactoring destinations.

---

## 1. Legacy Crate Overview

| Legacy Location | Target Location | Crate Name | Files | Total LOC | Purpose & Key Primitives |
|---|---|---|---|---|---|
| `crates/tbd-gate` | `tools_v2/verification-core` | `verification-core` | 8 | 2,260 | Fail-closed static assertion library (`Verdict`, `GateLock`, `Report`, `proc::Run`). |
| `crates/tbd-tickets` | `tools_v2/ticket-engine` | `ticket-engine` | 7 | 5,078 | Ticket schema, canonical TOML serialization, transactional mutation engine. |
| `xtask/` | `tools_v2/xtask` | `xtask` | 175 | ~75,000+ | Workspace task runner, ticket validation, markdown syncing, wave lockfile, 3D mesh CAD compiler. |
| `tools/tbd-tools` | `tools_v2/developer-tools` | `developer-tools` | 52 | ~24,000+ | Heavy async CLI suite: CDP browser harness, Enfusion oracle, map imagery pipeline, world export. |
| `tools/pbo` | *(Deleted)* | — | 1 | — | Orphaned Python bytecode cache (`derap.cpython-311.pyc`). |
| `tools/editor-capture` | *(Deleted)* | — | 1 | — | Obsolete Markdown README; capture logic already ported to Rust. |

---

## 2. Forensic Analysis of `gate_*.rs` and `gate_t*.rs` (xtask)

### 2.1 Ticket-Named Gates (`gate_t*.rs`)
Historical ticket numbers violate **Law 4 (Zero Context Needed)** and **Law 8 (Present-Tense Documentation)**. Below is the mapping of every `gate_t*.rs` to its live present-tense invariant:

| Current File | LOC | Ticket | Present-Tense Invariant Enforced | New Self-Describing Location |
|---|---|---|---|---|
| `gate_t180.rs` | 425 | T-180 | Proves ORBAT and entity mutations route strictly through the CRDT store; bans direct mutation calls in frontend/map-engine. | `tools_v2/xtask/src/verifications/architecture/editor_orbat_coherency.rs` |
| `gate_t296.rs` | 230 | T-296 | Enforces the `#tbd link` comment contract on `TBD_ResultsReporter.c` with perturbation proofs. | `tools_v2/xtask/src/verifications/mod_scripts/results_reporter_comments.rs` |
| `gate_t437.rs` | 338 | T-437 | Proves destroy-objective entity diagnostics handle missing target entities cleanly in `TBD_ObjectiveRegistry.c`. | `tools_v2/xtask/src/verifications/mod_scripts/destroy_target_diagnostics.rs` |
| `gate_t438.rs` | 195 | T-438 | Pin ensuring staging deploy commands target `docker-compose.staging.yml`. | `tools_v2/xtask/src/verifications/deployment/staging_compose_path.rs` |
| `gate_t439.rs` | 385 | T-439 | Audits parity between editor object palette aliases and Enfusion spawn registry entries. | `tools_v2/xtask/src/verifications/registry/object_alias_spawn_census.rs` |
| `gate_t440.rs` | 275 | T-440 | Verifies the US Army 1980s faction library SQL seed applies cleanly. | `tools_v2/xtask/src/verifications/database/faction_library_seed.rs` |
| `gate_t444.rs` | 240 | T-444 | Verifies tactical field manual wiki page SQL seeds apply cleanly. | `tools_v2/xtask/src/verifications/database/wiki_page_seed.rs` |
| `gate_t456.rs` | 210 | T-456 | Enforces HTTP body size limits on the game mission REST loader before JSON parsing. | `tools_v2/xtask/src/verifications/mod_scripts/mission_loader_rest_size.rs` |
| `gate_t468.rs` | 315 | T-468 | Pin ensuring all schema verification gates remain wired into CI tasks. | `tools_v2/xtask/src/verifications/ci/ci_schema_parity.rs` |

### 2.2 Operational Commands Misnamed as Gates
During the T-853 bash migration, operational CLI commands were prefixed with `gate_`. They belong in `commands/`:

| Current Misnamed File | LOC | Actual CLI Command | Target Domain Path |
|---|---|---|---|
| `gate_bootstrap_staging_server.rs` | 380 | `cargo xtask mod bootstrap-staging` | `tools_v2/xtask/src/commands/mod_ops/bootstrap_staging.rs` |
| `gate_debug_direct_join.rs` | 320 | `cargo xtask debug direct-join` | `tools_v2/xtask/src/commands/debug/direct_join.rs` |
| `gate_deploy_website.rs` | 420 | `cargo xtask deploy website` | `tools_v2/xtask/src/commands/deploy/website.rs` |
| `gate_export_terrain.rs` | 240 | `cargo xtask map export-terrain` | `tools_v2/xtask/src/commands/map/export_terrain.rs` |
| `gate_fetch_vanilla_api.rs` | 350 | `cargo xtask fetch vanilla-api` | `tools_v2/xtask/src/commands/fetch/vanilla_api.rs` |
| `gate_fetch_vanilla_source.rs`| 350 | `cargo xtask fetch vanilla-source` | `tools_v2/xtask/src/commands/fetch/vanilla_source.rs` |
| `gate_mcp_call.rs` | 330 | `cargo xtask mcp call` | `tools_v2/xtask/src/commands/mcp/call.rs` |
| `gate_mcp_smoke.rs` | 220 | `cargo xtask mcp smoke` | `tools_v2/xtask/src/commands/mcp/smoke.rs` |
| `gate_mcp_wb_logs.rs` | 460 | `cargo xtask mcp wb-logs` | `tools_v2/xtask/src/commands/mcp/wb_logs.rs` |
| `gate_mission_version_upload_repro.rs` | 160 | `cargo xtask repro mission-upload` | `tools_v2/xtask/src/commands/repro/mission_upload.rs` |
| `gate_mod_compile.rs` | 740 | `cargo xtask mod compile` | `tools_v2/xtask/src/commands/mod_ops/compile.rs` |
| `gate_remote_log_grep.rs` | 480 | `cargo xtask mod remote-logs` | `tools_v2/xtask/src/commands/mod_ops/remote_logs.rs` |
| `gate_run_dev_server.rs` | 130 | `cargo xtask mod dev-server` | `tools_v2/xtask/src/commands/mod_ops/dev_server.rs` |
| `gate_seed_milestone_announcement.rs` | 210 | `cargo xtask mod seed-announcement` | `tools_v2/xtask/src/commands/mod_ops/seed_announcement.rs` |
| `gate_setup_client_addons.rs` | 270 | `cargo xtask setup client-addons` | `tools_v2/xtask/src/commands/setup/client_addons.rs` |
| `gate_setup_mcp_game_root.rs` | 180 | `cargo xtask setup mcp-game-root` | `tools_v2/xtask/src/commands/setup/mcp_game_root.rs` |
| `gate_setup_server_profile.rs`| 290 | `cargo xtask setup server-profile` | `tools_v2/xtask/src/commands/setup/server_profile.rs` |
| `gate_setup_workbench_linux.rs`| 290 | `cargo xtask setup workbench-linux` | `tools_v2/xtask/src/commands/setup/workbench_linux.rs` |
| `gate_tbd_dev_bootstrap.rs` | 480 | `cargo xtask mod dev-bootstrap` | `tools_v2/xtask/src/commands/mod_ops/dev_bootstrap.rs` |
| `gate_test_mission.rs` | 260 | `cargo xtask mod test-mission` | `tools_v2/xtask/src/commands/mod_ops/test_mission.rs` |
| `gate_test_phase1_api.rs` | 250 | `cargo xtask mod test-phase1-api` | `tools_v2/xtask/src/commands/mod_ops/test_phase1_api.rs` |

### 2.3 Genuine Invariant Verifications (xtask)
These true verification checks belong in `tools_v2/xtask/src/verifications/`:

| File | LOC | Domain | Invariant Checked | Target Path |
|---|---|---|---|---|
| `gate_engine_layers.rs` | 580 | Architecture | Guarantees `graphics-engine` never imports `map-engine` and has zero map concepts (Law 6). | `verifications/architecture/engine_layer_boundaries.rs` |
| `gate_route_tags.rs` | 966 | API | Validates bidirectional parity between Axum handlers and `@route` doc tags. | `verifications/architecture/route_tags.rs` |
| `gate_crf_leak.rs` | 830 | Licensing | Guarantees zero Coalition Reforger Framework (CRF) symbols or GUIDs leak into `tbd-framework`. | `verifications/licensing/no_crf_oracle_leak.rs` |
| `shell_free.rs` | 460 | Language Bans | LANG-1: Hard-zero shell scripts in repository. | `verifications/language_bans/no_shell_scripts.rs` |
| `gate_no_python.rs` | 180 | Language Bans | LANG-2: Hard-zero Python in repository. | `verifications/language_bans/no_python_scripts.rs` |
| `node_free.rs` | 680 | Language Bans | LANG-3: Hard-zero Node/npm scripts; enforces file length pins. | `verifications/language_bans/no_node_scripts.rs` |
| `sql_gates.rs` | 165 | Database | Bans `SELECT *` on tables with nullable columns. | `verifications/database/no_select_star.rs` |
| `mod_comment_gates.rs` | 222 | Mod Scripts | Verifies `TBD_PlayerIdentity.c` comment contracts. | `verifications/mod_scripts/player_identity_comments.rs` |
| `gate_ui_layouts.rs` | 480 | Enfusion UI | Validates `.layout` syntax, slot types, and widget contracts. | `verifications/mod_scripts/enfusion_ui_layouts.rs` |
| `gate_tbd_spawn_determinism.rs` | 380 | Mod Scripts | Seeded entity spawn placement determinism. | `verifications/mod_scripts/spawn_determinism.rs` |
| `golden_gate.rs` | 990 | Map Assets | Map object semantics golden gates S2-S15. | `verifications/map_assets/map_object_golden.rs` |
| `label_gates.rs` | 975 | Map Assets | Height/town label declutter and DEM elevation validation. | `verifications/map_assets/terrain_labels.rs` |
| `verify_blas_manifest.rs` | 247 | Map Assets | Building BLAS BVH library manifest validation. | `verifications/map_assets/blas_manifest.rs` |
| `verify_ci_shell.rs` | 420 | CI | Bans inline bash logic in `.github/workflows/`. | `verifications/ci/ci_workflow_shell.rs` |

---

## 3. Analysis of Massive Files (>500 LOC)

### 3.1 `schema_gates.rs` (4,764 LOC, 203 KB)
A monolithic port of `contracts_v2/scripts/*.mjs`.

**Decomposition Plan (<500 LOC per module under `tools_v2/xtask/src/verifications/schemas/`):**
1. `contract_citations.rs` (~390 LOC): RFC-6901 JSON pointer validation.
2. `sentence_and_tile_budgets.rs` (~110 LOC): N6 sentence rules and N10 tile budget checks.
3. `map_object_enums.rs` (~150 LOC): Enum catalog schema validation.
4. `type_inventory.rs` (~390 LOC): Type inventory checks I1–I7.
5. `terrain_manifest.rs` (~280 LOC): Binary manifest chunk headers and boundaries.
6. `terrain_binary_blocks.rs` (~280 LOC): POD format validation & elevation block limits.
7. `spec_consistency.rs` (~410 LOC): 12 spec-to-model consistency gates.
8. `kit_alias_spawn_registry.rs` (~105 LOC): Kit alias ↔ spawn registry cross-referencing.
9. `schema_version_wire_fields.rs` (~480 LOC): Schema version wire fields tripwire.
10. `validator/compiler.rs` (~450 LOC) & `validator/rules.rs` (~450 LOC): JSON Schema validator.
11. `map_glyphs/manifest.rs` (~400 LOC) & `map_glyphs/rules.rs` (~380 LOC): NATO glyph catalog validation.

### 3.2 `check.rs` (2,330 LOC, 99 KB)
Enforces ticket database integrity. **1,066 lines of inline tests (`check.rs:1264–2330`)!**
- Relocated to `tools_v2/ticket-engine/src/validation/`.
- Tests extracted to `tools_v2/ticket-engine/tests/ticket_check_tests.rs`.
- Production logic split into: `schema.rs`, `scope.rs`, `body_rules.rs`, `gates.rs`, `debt.rs`, `hierarchy.rs`.

### 3.3 `cmds.rs` (2,214 LOC, 91 KB)
Ticket CLI commands. **871 lines of inline tests (`cmds.rs:1343–2214`)!**
- Relocated to `tools_v2/ticket-engine/src/cli/`.
- Tests extracted to `tools_v2/ticket-engine/tests/ticket_cmds_tests.rs`.
- Production logic split into: `query.rs`, `mutation.rs`, `shipping.rs`, `batch_run.rs`.

### 3.4 `wave_lock.rs` (2,170 LOC, 93 KB)
The wave lockfile compiler and repack/check tool. **1,027 lines of inline tests (`wave_lock.rs:1143–2170`)!**
- Relocated to `tools_v2/ticket-engine/src/wave_lock/`.
- Tests extracted to `tools_v2/ticket-engine/tests/wave_lock_tests.rs`.
- Production logic split into: `model.rs`, `packer.rs`, `verifier.rs`.

### 3.5 `smokes.rs` (4,071 LOC, 186 KB)
The headless Chrome DevTools Protocol editor smoke test suite.
- Relocated to `tools_v2/developer-tools/src/browser_testing/editor_smoke_tests/`.
- Split into: `harness.rs`, `runner.rs`, `core_tests.rs`, `canvas_tests.rs`, `dock_widget_tests.rs`, `mutation_tests.rs`.

### 3.6 `aux.rs` (1,644 LOC, 66 KB)
Catch-all junk drawer in `tools-tools`.
- Relocated to `tools_v2/developer-tools/src/world_export_pipeline/`.
- Split into 5 focused modules: `dem_elevation_import.rs`, `export_validation.rs`, `object_census.rs`, `export_profile_staging.rs`, `export_spike_verification.rs`.

---

## 4. Analysis of `xtask/src/map_blueprint/` (13,409 LOC, 32 Files)

| File | LOC | Purpose | Target Path in `developer-tools` |
|---|---|---|---|
| `xob.rs` | 705 | Binary reader for Enfusion `.xob` 3D models | `src/blueprint/mesh_reader.rs` |
| `xob_nodes.rs` | 541 | Scene node hierarchy table parser | `src/blueprint/mesh_nodes.rs` |
| `mesh.rs` | 904 | Mesh-based voxel extractor (raymarching) | `src/blueprint/voxel_raymarcher.rs` |
| `walls.rs` | 990 | Architectural wall segment extraction | `src/blueprint/architectural_analysis/walls.rs` |
| `batch.rs` | 943 | BLAS acceleration structure batch walker | `src/blueprint/bvh/batch.rs` |
| `prefab.rs` | 822 | Parser for Enfusion entity templates (`.et`) | `src/blueprint/prefab_parser.rs` |
| `verify.rs` | 765 | Instance reconstruction verifier | `src/blueprint/verify.rs` |
| `pak.rs` | 679 | FORM/PAC1 Enfusion `.pak` archive reader | Unified into `src/enfusion_pak/` |
| `archive_emit.rs`| 651 | Binary `building_blueprints.rkyv` archiver | `src/blueprint/archive_compiler.rs` |
| `library.rs` | 617 | Prefab BLAS catalog generator | `src/blueprint/library.rs` |
| `bvh.rs` | 566 | 3D BVH collision sidecar writer (`.bvh`) | `src/blueprint/bvh/builder.rs` |
| `mod.rs` | 500 | Module declarations & CLI router | `src/blueprint/mod.rs` |
| `rotation_pin.rs`| 408 | Euler angle composition order pin | `src/blueprint/rotation_pin.rs` |
| `world_row.rs` | 399 | Terrain world entity row verifier | `src/blueprint/world_row.rs` |
| `rings.rs` | 376 | Rectilinear 2D boundary tracing | `src/blueprint/architectural_analysis/rings.rs` |
| `emit.rs` | 344 | Assembles `BuildingBlueprint` JSON schema | `src/blueprint/emit.rs` |
| `inspect.rs` | 312 | Low-level CLI inspector for `.xob` chunks | `src/blueprint/inspect.rs` |
| `hull.rs` | 267 | 3D convex hull triangulation | `src/blueprint/hull.rs` |
| `roof.rs` | 246 | Top surface voxel roof heightfield downsampler | `src/blueprint/architectural_analysis/roofs.rs` |
| `params.rs` | 223 | Tunable heuristic parameters | `src/blueprint/params.rs` |
| `parse.rs` | 216 | NDJSON voxel dump reader | `src/blueprint/parse.rs` |
| `library_cli.rs` | 211 | CLI harness for `bvh-batch` | `src/blueprint/library_cli.rs` |
| `surface_kind.rs`| 208 | Maps collision materials to surface kinds | `src/blueprint/surface_kind.rs` |
| `slabs.rs` | 186 | Vertical voxel column slab analyzer | `src/blueprint/architectural_analysis/slabs.rs` |
| `types.rs` | 167 | In-memory voxel and blueprint models | `src/blueprint/types.rs` |
| `march.rs` | 136 | Raymarching skeleton for voxel clouds | `src/blueprint/march.rs` |
| `pair.rs` | 122 | Face pairing for collision geometry | `src/blueprint/pair.rs` |
| `plate.rs` | 60 | Floor plate detector | `src/blueprint/architectural_analysis/plates.rs` |
| *(Test Files)* | 1,738 | Unit tests for mesh, library, synth, and batch | `src/blueprint/tests/` |

**Conclusion**: Removing `map_blueprint/` from `xtask` strips 13.4k LOC of heavy 3D math and allows `xtask` to compile rapidly without linking `website-map-engine` 3D features.

---

## 5. Forensic Analysis of Root `scripts/` Directory

### 5.1 Reality: Zero Scripts in `scripts/`
Following the T-853 / T-620 shell script eradication programs, all bash and python scripts were completely ported to Rust and removed from git. However, the `scripts/` directory was left sitting at the repository root as an obsolete container holding configuration files, systemd templates, test fixtures, and orphaned bytecode:

| Path in `scripts/` | Git Status | What It Actually Is | Why It Violates Core Laws | Target Clean Destination |
|---|---|---|---|---|
| `scripts/deploy/deploy.env.example` | Tracked | Staging server environment variable template (TBD_SSH_HOST, etc.) | Configuration template, not a script. Violates Law 4. | `tools_v2/xtask/deploy/deploy.env.example` (or `packages/deployment/`) |
| `scripts/deploy/Caddyfile.website` | Tracked | Production & staging Caddy reverse proxy configuration | Server web configuration, not a script. Violates Law 4. | `tools_v2/xtask/deploy/Caddyfile.website` |
| `scripts/deploy/*.service`, `*.timer` | Tracked (5 files) | Linux systemd service and timer unit definitions for backups and website runner | Systemd unit configuration files. Violates Law 4. | `tools_v2/xtask/deploy/systemd/` |
| `scripts/mod/fixtures/mcp-*.jsonl` | Tracked (5 files) | Test data fixtures for MCP JSON-RPC daemon testing | JSONL test fixtures sitting in a "scripts" folder. Violates Law 4 & 5. | `tools_v2/developer-tools/test_fixtures/mcp/` |
| `scripts/mod/tbd-*-server.config.json`| Tracked (2 files) | Dedicated server JSON profile configurations (dev & staging) | Game server configuration files. Violates Law 4. | `apps/mod/Configs/Server/` (or `tools_v2/xtask/src/commands/mod_ops/configs/`) |
| `scripts/mod/package.json` | Tracked | Pinned npm package for `enfusion-mcp: 0.6.1` | Node dependency manifest for tier-2 MCP calls. | `tools_v2/developer-tools/src/enfusion_tooling/mcp_node_bridge/package.json` |
| `scripts/mod/node_modules/` | Untracked | Local node dependencies for `enfusion-mcp` | Untracked dependency debris. | Deleted / gitignored under tools. |
| `scripts/platform/__pycache__/` | Untracked | Python bytecode from pre-Rust era (`slice-collisions.cpython-311.pyc`) | Orphaned dead-weight Python bytecode. Violates Law 3. | Deleted entirely. |

### 5.2 Resolution: Complete Elimination of Root `scripts/`
Once the deployment configurations and systemd units are moved to `tools_v2/xtask/deploy/`, the MCP fixtures are moved to `tools_v2/developer-tools/test_fixtures/mcp/`, and the server profiles are moved to their respective configs, the root `scripts/` directory is **completely deleted**.

This achieves the core architectural mandate: **the repository root retains strictly `apps/`, `tools/`, `packages/`, and `docs/`**.
