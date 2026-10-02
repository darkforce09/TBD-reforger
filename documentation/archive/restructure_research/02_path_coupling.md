**Status:** archived — see [the restructure program](/documentation/restructure/README.md)

# Path coupling census

I didn't change anything. The short answer is that about 6,000 of the 16,209 tracked files spell at least one of the paths you plan to rename. The tooling mostly keeps its paths in a few layout modules, but the website crates, the deploy files and the `.ai/tickets/` files repeat them as plain strings.

## 1. Counts per path (`git grep -l -F` files / total occurrences)

| Path string | Files | Occurrences | xtask/src | developer-tools | other tools_v2 | .github | documentation_v2 | .ai/tickets | READMEs in code trees | apps .rs | config | apps/mod (non-README) | rest¹ |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `assets_v2` | 278 | 660 | 32 | 55 | 1 (+2 xtask non-src) | 2 | 71 | 15 | 68 | 24 | 2 | 0 | 6 |
| `contracts_v2` | 723 | 1,327 | 43 | 24 | 2 (+2) | 3 | 95 | 21 | 189 | 330 | 2 | 5 | 7 |
| `documentation_v2` | 2,020 | 12,005 | 89 | 59 | 27 (+8) | 2 | 546 | 709 | 540 | 10 | 2 | 7 | 15 (+6 .ai/artifacts) |
| `tools_v2` | 835 | 3,299 | 168 | 79 | 37 (+13) | 3 | 174 | 121 | 206 | 12 | 8 | 0 | 8 (+6) |
| `apps/website/api_v2` | 512 | 1,845 | 78 | 3 | 13 (+5) | 2 | 193 | 69 | 167 | 19 | 3 | 0 | 11 |
| `apps/website/frontend` | 1,529 | 5,207 | 27 | 28 | 13 (+2) | 2 | 459 | 490 | 292 | 13 | 2 | 1 | 9 (+274 .ai/artifacts) |
| `apps/website/map-engine` | 361 | 939 | 9 | 10 | 12 | 1 | 82 | 49 | 182 | 6 | 2 | 0 | 2 (+13) |
| `apps/website/graphics-engine` | 64 | 117 | 4 | 0 | 10 | 1 | 23 | 8 | 14 | 8 | 2 | 0 | 1 |
| `apps/website/offline-service-worker` | 18 | 23 | 0 | 1 | 2 | 0 | 5 | 2 | 6 | 0 | 1 | 0 | 1 |
| `src/v2` | 632 | 2,313 | — | — | — | — | — | — | — | — | — | — | — |
| `crate::v2::` | 704 | 2,484 | — | — | — | — | — | — | — | — | — | — | — |

¹ "rest" is mostly `CLAUDE.md` (22 lines), the root `README.md`, `.cursor/` and `.claude/` (10 files).

- **Doc subfolders with the same names:** `documentation_v2/` has subfolders called `assets_v2/`, `contracts_v2/` and `tools_v2/`. 11, 23 and 33 files refer to them through `documentation_v2/<name>`. A blind find-and-replace of `assets_v2` would also rename those doc paths.
- **Mod files:** 166 READMEs plus 11 EnfScript `.c` files, all comments except one generated header (see section 9).
- **Package names (files):**

| Name | Files | Main places |
|---|---|---|
| `website-api` | 267 | xtask 28, verification-core 15 |
| `website-frontend` | 332 | — |
| `website-map-engine` | 157 | — |
| `website-graphics-engine` | 88 | — |
| `website-offline-service-worker` | 12 | — |
| `website_map_engine` (Rust `use` name) | 420 | apps .rs 227, developer-tools 79 |
| `website_api` | 207 | apps .rs 166 |
| `website_graphics_engine` | 72 | — |

