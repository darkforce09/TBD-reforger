**Status:** archived — see [the restructure program](/documentation/archive/restructure/README.md)

# Tooling split analysis and claim checks

I've finished both jobs. All counts come from read-only `grep`/`wc` over `/home/user/TBD-reforger`. In the line counts, "prod/test" means files under `tests/` (or named `*tests.rs`) count as test, and so does everything from the first `#[cfg(test)]` in a file onward. Edge counts include doc-comment links; where it matters I name the edges that exist in real code.

# JOB 1: Tooling crates split analysis

## 1.1 xtask (`tools_v2/xtask`): 639 files, 78,865 prod / 36,324 test

**Top-level tree (prod/test):**
- `cli` 210/0: the single clap `TopCmd` at `src/cli/mod.rs:36-160` and the router at `src/cli/dispatch.rs:7-46`.
- `core` 1,764/829: `host_execution.rs` 467, `deploy_environment*` 623, `cargo_target_directory.rs` 353, `repository_layout.rs` 163, `secure_shell_transport.rs` 122, `repository_root.rs` 23.
- `commands` 56,532/20,564:

| family | prod | test |
|---|---|---|
| staging | 14,197 | 6,182 |
| mod_ops | 10,559 | 2,761 |
| platform | 9,952 | 3,210 |
| deploy | 7,114 | 3,047 |
| ballistics | 2,318 | 647 |
| mcp | 2,198 | 407 |
| db | 1,874 | 923 |
| debug | 1,492 | 591 |
| ci | 1,486 | 350 |
| setup | 849 | 629 |
| generate | 832 | 319 |
| build | 710 | 332 |
| map | 706 | 283 |
| fetch | 653 | 198 |
| agent_context | 449 | 149 |
| ticket | 349 | 62 |
| verify | 289 | 0 |
| reproduction | 279 | 17 |
| schema | 138 | 457 |
| wave | 49 | 0 |

- `verifications` 20,340/13,264:

| family | prod | test |
|---|---|---|
| documentation | 5,162 | 5,325 |
| mod_scripts | 4,915 | 1,084 |
| schemas | 2,686 | 1,056 |
| api_readiness | 2,234 | 2,184 |
| database | 1,105 | 539 |
| architecture | 1,104 | 994 |
| ci | 1,039 | 501 |
| deployment | 537 | 395 |
| language_bans | 508 | 615 |
| licensing | 507 | 240 |
| registry | 418 | 194 |
| map_assets | 43 | 0 |

**Notable sub-families (prod/test):**
- platform: `platform/wave_execution` 8,327/2,128, `preflight` 814, `slice_worktree` 724.
- mod_ops: `playtest_server` 3,111/892, `equipment_vehicle_export` 1,568, `equipment_gameplay` 1,215, `website_api_client` 954, `world_boot` 776, `compile` 739, `wave_execution` 614.
- staging: `fleet_procedure` 4,587/1,748, `load_procedure` 2,975/1,598, `discord_procedure` 1,827, `procedure_runner` 1,406.
- deploy: `deploy/staging` 4,017/2,147, `deploy/website` 950, `database_operations` 799, `restore_drill` 548.
- `verifications/documentation/link_check` 3,788/2,899.

**Libraries vs CLI glue.**
- Every command family follows the same `cli.rs` (clap `Subcommand`) + `dispatch.rs` + library-body pattern.
- `commands/verify` (289 lines) is pure glue over `verifications/*`.
- `commands/schema` (138) is glue over `generate` and `ci`.
- `commands/ticket` and `commands/wave` are thin adapters over ticket-engine (a test enforces this at `src/tests/tooling_dependency_boundaries.rs:~127`).
- `ci` (`ci/task_definitions.rs`, `task_runner`) and `build` (`build/recipes.rs:TARGETS`) are data tables of shell and native steps.
- The real libraries are `verifications/*`, `platform/wave_execution`, `staging/*`, `deploy/staging`, `mod_ops/playtest_server`, `generate/schema_types` and `core`.

**Cross-module real-code cycles (comments and tests excluded):**
- `commands::ci` ↔ `verifications::ci`: v::ci → ci 11 (reads the `task_runner::TASKS` table, `verifications/ci/schema_parity/source_audit.rs:178-287`); ci → v::ci 2 (`commands/ci/task_definitions.rs:12`).
- `commands::platform` ↔ `commands::ticket`: `platform/dispatch.rs:2` does `use crate::commands::ticket::*`; ticket → platform 2.
- `core` is not a leaf:
  - `core/cargo_target_directory.rs:47` → `commands::build::recipes`
  - `core/repository_root.rs:26` (test-only) → `commands::platform::wave_execution::testcwd`
- `verifications::documentation` → `crate::cli::Cli`, `commands::build::recipes` and `commands::ci::task_runner` (`link_check/command_citations.rs:32-34`). Since `cli` → `commands::verify` → `verifications::documentation`, this closes a cycle through the root CLI.
- One-way but cross-family edges:
  - mod_ops → platform 3, setup 2, debug 1
  - ci → build 1; platform → ci 2
  - staging → deploy 9 (`deploy::staging`, `database_*`, `remote_rust_toolchain`)
  - staging → `verifications::api_readiness` 36 (heavy)
  - db → deploy 12; debug → deploy 6
  - verifications::database → commands::db 2
  - generate → `verifications::language_bans::node_and_file_limits::gen_font_table` (`commands/generate/dispatch.rs:8`)
- Crate-root `#[macro_export]` macros `wprintln!`/`wprint!`/`werr!` are defined in `commands/platform/wave_execution/mod.rs:141-160`. They are used by 24 files, all in platform.
- `use crate::*` glob imports: `cli/dispatch.rs:3`, `commands/{schema,wave,map,verify}/dispatch.rs`.

**External deps, by user module (non-test files):**
- anyhow: 206 files, everywhere.
- clap: every `cli.rs`.
- regex: mod_scripts, mod_ops, platform, deploy, architecture, and more.
- serde_json: staging 29, mod_ops 29.
- jsonschema: verifications/schemas, mod_ops.
- typify, schemars, prettyplease, heck: `commands/generate` only (heck also has 1 use in `mod_ops/equipment_gameplay`).
- syn: generate and `verifications/architecture`.
- walkdir: platform, mod_ops, schemas, generate.
- sha2 0.10: mod_ops, staging, schemas, api_readiness, ballistics.
- flate2 and tar: `mod_ops/equipment_vehicle_export/upload_bundle.rs`, plus flate2 in `schemas/checks/map_glyphs.rs:36`.
- serde_norway: `verifications/ci` only.
- libc: mod_ops, mcp, core, platform, ci, api_readiness.
- **`png` is declared (`Cargo.toml:18`) but never used.**
- **`toml` is used only in `src/tests/`** (`tooling_dependency_boundaries.rs:4`), so it should be a dev-dependency.
- **No tokio or reqwest directly** (`commands/deploy/staging/render.rs:88` says so explicitly). But xtask links `developer-tools`, which pulls in tokio "full", axum, reqwest, tokio-tungstenite, resvg, image and webp, and `website-map-engine` transitively. The boundary test only forbids a direct map-engine dependency (`tooling_dependency_boundaries.rs:36-37`).
- **What xtask actually uses from `developer_tools`:**
  - `repository_layout::*`: ~22 refs
  - `staging_verification::load_generation`: 14 (staging)
  - `world_export_pipeline`: 5
  - `map_verification::*`: 9
  - `blueprint::run_*`: 10
  - `enfusion_tooling::enfusion_mcp_entrypoint`: 3
  - `content_digest::sha`: 1
- **From `verification_core`:** `proc` 55+4, `verdict` 13, `scan` 6, `repository_laws::{file_length, engine_layers, source_roots}`, `gate`, `pattern`, `lock`. Raw `Command::new` still appears 113 times (platform 43, mod_ops 16, mcp 10, deploy 10, ci 8, db 7).

## 1.2 developer-tools: 308 files, 44,998 prod / 15,887 test

**Tree (prod/test):**
- `browser_testing` 11,372/2,287:
  - `editor_smoke_tests` 4,629
  - `mortar_offline` 1,313
  - `ballistics_agreement` 866
  - `dom_oracle` 800
  - `diagnostics` 762
  - `cdp` 750
  - `server` 566
  - `screen_capture` 536
  - `equipment_data_viewer` 412
  - `cli` 315
  - `capture_cli` 135
- `blueprint` 9,817/4,113:
  - `bvh` 3,251
  - `architectural_analysis` 1,808
  - `voxel_processing` 1,568
  - `archive_emission` 1,505
  - `mesh_decoding` 1,351