- **Cargo path dependencies that break:**
  - `tools_v2/developer-tools/Cargo.toml:24` points to `../../apps/website/map-engine`.
  - `apps/website/api_v2/Cargo.toml:85` points to `../../../tools_v2/verification-core`.
  - `apps/ticketboard/Cargo.toml:51` points to `../../tools_v2/ticket-engine`.
  - The api, frontend and map-engine manifests use sibling paths like `../map-engine` and `../graphics-engine`.

## 2. Is xtask centralised?

Partly. There are owner modules, but many literals sit outside them.

**Owner modules:**
- `/home/user/TBD-reforger/tools_v2/xtask/src/core/repository_layout.rs` (166 lines, imported by 107 files). It holds the deploy, systemd, Caddyfile and profile paths. Its `documentation` submodule holds `CODE_TREES = ["apps","tools_v2","contracts_v2","assets_v2"]`, `ARCHIVE_DIR`, `TICKET_DOCUMENTS_DIR`, `PROGRAM_RECORDS_PREFIX`, `API_READINESS_*` and the runbook paths. Its own header says a move should be "one edit here".
- `/home/user/TBD-reforger/tools_v2/ticket-engine/src/repository.rs`: `documentation::TREE_DIR = "documentation_v2"`, `PLANS_DIR`, `SPECS_DIR`, `ROADMAP`, `GAP_ANALYSIS`, and `SPARSE_CHECKOUT_SETS` (pins `apps/website`, `contracts_v2`, `tools_v2`).
- `/home/user/TBD-reforger/tools_v2/developer-tools/src/repository_layout.rs`: `contracts_v2`, `assets_v2/terrains|glyphs|scratch`, `documentation_v2`.
- `/home/user/TBD-reforger/tools_v2/developer-tools/src/repository_paths.rs`.
- `/home/user/TBD-reforger/tools_v2/verification-core/src/repository_laws/source_roots.rs`.

**Literals outside those modules** (non-test xtask files):

| Path string | Files |
|---|---|
| `apps/website` | 30 |
| `tools_v2` | 19 |
| `contracts_v2` | 13 |
| `assets_v2` | 11 |

The main offenders:
- `verifications/architecture/editor_orbat_coherency.rs`: 84 `apps/website/frontend/src/v2/...` literals.
- `commands/ci/task_definitions.rs`: `cd apps/website/api_v2 && …` and LFS includes.
- `ci/task_definitions/map_asset_steps.rs`
- `deploy/website/rsync_argv.rs`, `deploy/staging/remote/ssh_argv.rs`, `deploy/website/asset_preflight.rs`
- `platform/wave_execution/{changed.rs (FRONTEND_DIR), trunk.rs, schema.rs, gate/gate_dispatch.rs, touch.rs}`
- `verifications/deployment/staging_compose_paths.rs`
- `db/operations/selftest.rs`

## 3. Paths that jump between crates (these break on any move)

I resolved every `include!`, `include_str!`, `include_bytes!` and `#[path]` against its crate root.

- **No `#[path]` crosses a crate.** All 305 frontend `#[path]` attributes are inside the crate; 41 go up with `../` inside `src/`, so flattening `src/v2` moves them too.
- **Relative string literals in `.rs` that leave the crate:** 131 files, 357 occurrences.

| Crate | Files |
|---|---|
| frontend | 55 |
| api_v2 | 30 |
| map-engine | 30 |
| xtask | 10 |
| verification-core | 5 |
| offline-service-worker | 1 |

| Target | Occurrences | Note |
|---|---|---|
| `../map-engine` | 189 | sibling path; breaks if map-engine leaves `apps/website/` |
| `../../../contracts_v2` | 49 | crate depth 3 is assumed |
| `../../../assets_v2` | 24 | crate depth 3 is assumed |
| deeper `../…/contracts_v2` and `../…/assets_v2` | ~30 | — |
| `../api_v2` | 10 | — |
| `../../frontend` and `../frontend` | 14 | — |
| `../graphics-engine` | 8 | — |
| `../shared` | 7 | — |
| `../mod` | 7 | — |