- `world_export_pipeline` 7,551/2,644:
  - `chunk_partitioner` 1,410
  - `mathematical_verification` 1,414
  - `export_preparation` 1,318
- `map_raster_pipeline` 5,871/903:
  - `aerial_orthophoto` 1,106
  - `inland_water` 1,036
  - `satellite_archive` 717
  - `cartographic_rendering` 682
- `staging_verification` 4,122/3,803: `load_generation` 2,758, `acknowledgement_relay` 1,349.
- `map_verification` 2,882/818.
- `enfusion_tooling` 2,461/362: `mcp_broker` 459, `cli` 359, `symbols` 278, `carve` 291.
- `enfusion_pak` 515/674.
- `repository_layout.rs` 290, `repository_paths.rs` 27, `content_digest.rs` 24, `timestamp_formatting.rs` 20.

**Binaries** are one-line shims (`src/bin/*.rs`):

| bin | entry module |
|---|---|
| enf | `enfusion_tooling::cli` |
| gate | `browser_testing::cli` |
| mcpd | `enfusion_tooling::mcp_broker` |
| world | `world_export_pipeline::cli` |
| map | `map_raster_pipeline::cli` |
| capture | `browser_testing::capture_cli` |
| acknowledgement-dropping-relay | `staging_verification::acknowledgement_relay` |

**Cross-module edges:**
- map_raster → world_export 25 and enfusion_pak 3.
- blueprint → enfusion_pak 9.
- map_verification → blueprint 2 and world_export 4.
- world_export → enfusion_pak 5.
- Everything → `repository_layout`/`repository_paths`.
- **Cycle:** `repository_layout` ↔ `repository_paths`. `repository_paths.rs:11` imports `ROOT_MARKER` from layout, and layout doc-links back.
- **Spurious coupling:** world_export (11), map_raster (8) and enfusion_tooling (1) depend on `browser_testing` only for `browser_testing::server::repo_root` (`browser_testing/server.rs:446`). All 20 references are that one function. It drags in tokio and axum.

**External deps by module:**
- tokio: browser_testing 16, staging_verification 9, `enfusion_tooling/mcp_broker.rs` 1.
- axum: browser_testing, staging_verification.
- reqwest: browser_testing 7, staging_verification 4.
- tokio-tungstenite, base64: browser_testing (cdp).
- image, resvg, webp, image_webp: map_raster (image also in browser_testing).
- bcdec_rs: world_export.
- jsonschema: map_verification, world_export, blueprint.
- flate2: blueprint, map_verification, world_export, enfusion_pak.
- png: map_verification, map_raster, world_export.
- website_map_engine: blueprint 16, world_export 10, browser_testing 10 (only `ballistics_agreement` and `mortar_offline`), map_verification 8, map_raster 4.
- anyhow: 138 files; 95 `pub fn … -> anyhow::Result` APIs.
- clap: each per-bin `cli`.
- **No dependency on verification-core**, which contradicts `tools_v2/verification-core/Cargo.toml:1-3` ("heavy tooling in developer-tools link this crate").

## 1.3 verification-core: 44 files, 4,148 prod / 2,979 test

- **Modules:** `verdict` 225, `proc` 566 (`runner`, `stream`, `lookup` with `which`/`retry`/`wait_for`), `lock` 136, `scan` 128, `gate` 111, `report` 87, `pattern` 84, and `repository_laws` 2,743/2,101.
- **`repository_laws` contents:**
  - `engine_layers/` ~1,527 lines: hard-codes map-engine module names (`crate::data`, `frame`, `streaming`, and so on, ~100 string refs)
  - `cargo_manifest.rs` 265
  - `sibling_test_placement` 299
  - `crate_dependencies` 189
  - `exemption_mechanisms` 184
  - `source_roots` 173
  - `file_length` 121
- **Deps:** regex and libc only. No anyhow and no thiserror (the one "thiserror" hit is a comment, `engine_layers/scanning.rs:34`).
- **No real cycle.** proc → verdict is code; verdict → proc is doc-only (`verdict.rs:78,95`).
- **Split:** keep `verification-core` as primitives (verdict, proc, lock, scan, pattern, gate, report; ~1.4k prod). Move `repository_laws` (~2.7k) into its own crate, or into `repository_layout` + `engine-layer-laws` crates.

## 1.4 ticket-engine: 129 files, 10,279 prod / 9,184 test

- **Modules:** `validation` 1,469, `wave_lock` 1,436, `cli` 1,386 (verb library, **no clap**), `metrics` 1,378, `ops` 1,269, `registry` 1,245, `model` 526, `sync` 405, `encoding` 376, `repository` 275, `store` 263, `vocab` 139, `corpus_pins` 58, `timestamp` 46. There are also 12 crate-level `tests/` files (proptest, trybuild).
- **Deps:** anyhow (38 files; 48 pub `anyhow::Result` APIs), regex, serde, serde_json(preserve_order), walkdir, jsonschema, time, toml(preserve_order).
- **Real cycles:**
  - `metrics` ↔ `cli`: `metrics/estimates/mod.rs:14` uses `crate::cli::commit_subjects::SubjectCommit`.
  - `ops` ↔ `registry`: 3 and 5 refs.
  - Most other back-edges (repository → metrics, ops → cli, sync → cli) are doc links.
- **Root-level re-exports** (`lib.rs:21-26`, `pub use model::*`) make `crate::is_sha_shaped`-style edges ubiquitous.
- Spawns `git` directly 9 times (e.g. `registry/ticket_status_history.rs:32-81`, `wave_lock/history.rs:75,92`) instead of using `verification_core::proc`. A boundary test forbids depending on that crate (`tooling_dependency_boundaries.rs:89-104`).

## 1.5 ticketboard (egui): 148 files, 9,899 prod / 5,461 test

- **Modules:** `ticket_browser` 2,241, `execution_metrics` 1,787, `application` 1,682, `ticket_actions` 1,635, `repository_status` 690, `ticket_registry` 603, `wave_plan` 494, `document_viewer` 399, `core` 333.
- **Layered and acyclic.** `application` → features → `ticket_registry`/`core`. Each feature has `models`/`services`/`ui`.
- **UI vs headless:** `ui/` totals 3,049 lines; `models`+`services` total 3,536 and are egui-free (only doc mentions). That makes a `ticketboard-model` headless crate a clean cut.
- **Deps:** eframe 0.36.1, egui_commonmark, egui_extras, notify, rfd, serde, serde_json, time, toml 0.8, ticket-engine. No anyhow and no tokio.
- Uses `ticket_engine::repository` 21 times. Shells `cargo xtask ticket …` via `core/process/streaming.rs:75`.

## 1.6 Duplicated helpers

**Repository-root discovery: 7 variants.**
1. `tools_v2/ticket-engine/src/repository.rs:87`: canonical walk to `ROOT_MARKER` (`:28`).
2. `tools_v2/developer-tools/src/repository_paths.rs:13`: a declared copy. `ROOT_MARKER` is duplicated at `developer-tools/src/repository_layout.rs:212`.
3. `developer-tools/src/browser_testing/server.rs:446`: `CARGO_MANIFEST_DIR/../..`. This is exactly the cross-worktree hazard documented at `xtask/src/core/repository_root.rs:9-19`, and it is used 20 times.
4. `xtask/src/core/repository_root.rs:4`: delegates to (1). Good.
5. `xtask/src/verifications/language_bans/shell_scripts.rs:109` and `…/node_and_file_limits/repository_access.rs:15`: `git rev-parse`. The latter ignores the exit status.
6. `xtask/src/commands/mcp/call.rs:153`: falls back to `CARGO_MANIFEST_DIR`.
7. `apps/ticketboard/src/ticket_registry/services/discovery.rs:17`: `walk_up_for_tickets` uses `TICKETS_DIR`, not `ROOT_MARKER`, so the semantics differ.

**Path-layout modules: 4.**
- `xtask/src/core/repository_layout.rs`: deploy and profile consts (`:9-41`).
- `ticket-engine/src/repository.rs`: `.ai/tickets` and `.ai/artifacts` consts (`:24-75`) plus `SPARSE_CHECKOUT_SETS` (`:112`).
- `developer-tools/src/repository_layout.rs`: ~40 `fn(root) -> PathBuf` for contracts, fixtures, terrain, glyph, mcp and `.ai/artifacts/*` (`:19-268`).
- `verification-core/src/repository_laws/source_roots.rs`: `FILE_LENGTH_PINS`, `MOD_SCRIPT_ROOTS`, walkers (`:21-159`).
- Both `.ai/artifacts` (`ticket-engine repository.rs:65` and `developer-tools repository_layout.rs:228`) and `ROOT_MARKER` are spelled twice.