Moving `apps/website/api_v2` to `apps/api` cuts the depth from 3 to 2, so every `../../../X` breaks even where X keeps its name.

**`include*!` that cross crates:**
- **api_v2 → `contracts_v2`:**
  - `src/core/text/tests/content_url_policy.rs`
  - `src/missions/contract/schema_validators.rs`
  - `src/missions/contract/tests/loadout_projection.rs`
  - `src/missions/handlers/mission_default_overrides.rs` (production code; this is why the Dockerfile copies `contracts_v2`)
  - `tests/current_profile_contract.rs`
- **api_v2 → frontend:**
  - `tests/current_profile_contract.rs`
  - `tests/event_access_contract.rs` (uses `frontend/tests/fixtures/api/*.json`)
  - `tests/fire_mission_solution.rs` (uses `frontend/src/v2/pages/field_tools/mortar/saved_fires/restore.rs`)
- **api_v2 → map-engine:** `src/missions/contract/tests/zone_quantisation.rs`
- **api_v2 → `tools_v2/xtask/deploy/Caddyfile.website`:** `tests/forwarded_for_trust.rs`
- **api_v2 and frontend → `apps/website/shared/is_http_url_cases.rs`** (a loose fragment that belongs to no crate; it needs a new home):
  - `src/core/text/tests/http_url_guard.rs`
  - `tests/aar_replay_url_backfill.rs`
  - `frontend/src/v2/pages/navigation/tests/layout.rs`
- **api_v2 → `apps/website/docker-compose.staging.yml`:** `src/core/configuration/tests/configuration.rs`
- **frontend → `contracts_v2/definitions/fire-mission.schema.json`:** `src/v2/pages/field_tools/mortar/tests/inputs.rs`
- **map-engine → `assets_v2/terrains/everon/*.json`:**
  - `src/overlay/symbology/tests/text_layout.rs`
  - `src/streaming/loaders/tests/manifest_tests.rs`
- **offline-service-worker → `assets_v2/terrains/{arland,everon}/manifest.json`:** `src/tests/offline_pack.rs`
- **xtask:**
  - `commands/deploy/tests/website/tests.rs` (uses the api `.env.example`, `Cargo.toml`, the staging compose file and `developer-tools/Cargo.toml`)
  - `commands/mod_ops/equipment_vehicle_export/validation.rs` (uses `contracts_v2/...` and is not a test)

**`CARGO_MANIFEST_DIR` + `..`:** 177 files use the macro. The ones that jump crates:
- api_v2 tests: `contract_support/mod.rs`, `route_acceptance_support/contracts.rs`, `enfscript_source_support/{mod,contract_tag}.rs`, `map_assets_rate_limit_exemption.rs`, `contract_parity_support/golden_index.rs` (`../frontend/tests/fixtures/api`), `registry_compat.rs`, `factions.rs`, `contract_parity_equipment_viewer.rs`, `contract_parity_mod_wire.rs`
- map-engine tests: `streaming/loaders/tests/*`, `world/environment/buildings/tests/prefab_tests.rs`, `t152_3_tests/mod.rs`
- frontend: `apps/editor/arsenal/asset_catalog.rs` (production code, reads `../../../apps/mod/tbd-framework/Data/registry.json`), `arsenal/tests/shell_wiring.rs` (many `/../map-engine/...` and one `documentation_v2/...` path)
- tools_v2: about 8 files that do `.join("../..")` from the crate to reach the repo root. These depend only on depth; `tools_v2` → `tools` keeps the depth, so they are fine.

## 4. Workflows (`/home/user/TBD-reforger/.github/workflows/`)

- **`ci.yml`:** no `paths:` filter.
  - The `website-api` job has `working-directory: apps/website/api_v2` (lines 70, 79, 95).
  - `git lfs pull --include assets_v2/terrains/everon/dem/everon-dem-16bit.png` appears at lines 128 and ~185.
  - The jobs are named `website-api` and `website-frontend`.
  - The rest is `cargo xtask …` calls.
  - `verify ci-schema-parity` (`verifications/ci/schema_parity.rs`) pins `.github/workflows/ci.yml` and `wave_execution/gate.rs`.