**Process running.**
- `verification_core::proc` (used by 46 xtask files).
- `xtask/src/core/host_execution.rs:354`, plus 113 raw `Command::new` in xtask.
- ticket-engine `git` × 9.
- developer-tools × 9.
- `ticketboard/src/core/process/streaming.rs`.

**Other duplicates.**
- Timestamps: hand-rolled `developer-tools/src/timestamp_formatting.rs:2` (Hinnant civil-from-days) vs `ticket-engine/src/timestamp.rs:39` (time crate).
- SHA helpers: sha2 0.10 in xtask vs 0.11 in developer-tools; ad-hoc sha256 helpers in 8 xtask files and 3 developer-tools files; `content_digest.rs` (sha384).
- MCP: `xtask/src/commands/mcp/daemon.rs` (lifecycle) and `developer-tools/src/enfusion_tooling/mcp_broker.rs` (broker) split a single concern across crates.

**Proposed `repository-layout` foundation crate** (deps: std plus optional anyhow/thiserror):
- `ROOT_MARKER`, `find_repo_root`, `is_repo_root`, `test_repo_root`.
- All path constants and path functions from the four modules.
- `source_roots` walkers.
- **Blocker:** the `foundational_engines_have_no_workspace_dependencies` test (`tooling_dependency_boundaries.rs:89`) must allow this leaf crate.

## 1.7 Proposed fine-grained crates (approximate prod lines)

**Foundations (leaf):**

| crate | prod | contents / deps |
|---|---|---|
| `repository-layout` | ~700 | §1.6 |
| `verification-verdict` | ~0.4k | verdict + report |
| `process-runner` | ~0.6k | `proc` + `host_execution` + `secure_shell_transport`; libc |
| `verification-scan` | ~0.4k | scan, pattern, gate, lock; regex |
| `repository-laws` | 1.2k | file_length, sibling_test_placement, cargo_manifest, crate_dependencies, exemption_mechanisms |
| `engine-layer-laws` | 1.5k | `engine_layers` |
| `tool-time` | small | timestamp |
| `tool-digest` | small | content_digest + sha helpers; unify on sha2 0.11 |

**ticket-engine split:**
- `ticket-model`: model + encoding + vocab + timestamp, ~1.1k.
- `ticket-store`: store + registry + corpus_pins.
- `ticket-validation`.
- `ticket-wave-lock`.
- `ticket-metrics`.
- `ticket-ops`/`ticket-cli-verbs`.
- **Blockers:** the metrics ↔ cli and ops ↔ registry cycles, and anyhow in the public API.

**xtask command families:**
- `xtask-ci-tasks`: `ci` table + `build` recipes, ~2.2k. Shared by `verifications::ci`, `verifications::documentation` and `core::cargo_target_directory`. **Blocker:** these consume the tables in-process, so extract the table types into a data crate first.
- `xtask-db`: 1.9k. Needs `deploy::database_*`, so extract `deploy-database`: `database_operations`, backup, restore, `restore_drill`, ~1.6k.
- `xtask-deploy-website`: ~1k.
- `xtask-deploy-staging`: ~4k.
- `xtask-staging-acceptance`: 14.2k. Could sub-split into fleet, load, discord and procedure-runner. Deps: `verifications::api_readiness` (36 refs) and `developer_tools::staging_verification::load_generation`.
- `xtask-mod-ops`: 10.6k. Could sub-split into compile, world_boot, playtest_server (3.1k), equipment_export (2.8k) and website_api_client (1k). Deps: platform 3, setup 2, debug 1.
- `xtask-platform-wave`: `wave_execution` 8.3k + preflight + slice_worktree. **Blocker:** `#[macro_export] wprintln` and the platform ↔ ticket glob cycle.
- `xtask-schema-codegen`: `generate` 832 + `schema` 138. Deps: typify, schemars, syn, prettyplease, heck. **Blocker:** the `gen_font_table` edge into language_bans.
- `xtask-mcp-client`: 2.2k.
- `xtask-ballistics`: 2.3k.
- `xtask-debug`: 1.5k.
- `xtask-setup`, `xtask-fetch`, `xtask-agent-context` (small).