- **`contracts.yml`:** `paths: apps/website/**, contracts_v2/**, apps/mod/**`.
- **`editor-gates.yml`:** `paths: apps/website/frontend/**, tools_v2/developer-tools/**, apps/website/map-engine/**`, plus `git lfs pull --include "assets_v2/terrains/everon/**"`.
- **`mod-gates.yml`:** its `paths:` list (`tools_v2/xtask/**`, `contracts_v2/fixtures/...`) is commented out.
- **`schema.yml`:** `paths: contracts_v2/**`.

## 5. Frontend `src/v2/`

- It is declared once, as `mod v2;` at `/home/user/TBD-reforger/apps/website/frontend/src/main.rs:10`. `main.rs:21-26` also uses `v2::…` directly.
- `src/v2/mod.rs` declares `pub mod apps; pub mod core; pub mod pages;` plus `#[path = "tests/doc_audit/mod.rs"] mod doc_audit`.
- `crate::v2::`: 688 files and 2,464 occurrences in the frontend (704 and 2,484 repo-wide). `src/v2` appears as a string in 204 frontend files (940 occurrences).
- `src/v2` holds 1,101 tracked files; `src/` outside it holds only 5.
- **Flattening clashes:**
  - Both `src/README.md` and `src/v2/README.md` exist.
  - Both `src/tests/` and `src/v2/tests/` exist.
  - The crate root would gain a `mod core;`, which makes `use core::…` ambiguous with the built-in `core` crate. Only one real use exists, inside a string literal in `class_r_scrub.rs`, but macros or `use core::` added later would hit it.
- **References from outside the frontend:** `documentation_v2/website` 151 files, other `apps/website` crates 118 (map-engine 86, mostly READMEs), `.ai/tickets` 53.

## 6. Layout gates and what they pin

| Gate | Source | What it pins | If the path disappears |
|---|---|---|---|
| engine-layers | `tools_v2/verification-core/src/repository_laws/engine_layers/rules.rs` (CRATE_REL, MAP_CRATE_REL, EDITING_REL, FRONTEND_REL, DATA_REL, WORLD_REL, SCENARIO_REL), `evaluation.rs`, `report_text.rs` | All `apps/website/{graphics-engine,map-engine,frontend}`, plus `website_*` names in regexes | Fails ("empty root" exits 1) |
| crate dependencies | `tools_v2/verification-core/src/repository_laws/crate_dependencies.rs` | 5 `crate_rel` paths under `apps/website/*` and the forbidden package names | Fails (`NotRun::TargetMissing`) |
| file-length / law source roots | `tools_v2/verification-core/src/repository_laws/source_roots.rs` | `FILE_LENGTH_PINS` (`tools_v2/*`, `apps/website/{api_v2,frontend}/src`, …) and `law_source_roots`, which walks `apps/website/*/{src,tests}` | **Skips silently**: if `apps/website/` is gone, `is_dir()` is false and map-engine, graphics-engine and the offline service worker drop out of the length law unnoticed |
| api engineering laws | `apps/website/api_v2/tests/engineering_laws.rs` (no `architecture_rules.rs` exists) | `apps/website/api_v2/Cargo.toml`, `tools_v2/xtask/src/commands/deploy`, `-p website-api`, `website-map-engine` | — |
| readme-coverage, markdown-placement | `verifications/documentation/{readme_coverage,markdown_placement}.rs` | `CODE_TREES` and `DOCUMENTATION_ROOT` from the layout module | — |
| link-check | `verifications/documentation/link_check/{judged_documents,backticked_paths}.rs` | Layout-module constants | See section 8 |
| enfusion-comments | `verifications/mod_scripts/enfusion_comments/mod.rs:42-48` | `apps/mod/...` only | Not affected |
| route tags | `verifications/architecture/route_tags.rs` | `apps/website/api_v2/src` and `core/http_router.rs` | — |
| editor/orbat coherency | `verifications/architecture/editor_orbat_coherency.rs` | 84 `apps/website/frontend/src/v2/...` file paths | — |
| staging compose paths | `verifications/deployment/staging_compose_paths.rs` | `GOOD_PATH = apps/website/docker-compose.staging.yml`, `BAD_PATH` and a regex on `apps/website/api_v2` | — |
| wave tooling | `commands/platform/wave_execution/` | See below | — |