**Verify families (one crate per family):**
- `verify-documentation`: 5.2k. **Blocker:** reads `crate::cli::Cli` via `clap::CommandFactory`. Needs an injected command tree or a dedicated `xtask-cli-model` crate.
- `verify-mod-scripts`: 4.9k.
- `verify-schemas`: 2.7k. Uses jsonschema and developer_tools layout.
- `verify-api-readiness`: 2.2k.
- `verify-database`: 1.1k. Needs a db-registry extraction.
- `verify-architecture`: 1.1k, syn.
- `verify-ci`: 1.0k, serde_norway.
- `verify-language-bans`, `verify-licensing`, `verify-deployment`, `verify-registry`.
- The `xtask` bin then becomes only `cli` + dispatch.

**developer-tools split:**

| crate | prod | deps / notes |
|---|---|---|
| `enfusion-pak` | 0.5k | flate2 |
| `enfusion-index` | ~2k | symbols, carve, apidoc, citations, index; + `enf` bin |
| `mcp-broker` | 0.5k | tokio, futures-util; + `mcpd` bin; merge in xtask `mcp/daemon` |
| `blueprint-compiler` | 9.8k | map-engine, jsonschema, flate2; could split bvh 3.3k, voxel, mesh_decoding, archive_emission |
| `world-export` | 7.6k | bcdec_rs, map-engine; + `world` bin |
| `map-raster` | 5.9k | image, resvg, webp; + `map` bin |
| `map-verification` | 2.9k | blueprint, world_export, jsonschema, png |
| `cdp-driver` | ~1k | tokio, tokio-tungstenite, base64; cdp + diagnostics |
| `gate-server` | 0.6k | axum, reqwest |
| `editor-gates` | ~8k | smoke tests, dom_oracle, mortar_offline, ballistics_agreement; + `gate` bin |
| `capture` | ~0.7k | |
| `staging-load-generator` | 2.8k | tokio, reqwest, axum |
| `ack-dropping-relay` | 1.3k | + bin |

- **Key win:** xtask stops compiling tokio, axum, reqwest and resvg. It would need only `repository-layout`, `blueprint-compiler`, `map-verification`, `world-export` and `staging-load-generator`.
- **Blockers:** replace `browser_testing::server::repo_root` with `repository-layout`, and fix the `repository_layout` ↔ `repository_paths` cycle.

**ticketboard:** `ticketboard-model` (headless models/services, ~3.5k) and `ticketboard` (egui).

## 1.8 anyhow usage per crate (files / pub-API exposure)

| crate | anyhow files | other |
|---|---|---|
| xtask | 206 | 0 thiserror |
| developer-tools | 138 | 95 pub `anyhow::Result` fns |
| ticket-engine | 38 | 48 pub `anyhow::Result` fns |
| api_v2 | 26 | thiserror in 3 files |
| verification-core | 0 | own `Verdict`/`NotRun` |
| ticketboard | 0 | |
| map-engine | 0 | thiserror in 15 files |
| fleet_host_agent | 0 | thiserror in 7 files |
| frontend, graphics-engine, offline-sw | 0 | |

# JOB 2: Claim verification

**a. TRUE.**
- The executable bodies are identical (diffed with comments stripped):
  - `apps/website/frontend/src/v2/core/auth/url_guard.rs:66-76`
  - `apps/website/api_v2/src/core/text/http_url_guard.rs:62-79`
- Both do `use url::Url` (`:36` and `:4` respectively).
- Dependency versions: frontend `url = "2.5"` (`frontend/Cargo.toml:42`); api_v2 `url = "2.5.8"` (`api_v2/Cargo.toml:66`).
- The shared case table `apps/website/shared/is_http_url_cases.rs` is `include!`d by both test files (`api_v2/src/core/text/tests/http_url_guard.rs:106`, `frontend/src/v2/core/auth/tests/url_guard.rs:7`).

**b. PARTIALLY TRUE: the ~139 figure matches frontend DTOs, not map-engine.** The `pub` id fields (`id` / `*_id`, including `Option<>`) break down as:

| type | map-engine | api_v2 (all) | api_v2 models | frontend (all) | frontend `v2/core/api/dto` |
|---|---|---|---|---|---|
| String | 129 | 108 | 68 | 182 | **139** |
| u16 | 1 | – | – | – | – |
| u32 | 5 | – | – | – | – |
| u64 | – | 1 | 1 | 3 | 1 |
| i64 | – | 3 | 2 | 1 | 1 |
| i32 | – | – | – | 1 | – |
| **primitive total** | **135** | 112 | 71 | 187 | **141** |
| Uuid | 0 | 152 | 119 | 0 | 0 |

- map-engine's 135 fields are mostly in `data` (85), `world` (20) and `io` (16); one of them is `pub(crate)`.
- id params:
  - map-engine: 42 String/u64 and 219 `&str`
  - api_v2: 25 `&str` and 14 Uuid
  - frontend: 39 String, 119 `&str` and 8 integer

**c. TRUE**, for the non-`.c`, non-`.md` files.
- References:
  - `apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et:23` (`TBD_ObjectivesComponent`)
  - `apps/mod/tbd-framework/UI/layouts/Hud/TBD_ObjectiveHud.layout:4` (`TBD_ObjectiveHud`)
  - plus the `.layout.meta:2` path
- None of the 15 `.conf` files reference a `TBD_Objective*` class.
- `resourceDatabase.rdb` contains only file-path strings (`TBD_ObjectiveRegistry`, `TBD_ObjectiveRules`, and so on).
- Other classes appear only in `.md` files (Registry, HudPublisher, Progression, …).

**d. TRUE, all exist, with nuances:**

| command | where it is defined |
|---|---|
| `ci verify-codegen-fresh` | `cli/mod.rs:156` `Ci { target: Option<String> }` → task row `commands/ci/task_definitions.rs:159` |
| `mk ci-local-leptos` | `commands/build/recipes.rs:163`, `recipes/execution.rs:55`; also a `ci` task (`task_definitions.rs:429`) |
| `mk leptos-gates` | `recipes.rs:160`, `execution.rs:52`, `shell_word.rs:249`; **mk only, not a `ci` task** |
| `mk leptos-build` | `recipes.rs:158`, `execution.rs:50`; also a `ci` task (`task_definitions.rs:441`) |
| `deploy website --dry-run` | `commands/deploy/cli.rs:6-7`; trailing args parsed at `website.rs:56` |
| `deploy staging --dry-run` | `cli.rs:15-16`; `staging.rs:144` |
| `db test-it` | `commands/db/operations.rs:219-220` |
| `verify engine-layers` | `commands/verify/cli.rs:110-111`, dispatched at `verify/dispatch.rs:97` |
| `mod world-boot` | `commands/mod_ops/cli.rs:117-118` |
| `mod compile` | `commands/mod_ops/cli.rs:102-103` |

- `mk` and `ci` targets are strings in tables, not clap subcommands, so clap help does not list them.

**e. TRUE, but the file is redundant.**
- `apps/website/api_v2/rust-toolchain.toml` and the root file are the same apart from comments and the `targets` line: channel `"1.95.0"`, components `["rustfmt","clippy"]`, profile `"minimal"`.
- The root adds `targets = ["wasm32-unknown-unknown"]`; api_v2 lacks it.

**f. TRUE.** Root `Cargo.toml` is 19 lines: only `[workspace]` with `resolver = "3"` and members. It has no `[workspace.dependencies]`, `[workspace.lints]` or `[workspace.package]`, and no member has `[lints]`.

| dep | crates | specs |
|---|---|---|
| serde | 9 | `"1"`+derive ×7 (map-engine optional); `"1.0.228"` in api_v2 and fleet |
| serde_json | 9 | 6 shapes: `"1"`, `"1.0.150"`, preserve_order, preserve_order+raw_value (xtask), float_roundtrip+raw_value (api_v2), optional |
| anyhow | 4 | `"1"` ×3; api_v2 `"1.0.103"` |
| regex | 4 | `"1"` |
| jsonschema | 4 | identical `0.46.10` |
| thiserror | 3 | `2.0.18` ×2, `"2"` optional |
| tokio | 3 | dev-tools `1.52` full, api_v2 `1.52.3` full, fleet `1.52.3` subset |
| reqwest | 3 | `0.13.4`, three different feature sets |
| axum | 3 | |
| url | 3 | |
| png | 3 | |
| flate2 | 3 | |
| libc | 3 | |
| wasm-bindgen, js-sys, web-sys | 4 each | |
| toml | 4 | `0.8` ×3 vs `1.1.3` in fleet_host_agent |
| sha2 | 3 | `0.10` in xtask vs `0.11` in dev-tools and api_v2 |

- **Real version splits in `Cargo.lock`:** toml 0.8.23 + 1.1.3; sha2 0.10.9 + 0.11.0; png 0.17.16 + 0.18.1; thiserror 1.0.69 + 2.0.18; rand ×3; getrandom ×3.
- `edition = "2024"` and `rust-version = "1.95"` are repeated in 10 manifests. `frontend/Cargo.toml` is **edition 2021 with no rust-version**.

**g. TRUE.**
- `apps/website/map-engine/src/frame/boot.rs:458` defines `pub(crate) fn instance_descriptor`. Doll uses it at `doll/renderer/lifecycle_1.rs:9`, `:123` and `:125`.
- `editing/tools/selection/gesture.rs:37` does `pub use crate::frame::EngineHandle;`, which `selection/mod.rs:20` re-exports again.

**h. PARTIALLY FALSE on both halves.**
- The frontend uses 4 of the 5 lib modules: `cache_names`, `network_fallback`, `offline_pack` and `request_classification` (`frontend/src/v2/core/offline/{pack_download.rs:22-25, offline_manifest.rs:19-20, service_worker_registration.rs:16, saved_copies.rs:21}`). **`range_slicing` is unused by the frontend.**
- The bin uses the lib's `cache_names`, `network_fallback`, `request_classification` and `range_slicing` (not `offline_pack`), plus web-sys, js-sys, wasm-bindgen and wasm-bindgen-futures (`offline-service-worker/Cargo.toml` wasm32 target deps; `src/bin/offline_service_worker/{worker_scope.rs:15, fetch_handling.rs:28-31, cached_range_response.rs:15}`). So it is not web-sys only.

**i. FALSE.**
- There are zero `sqlx::query!`, `query_as!` or `query_scalar!` macro calls in `apps/` or `tools_v2/`, and no `.sqlx/` directory exists.
- The `sqlx` "macros" feature is enabled (`api_v2/Cargo.toml:57`), and 60 api_v2 files derive `FromRow`.
- Two leftovers point at the macros anyway: the vestigial `mk rust-sqlx-prepare` target (`build/recipes/shell_word.rs:143`) and `SQLX_OFFLINE: "true"` in `.github/workflows/ci.yml:85,89,111`.

**j. Not usable out of the box; it works with an env override, and trunk is missing.**
- **What `/opt/pw-browsers` contains:** `chromium-1194/chrome-linux/chrome`, which runs ("Chromium 141.0.7390.37"), and `chromium_headless_shell-1194`. `PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers` is set. `~/.cache/ms-playwright` is absent.
- **How the harness looks for Chrome:** `find_chromium` (`developer-tools/src/browser_testing/cdp/sleep_ms.rs:18-49`) checks only `CHROME_HEADLESS_SHELL`, then `$HOME/.cache/ms-playwright`, then the relative paths `chrome-linux64/chrome` or `chrome-headless-shell-linux64/…`. It ignores `PLAYWRIGHT_BROWSERS_PATH` and the old `chrome-linux/` layout, so it finds nothing here. Gate doctor then fails with "✗ chromium not found" (`browser_testing/diagnostics/ensure_gate_font_cache.rs:282-286`).
- **What gate-env.json pins:** `tools_v2/developer-tools/gate-env.json` (read at `ensure_gate_font_cache.rs:186`) pins Playwright build 1228 and Chrome 149.0.7827.55 (full chrome build), rustc 1.95.0, trunk 0.21.14 and wasm_bindgen 0.2.126.
- **Workaround:** set `CHROME_HEADLESS_SHELL=/opt/pw-browsers/chromium-1194/chrome-linux/chrome`. That resolves a full build, and the 141-vs-149 mismatch is only a warning ("version drift", `:299-305`) unless `gate doctor --strict` (`browser_testing/cli.rs:77`).
- **CI path:** CI installs Chrome-for-Testing via `cargo xtask ci ci-chrome` (`xtask/src/commands/ci/chromium_install.rs:16-17,91`), which exports `CHROME_HEADLESS_SHELL`.
- **Other blockers:** `trunk` and `wasm-bindgen` are not installed, so `leptos-build` and `leptos-gates` cannot run. rustc 1.95.0 and the wasm32 target are present.