The wave tooling finds a file's crate dynamically: `changed/changed_rs.rs::owning_package_dir` walks up to the nearest `[package]` `Cargo.toml`. What it hardcodes:
- `changed.rs:13` `FRONTEND_DIR = "apps/website/frontend"`
- `trunk.rs:63,85`
- `schema.rs:48,144-178` (DEM path, `apps/website/map-engine/src` and `Cargo.toml`)
- `-p website-api|website-frontend|website-map-engine` in `gate/gate_dispatch.rs`, `touch.rs:318-352`, `db.rs:306` and `changed_rs.rs:242,333`
- `include_consumer_package_dirs.rs:12` (`["apps","tools_v2"]`)

## 7. Build-artifact folders at the root

- **On disk:** none. There are no `target*` or `dist*` folders at the root or under `apps/website/frontend`.
- **`.gitignore` entries:** `/target/`, `/target-ci/`, `target-*/`, `target-gate-frontend/`, `/target-gate-trunk/`, `/dist-gate-frontend/`, `/apps/website/frontend/dist/`, `/apps/website/frontend/dist-debug/`. It also pins `assets_v2/scratch/`, `assets_v2/terrains/**/tiles/`, `/assets_v2/equipment/`, `/tools_v2/ticket-engine/wip/` and `tools_v2/xtask/deploy/deploy.env`.

**What creates each folder:**

| Folder | Created by |
|---|---|
| `target-gate-trunk`, `dist-gate-frontend`, `target-gate-check`, `target-gate-schema` | `tools_v2/xtask/src/commands/platform/wave_execution/mod.rs:277-292` (override with `TBD_GATE_*`) |
| `target-gate-slice-frontend-<slice>` | `wave_execution/changed/changed_rs.rs:330` |
| `target-dev-api` | `cargo xtask mk rust-api` (`core/cargo_target_directory.rs:71` `DEV_API_TARGET`, `commands/build/recipes.rs`) |
| `target-ci` | reclaimed by `mk reclaim-target-ci` |
| `target-container`, `target-container-api-v2` | **No tool.** These are hand-set `CARGO_TARGET_DIR` values; the latter is named only in docs/briefs (e.g. `documentation_v2/refactor_program_plan.md:193`) |

`reclaim/du_mb.rs` globs `target-*`, `target-gate-*` and `dist-gate-*`. The deploy rsync excludes `target-gate-*/` and `dist-gate-*/`.

## 8. Tickets and frozen docs

- **The ticket store is `.ai/tickets/`:** 1,725 `T-*.toml` files and 1,167 estimates.
- **The ticket engine does check paths:** `tools_v2/ticket-engine/src/validation/references.rs::check_spec_and_plan_files_exist` requires every `spec` and `plan` to exist for any status except `idea` or `cancelled`. **757 such entries point into `documentation_v2/`** (571 spec and 207 plan lines in total), so renaming `documentation_v2` breaks `ticket check --strict` in CI unless they are rewritten. `owns` is only checked for being non-empty (`scope.rs`), and `citations` is not checked for existence. All 88 queued or ready tickets mention renamed paths. `scope-vocab.toml` and `corpus-pins.toml` mention them only in comments.
- **Frozen docs are partly checked by link-check:** `documentation_v2/tickets/` and `documentation_v2/archive/` count as "FrozenDocumentation". Markdown links in them **are** checked; backticked paths and command citations are skipped (`judges()` returns `!is_frozen()`). Links that would need rewriting:

| Area | Files with affected links | Links |
|---|---|---|
| tickets | 64 | 322 |
| archive | 87 | 612 |
| live docs | 398 | 3,904 |
| READMEs outside `documentation_v2` | 720 | 2,679 |

  Any README that moves to a different depth (449 under `apps/website`) also has its relative links shifted.
- **Stale backticked paths can pass silently:** a backticked span counts as a repo path only if its first segment is a *currently tracked* top-level folder. After the rename, a leftover `` `assets_v2/...` `` is skipped instead of flagged, unless it is added to `RETIRED_DOCS_ROOT`/`HISTORICAL_PATH_SPELLINGS`.
- **Precedent:** the earlier docs → `documentation_v2` move left a pin catalogue at `/home/user/TBD-reforger/documentation_v2/refactor_pin_catalogue.md`, with `refactor_move_manifest.tsv` and `refactor_ticket_rewrites.tsv` beside it.

## 9. Runtime paths

- **`/map-assets`:** `apps/website/api_v2/src/core/http_router.rs:109-117` falls back to `../../../assets_v2/terrains` and `../../../assets_v2/glyphs` when `MAP_ASSETS_DIR`/`GLYPH_ASSETS_DIR` are empty. `core/configuration/mod.rs:32,152` holds `DEVELOPMENT_UPLOAD_DIR` and the equipment default (`../../../assets_v2/...`). All of these are relative to the crate folder, so they break on the depth change.
- **systemd** (`tools_v2/xtask/deploy/systemd/tbd-website-api.service`): `WorkingDirectory=…/apps/website/api_v2`, `EnvironmentFile=…/apps/website/api_v2/.env`, `MAP_ASSETS_DIR=…/assets_v2/terrains`, `GLYPH_ASSETS_DIR=…/assets_v2/glyphs`. The backup units cite `tools_v2/...`.
- **`apps/website/api_v2/.env.example`:** 13 lines (`../../../assets_v2/...` defaults).
- **`apps/website/Dockerfile`:** an inline workspace member list (lines 26-28), `COPY apps/website/{api_v2,map-engine,graphics-engine}`, `COPY contracts_v2/definitions`, `COPY contracts_v2/rules/kit-aliases.json`, and `-p website-api`.
- **`apps/website/docker-compose.staging.yml`:** `../../tools_v2/xtask/deploy` and `../../assets_v2/{terrains,glyphs}` mounts, plus `dockerfile: apps/website/Dockerfile`.
- **`Caddyfile.website`:** refers to `apps/website/frontend` mounted at `/srv/tbd-frontend`.
- **rsync excludes** (`commands/deploy/website/rsync_argv.rs:34-60`, `deploy/staging/remote/ssh_argv.rs:29-50`): `apps/website/frontend/dist/`, `apps/website/api_v2/.env`, `apps/website/api_v2/.tools/`, `assets_v2/{terrains,scratch,equipment}/`. `asset_preflight.rs:35` uses `assets_v2/terrains/terrain-registry.json`.
- **`.gitattributes`:** 11 LFS rules on `assets_v2/terrains/**`. Rename these in the same commit as the move.
- **Other config:** `.editorconfig-checker.json` (4 paths), `tools_v2/developer-tools/gate-env.json` (1), `rust-toolchain.toml` (comments only). `Trunk.toml` (only a `/map-assets` URL) and `.cargo/config.toml` (`--package xtask` only) are path-free.
- **Generated EnfScript header:** `apps/mod/tbd-export/.../TBD_GameplayPolicyGenerated.c:1` says "Generated from contracts_v2/rules/equipment-gameplay". The generator (`mod_ops/equipment_gameplay`) and the `verify-codegen-fresh` check would both need regenerating.
