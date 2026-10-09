**Status:** archived — see [the restructure program](/documentation/archive/restructure/README.md)

# First plan draft

## Context

The repo grew around a handful of large crates under `apps/website/`. The biggest is `website-map-engine` (~127k lines), which uses a 9-feature Cargo matrix (`scenario, store, bvh, world, io, streaming, render, editing` plus `default`) instead of real crate boundaries. Transitional `_v2`/`v2` names are also spread through the tree (`assets_v2/`, `contracts_v2/`, `documentation_v2/`, `tools_v2/`, `api_v2/`, `frontend/src/v2/`). The operator's draft `documentation_v2/architecture_blueprint.md` (written with Gemini from a surface scan) sets the direction: a Zed/Rerun-style workspace with a flat `apps/` of runnables, `crates/` of single-purpose libraries, no `_v2` names, and per-crate code standards (thin `lib.rs` + `prelude.rs`, a `thiserror` `Error` per crate, newtype IDs at crate boundaries). The two `improved_layout` plans (website, mod) add layout polish. The code is moved and refactored, not rewritten.

Research checked the blueprint and both improved_layout plans against the code. Their direction holds, but several specifics are wrong. The corrections below are binding on the program.

## Verified findings (what the code really is)

**F1. map-engine is one dependency cycle, not twelve separable modules.**
- `data` and `io` are leaves.
- `editing` is a sink.
- `{camera, frame, overlay, world, spatial, streaming, diagnostics, doll}` form ONE strongly connected component. There are 10 two-way back-edges, e.g. `frame<->world` through `world::scene::ANCHOR` and `world/*/buffers.rs` importing `frame`; `spatial<->world`; `spatial<->streaming` through `WorldChunk`; `streaming<->world`. Console macros in `diagnostics::platform::console` also create a cycle.
- `impl RenderEngine` blocks (type at `map-engine/src/frame/engine.rs:82`) are spread over frame 40, overlay 42, diagnostics 19, world 13, camera 10 and spatial 2. That is illegal across crates (E0116).
- So the blueprint's extraction stages cannot run as written. An in-crate untangling stage must come first.

**F2. Feature consumers.**
- `website-api` uses only `data::scenario`.
- `developer-tools` (native) uses world/io/spatial/streaming/scenario.
- The frontend uses everything.
- `data::scenario::ballistics` (14.5k lines) imports nothing else in `scenario`, and nothing in `scenario` imports it. It is a natural crate of its own.
- `earcutr` is declared in map-engine but never used.

**F3. graphics-engine has a clean CPU/GPU seam.** Two frictions:
- `draw::{encode,lines,polygons}` import `frame::{batch,buffers,packet,text}`.
- `frame/atlas.rs` imports `text::pack`.

**F4. Path coupling: about 6,000 of 16,209 tracked files spell a path that will be renamed.**
- `.ai/tickets/` has 757 spec/plan entries pointing into `documentation_v2/`. They are existence-checked by `tools_v2/ticket-engine/src/validation/references.rs`.
- Layout constants are centralised in:
  - `tools_v2/xtask/src/core/repository_layout.rs`
  - `tools_v2/ticket-engine/src/repository.rs`
  - `tools_v2/developer-tools/src/repository_layout.rs`
  - `tools_v2/verification-core/src/repository_laws/source_roots.rs`
- About 70 literals in xtask sit outside those modules. The worst is `verifications/architecture/editor_orbat_coherency.rs`, with 84 frontend paths.
- 357 cross-crate relative paths break on any move, mostly `../map-engine` and `../../../contracts_v2`. Moving `api_v2` from depth 3 to depth 2 breaks every `../../../X`.
- Runtime paths also break: the api's `/map-assets` fallback (`core/http_router.rs:109-117`), `.env.example`, the systemd unit, the Dockerfile, compose files, the Caddyfile, rsync excludes and `.gitattributes` LFS rules.
- `source_roots.rs` **silently skips** missing roots, so the file-length law would quietly stop covering moved crates.

**F5. Website improved_layout plan. Valid parts:**
- deploy consolidation
- `shell/` extraction
- `core/api` → `transport` rename
- the two inverted dependencies (`core/ui/{slider,select,search_box}.rs` import editor tokens; `core/auth/store.rs:259` calls the editor purge)
- `shared/` needs a home

**F6. Website improved_layout plan. Invalid parts:**
- Deleting the frontend DTOs for a shared `website-api-types` crate. The DTOs deliberately differ: string enums, `flatten` catch-alls, tri-state patches. The api models carry `sqlx` derives.
- Merging the api integration tests. There are 154 binaries, not 95, and each binary has its own database plus process-global failpoints.
- The `map_engine/` cruft directory does not exist.
- Several counts are wrong.

**F7. Mod improved_layout plan.**
- The `TBD/` folder is the anti-shadow namespace against vanilla, not redundant.
- The proposed objective `Types/{Capture,Destroy,HoldUntil,HVT}` classes do not exist, and there is no HVT kind.
- Objective kind dispatch is `TBD_EObjectiveKind` switches in 11 files:
  - `Gamemode/Objectives/{Registry,Runtime,Model}/*`
  - `Systems/Zones/Volumes/TBD_ZoneVolume.c`
  - `Systems/Mission/Loaders/Validation/TBD_MissionWinConditionChecks.c`
- `resourceDatabase.rdb` holds 213 `Scripts/Game/TBD` path entries.

## Binding decisions (operator, 2026-10-01)

- **D1.** Commits go to `claude/compassionate-cannon-nz3hts` and are pushed; the operator merges them into main. There is one green commit per stage.
- **D2.** Full code standards are applied per crate as it is carved, and a new gate enforces them.
- **D3.** Directories and package names are pure snake_case with no prefix: `crates/mission_data`, `package.name = "mission_data"`, imports `mission_data::`.
- **D4.** Mod:
  - The `tbd-missions` addon is dropped.
  - `Scripts/Game/TBD/` is kept.
  - `apps/mod/References/` is created.
  - Objectives become `Engine/` plus real per-kind classes: `TBD_ObjectiveKindBehaviour` with `Types/Capture`, `Types/Destroy` and `Types/HoldUntil`. Class names that prefabs and configs bind to stay frozen.
- **D5.** Frontend:
  - `src/v2/` is removed.
  - `apps` → `src/workspaces/`
  - `core` → `src/foundation/`
  - `foundation/api` → `foundation/transport`
  - New `src/shell/`
  - The two inverted dependencies are fixed.
- **D6.** One root `deploy/` holds the Dockerfile, the dev and staging compose files, the Caddyfile, the systemd units and `deploy.env.example`.
- **D7.** Rejected: the shared api-types crate replacing the DTOs, merging the api test binaries, and the `tbd-missions` addon.

- **D8.** Defaults taken from the second review (the operator can override any of them at approval):
  - `world_model` may carry one dev-only `test_fixtures` feature, used only from `[dev-dependencies]`. It is the only feature the new crate-anatomy law allows.
  - Enfusion addon directories (`tbd-framework`, `tbd-export`, `tbd-emcp`) stay kebab-case, because `.gproj` and the server's `-addonsDir` bind them. Snake_case applies to Rust crates.
  - Trunk is installed in the container in S0 (crates.io is reachable and Chromium is preinstalled), so the frontend gates run here. Anything that still cannot run here becomes operator checkpoint OC-web.
  - For this program D1 overrides CLAUDE.md law 2, and the shared brief says so.

## Corrections to the blueprint (C1…C10)

- **C1. Cut the engine by phase, not by subject.** The blueprint splits map-engine into world, overlay, streaming and frame. Each of those modules mixes CPU, fetch and GPU code, which is what makes F1 a cycle. The cut is model → decoding → queries → streaming → rendering. The GPU half of every subject goes into one `map_renderer` crate, and that crate owns `RenderEngine` (fields stay crate-private, which avoids E0116).
- **C2. Line of sight gets its own crate.** It sits above `world_model`, because LOS reads buildings, DEM and world chunks. `spatial` stays generic and becomes `spatial_index` (bvh, cluster, picking, point index).
- **C3. The streaming parsers are world decoding.** `streaming/loaders/{store,chunk,chunk_bin,manifest,prefab}.rs` move into `world_model::decoding`, so developer_tools no longer needs a streaming crate. The world `*/loader.rs` fetch halves move into `map_streaming`.
- **C4. New tier-0 crates the blueprint missed:**
  - `map_coordinates`: ANCHOR, bounds, chunk math, rounding, grid reference.
  - `camera`: without the viewport.
  - `browser_console`: the console macros.
  - `newtype_ids`: ID macros.
  - `game_ballistics`.
  - `http_url_guard`: one predicate plus the shared case table. The two ports have identical bodies.
  - `offline_cache_policy`: the service worker's library half.
- **C5. Fleet host agent edge.** `fleet_host_agent` depends on no engine crate; the blueprint drew an edge that does not exist.
- **C6. `world_model` may depend on `graphics_core`.** The firewall is wgpu/web-sys, not CPU byte layouts.
- **C7. Untangle first, then extract.** S7 untangles inside the one crate, and S8 extracts mechanically.
- **C8. Leaves come out first.** Graphics, data, io, camera and bvh are carved before the untangle, which takes `api` off the map engine early and removes 4 features before the hard part.
- **C9. Gates get new targets.**
  - "Move root `target-*` into `target/`" means changing the tools that create those folders. None exist on disk.
  - `source_roots.rs` must fail closed.
- **C10. Archived, not deleted.** The improved_layout docs are corrected and archived. The blueprint is replaced by a living `documentation/architecture/workspace_layout.md`, and the draft goes to the archive.

## Target layout

```text
apps/      api · frontend · fleet_host_agent · ticketboard · offline_service_worker (thin bin) · mod/
crates/    see topology table
tools/     xtask · verification_core · ticket_engine · developer_tools · enfusion_mcp_node_package
deploy/    Dockerfile · compose.dev.yml · compose.staging.yml · Caddyfile · systemd/ · deploy.env.example
assets/ · contracts/ · documentation/ · .ai/tickets/
apps/frontend/src/   main.rs · app_routes.rs · router.rs · shell/ · pages/ · workspaces/ · foundation/{auth,transport,ui,map_view,offline,utils} · tests/
apps/mod/            References/ (gitignored) · tbd-framework · tbd-export · tbd-emcp
  …/Gamemode/Objectives/   Engine/{Model,Registry,Runtime,Tasks} · Types/{TBD_ObjectiveKindBehaviour.c, Capture/, Destroy/, HoldUntil/}
```

### Crate topology (package = directory name; edges point downward only)

| Tier | Crate | From | Depends on (workspace) |
|---|---|---|---|
| 0 | `newtype_ids` | new: `string_id!`/`integer_id!` (serde transparent, `Borrow<str>`, `Display`) | — |
| 0 | `map_coordinates` | `world/scene.rs` consts, `camera/math/shaping.rs`, `streaming/scheduler/chunk_math`, `camera/grid_reference.rs` | — |
| 0 | `camera` | `camera/{math,ortho,orbit}` | map_coordinates |
| 0 | `spatial_index` | `spatial/bvh`, `spatial/indexing/{cluster,picking,point_index}` | — |
| 0 | `terrain_formats` | `io/` | — |
| 0 | `graphics_core` | graphics CPU half: text, draw::{compose,geometry,grid,instances,triangulate,cull::oracle}, frame::{camera,damage,ids}, layout, shaders (WGSL text) | — |
| 0 | `browser_console` | `diagnostics/platform/console.rs` | — |
| 0 | `mission_data` | `data/scenario` minus ballistics | newtype_ids |
| 0 | `game_ballistics` | `data/scenario/ballistics` | — |
| 0 | `http_url_guard` | both `is_http_url` ports + `apps/website/shared/is_http_url_cases.rs` as `cases` | — |
| 0 | `offline_cache_policy` | offline-service-worker lib modules | — |
| 1 | `mission_document` | `data/store` (yrs CRDT) | mission_data |
| 1 | `wgpu_backend` | graphics GPU half: device, pipeline, loop, draw::{encode,lines,polygons,cull::compute}, frame::{atlas,batch,buffers,packet,present,text}, `frame/boot.rs` `instance_descriptor` | graphics_core |
| 1 | `world_model` | world architecture/environment/terrain CPU, mesh, `decoding/` (C3), `indexing/world.rs`, `overlay/lod.rs` as `level_of_detail` | terrain_formats, spatial_index, map_coordinates, graphics_core |
| 2 | `line_of_sight` | `spatial/los/{terrain (minus overlay.rs), interior, world}` | world_model, spatial_index, terrain_formats, map_coordinates |
| 2 | `map_overlay` | overlay CPU: lanes/LaneRole, fire_mission_marks, symbology CPU, label builders from peaks/towns | world_model, spatial_index, graphics_core, map_coordinates |
| 3 | `map_streaming` | streaming (minus `publish_engine`) + world `*/loader.rs` | world_model, line_of_sight, map_overlay, terrain_formats, map_coordinates, browser_console |
| 3 | `mission_editor` | `editing/` (the `EngineHandle` re-export at `tools/selection/gesture.rs:37` deleted) | mission_data, mission_document, line_of_sight, map_overlay, world_model, spatial_index, camera, map_coordinates |
| 4 | `map_renderer` | frame/, diagnostics/, camera/viewport.rs, every `impl RenderEngine` file, satellite textures/quadtree, relief host | wgpu_backend, graphics_core, map_streaming, map_overlay, line_of_sight, world_model, camera, map_coordinates, browser_console |
| 4 | `arsenal_preview` | doll/, shaders/doll.wgsl, diagnostics/readback/doll.rs | wgpu_backend, graphics_core, camera |

App edges:
- `api` → mission_data, game_ballistics, http_url_guard (it has no map engine at all after S5).
- `frontend` → the tier 3–4 crates plus mission_*, game_ballistics, http_url_guard and offline_cache_policy.
- `developer_tools` → world_model, terrain_formats, line_of_sight, spatial_index and mission_data, all native.
- `fleet_host_agent` and `ticketboard` → none. `ticketboard` keeps its dependency on `ticket_engine`.

## Stages (one green commit per stage, pushed to the session branch)

Each stage runs as the orchestration runbook prescribes (`documentation_v2/runbooks/sub_agent_orchestration.md`): a shared brief, one prompt file per agent, file-disjoint agents, a gate between waves, then closing fixes. A mechanical rename always runs as one scripted agent and never in parallel. Gates that pin paths change in the same commit as the paths they pin.

**Standard gate set (GS)**, one command per call, each to its own log:
1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets --locked -- -D warnings`
3. `cargo clippy -p frontend --target wasm32-unknown-unknown -- -D warnings`
4. `cargo xtask ci ci-local`
5. `cargo xtask db up` then `cargo xtask db test-it`
6. `cargo xtask mk ci-local-leptos`
7. `cargo xtask ci verify-documentation`
8. `cargo xtask ticket check`
9. Dependency-drift probe: the external package set from `cargo metadata` must equal the baseline.

**OC-web:** `cargo xtask mk leptos-gates` plus a live walkthrough. It runs here if trunk and Chromium work; otherwise it goes to the operator.

### S0: tooling and baselines
- **T1 (L): `cargo xtask refactor relocate --manifest <tsv> --dry-run|--apply|--verify`.**
  - `git mv` per manifest row.
  - Rewrites root-relative path spellings in all tracked text (`.rdb` and other binaries skipped).
  - Re-relativizes every `../` in string literals, `#[path]`, `include*!`, Cargo `path =` and markdown links, against each file's old and new directory.
  - Rewrites Rust module paths (`crate::v2::core::` → `crate::foundation::`; `crate::world_model::` → `world_model::` outside the crate) through a module-tree walker that honours `#[path]`.
  - Leaves backticks in frozen docs alone.
  - `--verify` fails on any retired spelling in live files.
  - Precedent: `documentation_v2/refactor_move_manifest.tsv` and `refactor_pin_catalogue.md`.
- **T2 (M): workspace manifest and fail-closed roots.**
  - Root `[workspace.package]`, `[workspace.dependencies]` and `[workspace.lints]`, hoisting identical specs only; `Cargo.lock` must stay unchanged.
  - `tools_v2/verification-core/src/repository_laws/source_roots.rs` derives its roots from the workspace members and fails closed on a missing one.
- **T3 (S): build output under `target/`.** `core/cargo_target_directory.rs` and `wave_execution/mod.rs` write into `target/<name>`; `.gitignore` drops the `target-*`/`dist-*` lines; trunk is installed.
- T1, T2 and T3 run in parallel.
- **Gate:** GS, plus perturbation proofs for `relocate --verify` and for fail-closed roots.
- **Baselines** go in the new program record `documentation_v2/restructure/progress_checkpoint.md`: GS counts, `git lfs ls-files | wc -l`, and the test-binary list.

### S1: top-level renames
- `assets_v2` → `assets`, `contracts_v2` → `contracts`, `documentation_v2` → `documentation`, `tools_v2` → `tools`. The tool folders and packages become snake_case.
- `documentation_v2/refactor_*` move to `documentation/archive/refactor_v2/`.
- The improved_layout docs and both code-tree anchor READMEs are archived. The website and mod plans are corrected by this program.
- **R1 (M), alone:** writes the manifest and runs relocate. The same pass rewrites the 757 ticket `spec`/`plan` entries and the `documentation_v2/{assets_v2,contracts_v2,tools_v2}` mirror folders.
- **Then in parallel:**
  - **R2 (M):** the four layout modules, the remaining xtask literals, and the paths in `engine_layers/rules.rs` and `crate_dependencies.rs`.
  - **R3 (S):** `.gitattributes` (11 LFS rules), the five workflows (`paths:`, LFS include, ci-schema-parity), `.editorconfig-checker.json`, `gate-env.json`, and regenerating the generated EnfScript header.
  - **R4 (M):** link-check's RETIRED/HISTORICAL spellings, so stale backticks fail; README Contents blocks; CLAUDE.md/AGENTS.md path lines.
- **Gate:** GS; LFS count equals the baseline; `git lfs fsck --pointers`; `cargo xtask ci verify-codegen-fresh`.

### S2: flat apps and `deploy/`
- **Moves:** `api_v2` → `apps/api`; `frontend` → `apps/frontend`; `offline-service-worker` → `apps/offline_service_worker`. Packages: `api`, `frontend`, `fleet_host_agent`, `ticketboard`. The deployment files go to `deploy/` (D6). The api's nested `rust-toolchain.toml` is removed.
- **A1 (M), alone:** relocate. The api's depth changes from 3 to 2, so its 49 `../../../contracts_v2` references (including production `missions/handlers/mission_default_overrides.rs`) get re-relativized.
- **Then in parallel:**
  - **A2 (M):** xtask deploy, staging and db. The `cd apps/api` and the seed paths in `commands/db/operations.rs` and `selftest.rs` change; `verifications/deployment/staging_compose_paths.rs` too.
  - **A3 (M):** `wave_execution` (`changed.rs` FRONTEND_DIR, `trunk.rs`, `schema.rs`, `gate_dispatch.rs`, `touch.rs`, every `-p` name).
  - **A4 (S):** the Dockerfile member list and COPYs; systemd `WorkingDirectory`/`EnvironmentFile`/`MAP_ASSETS_DIR`; `.env.example`; the api runtime fallbacks (`core/http_router.rs:109-117`, `core/configuration/mod.rs`); the Caddy mount.
- **Gate:** GS; `docker build -f deploy/Dockerfile .`; `cargo xtask deploy website --dry-run`; OC-web.
- **Operator checkpoint:** move the gitignored `.env`, `deploy.env` and `dist`, and the server's EnvironmentFile. xtask detects stale old paths and names the new ones.

### S3: frontend restructure, shared crates, new laws
- **F1 (M), alone:** relocate `src/v2/*` → `src/{workspaces,foundation,pages}` and `foundation/api` → `foundation/transport`.
  - Merges `v2/tests` into `src/tests` and the two READMEs, and deletes `mod v2;`.
  - Covers 2,464 `crate::v2::` paths, 305 `#[path]` attributes, and the 84 paths in `editor_orbat_coherency.rs`.
- **Then in parallel:**
  - **F2 (M):** extracts `src/shell/` (`layout.rs`, `sidebar.rs`, `top_nav.rs`, `nav_config.rs`, `membership_status.rs`); `not_found` stays a page. Renames `workspaces/editor/shell` → `workspaces/editor/session`, so "shell" means one thing.
  - **F3 (S):** creates `foundation/ui/tokens.rs` (`DISABLED_GLYPH`, `HOVER_FILL`), and `foundation/auth/logout_hooks.rs` with the editor registering its purge at boot. That replaces `store.rs:259`.
  - **F5 (L):** new `verification_core` laws, each with a perturbation proof:
    - **tier-dag:** an allowed-edge table that replaces the forbidden list in `crate_dependencies.rs`.
    - **crate-anatomy:** for every library crate in `crates/` and `tools/`:
      - `lib.rs` is at most 80 lines and holds only docs, attributes, `mod` and `pub use`;
      - a `pub mod prelude` exists;
      - `error.rs` has a `thiserror` `pub enum Error` and `pub type Result`;
      - there is no `anyhow` dependency;
      - a README exists, and the package name equals the directory;
      - there is no `[features]` table, except `test_fixtures`;
      - no primitive-typed public `*_id` exists outside `#[wasm_bindgen]`.
    - **frontend-layering:** `foundation` never imports `shell`, `pages` or `workspaces`.
- **After F2:**
  - **F4 (M):** creates `crates/http_url_guard` and `crates/offline_cache_policy`; the 9 `include!` sites become `use http_url_guard::cases`.
  - **F6 (L):** brings `ticket_engine` and `verification_core` up to anatomy; `anyhow` leaves the library crates.
- **Gate:** GS; OC-web.

### S4: graphics split
- **G1 (L), alone:** `graphics_core` and `wgpu_backend`, with full anatomy.
  - Resolves the `draw::encode` → `frame::batch` friction by moving encode-side types into `wgpu_backend`.
  - Updates map-engine imports.
- **G2 (S):** retargets engine-layers rules 1, 2 and 6. Rules 3a/3b become manifest rules: wgpu is allowed only in `wgpu_backend`, `map_renderer` and `arsenal_preview`.

### S5: mission data
- **D1 (L):** `newtype_ids`, `mission_data` and `game_ballistics`, with full anatomy.
  - IDs use serde-transparent newtypes, so the wire format and goldens are unchanged.
  - `api` drops `website-map-engine`.
- **D2 (M), after D1:** `mission_document`.
- **D3 (S):** retires rule 4, its `RULE4_PIN`s and the data half of rule 7, in favour of tier-dag; deletes the `scenario` and `store` features.

### S6: geometry leaves
- **L0 (S), in place:** moves `camera/viewport.rs` into `frame/`; moves the scene calibration/stress instances into `diagnostics/`; deletes the 12 dead re-exports in `world/terrain/dem/sample/mod.rs`; drops the unused `earcutr`.
- **Then in parallel, with the orchestrator owning the map-engine `Cargo.toml`/`lib.rs` hunks:**
  - **L1 (M):** `map_coordinates` and `camera`.
  - **L2 (M):** `spatial_index`, after `indexing/world.rs` moves into world.
  - **L3 (M):** `terrain_formats` and `browser_console`.
- Deletes the `bvh` and `io` features.

### S7: untangle the cycle in place
- **Goal:** the top-level modules of map-engine become exactly the future crates.
- **Movers run in sequence,** because each one rewrites imports across the whole crate. Each ends with a relocate pass:
  1. **U1 (L):** world decoding (C3), world loaders → streaming, label builders → overlay, `lod` → world.
  2. **U2 (M):** LOS becomes a `line_of_sight` module. U4 (S) runs alongside it: doll → `arsenal_preview`.
  3. **U3 (L):** renderer gather. Every `impl RenderEngine` and GPU file goes into `map_renderer/{frame,layers/<subject>,viewport,diagnostics}`, together with the draw-order source-text tests and `wash_palette`.
  4. **U5 (S):** module renames.
- **U6 (M):** a module-mode evaluator of the same tier-dag table. It runs in ratchet mode from the start and becomes hard-zero after U5, so cycles cannot come back.
- **Gate:** GS plus the module law. Also record the number of cross-module `pub(crate)` uses; that count is the ceiling on `pub` widening in S8.

### S8: extract and delete the feature matrix
- **X1 (L), alone:** a scripted extraction of the 7 crates.
  - Deletes `[features]`, `src/tests/feature_gate_tripwire.rs`, the map-engine and graphics-engine directories, and `apps/website/`.
  - Rewrites the frontend and developer_tools manifests.
  - Removes `--all-features` from `ci/task_definitions.rs`, `gate_dispatch.rs` and `touch.rs`.
- **Anatomy runs in tier order:** X2 `world_model` (L) → X3 `line_of_sight` ∥ X4 `map_overlay` → X5 `map_streaming` ∥ X6 `mission_editor` (L) → X7 `map_renderer` (L) ∥ X8 `arsenal_preview`. Newly public items are reviewed as API and go into the prelude.
- **X9 (M):** engine-layers becomes `cargo xtask verify crate-layers`; ci.yml, `task_definitions.rs` and ci-schema-parity change in the same commit. The module-mode law is deleted.
- **Gate:** GS; `cargo check --target wasm32-unknown-unknown` for every crate the frontend links; Docker build; deploy dry-run; OC-web with a walkthrough online and with the API stopped behind the proxy.

### S9: documentation close
- `documentation/standards/engine_boundary_rules.md` becomes `crate_boundary_rules.md`. It is updated section by section in S4–S8 (law 10) and closed here.
- CLAUDE.md §1.6 (boundary layers) and §2 (atlas) are rewritten.
- `documentation/architecture/workspace_layout.md` replaces the blueprint; the draft is archived.
- The records agent closes the program record and files every NOTE finding as a ticket.

### Mod lane (M1–M3; starts after S2 and interleaves its commits)
- **M1 (S): `apps/mod/References/`** for `crf_framework`, `vanilla_reference` and `playable_selector`.
  - Updates `.gitignore`, `verifications/licensing/upstream_code_leaks.rs:92`, `platform/slice_worktree/git_plain.rs`, `fetch/vanilla_{api,source}.rs`, `enfusion_tooling/{carve,cli,mod}.rs`, the rsync excludes, `staging/remote.rs:25`, `source_roots.rs:36` and `ui_layouts.rs:32`.
  - The tools fail closed and name the new path.
  - Operator checkpoint: move the ignored folders locally.
- **M2 (S): `git mv`** of `Objectives/{Model,Registry,Runtime,Tasks}` → `Objectives/Engine/`.
  - Updates the 7 pinned xtask paths: `mod_scripts/{results_reporter_identity_comments,mission_rest_size_limits,ui_layouts,destroy_target_diagnostics,player_identity_comments}.rs` and `schemas/checks/{mission_validation,contract_validation/validate_all}.rs`.
  - Operator checkpoint: Workbench regenerates `resourceDatabase.rdb`, then `cargo xtask mod compile` and `mod world-boot`.
- **M3: kind behaviours.**
  - **M3a (S), explorer:** catalogues the 11 switch sites.
  - **M3b (M):** writes `Types/TBD_ObjectiveKindBehaviour.c` and `Types/{Capture,Destroy,HoldUntil}/`, with a kind → behaviour lookup. NONE is the no-op base.
  - **Then M3c ∥ M3d (M):** M3c rewires `Engine/Registry/`; M3d rewires `Runtime/`, `Model/TBD_ObjectiveText.c`, `Systems/Zones/Volumes/TBD_ZoneVolume.c` and `Systems/Mission/Loaders/Validation/TBD_MissionWinConditionChecks.c`.
  - **Frozen names:** `TBD_ObjectivesComponent` and `TBD_ObjectiveHud`, which prefabs and configs bind to.
  - **Gate:** `cargo xtask verify enfusion-comments`, `verify file-length` and the xtask mod tests, then an operator checkpoint to compile, boot the world, and playtest one objective of each kind.

## Gate evolution

| Today | After |
|---|---|
| engine-layers rules 1/3a/3b/4/6/7 (regexes and pins) | manifest rules in the tier-dag table. The pins go, because `RenderEngine` legitimately lives in `map_renderer`. |
| rule 2 (no map nouns in graphics) | kept, retargeted to `graphics_core` and `wgpu_backend` |
| rule 5 (no browser crates in editing) | kept for `mission_editor` (manifest ban plus the source regex) |
| `crate_dependencies.rs` forbidden list | one allowed-edge tier DAG |
| `feature_gate_tripwire` | deleted; the "no `[features]`" anatomy rule replaces it |
| `source_roots` silently skips missing roots | fails closed; roots come from the workspace members |

## Risks

- **`pub(crate)` explosion.** S7 moves code so items stay inside their future crate, and its measured count caps S8's widening.
- **Newtype ID ripple** (about 139 primitive `pub *_id` fields). Serde-transparent IDs plus `Borrow<str>` keep the wire format and map lookups unchanged. Conversion runs bottom-up per crate.
- **Gitignored local state does not follow `git mv`.** This covers `.env`, `deploy.env`, `References/` and `dist`. xtask detects stale paths and names the move; each case is an operator checkpoint.
- **Server paths.** systemd and the EnvironmentFile change in S2. The operator reviews `deploy website --dry-run` before the first real deploy.
- **LFS.** Rules move in the same commit as the files; the LFS count and `fsck` are gated.
- **`.rdb` and Workbench.** Only the operator can regenerate them, so M2/M3 cannot close without the Windows checkpoint.
- **Cross-crate `include_str!` source tests.** They move with their targets in U3. The relocate tool's relative mode handles the rest, and `cargo test` catches misses.
- **Scale.** About 6,000 files touched in S1–S3. The relocate tool's `--verify` and the fail-closed roots are the safety net. Never hand-edit.
- **No surprises from these:** no `.sqlx` offline data (0 `query!` macros); the frontend stays on edition 2021.

## Verification (end state)

- **GS green**, with test counts at or above the S0 baseline. No test is deleted except `feature_gate_tripwire`, which is replaced by a law.
- **Removed:**
  - `cargo metadata` shows no `[features]` on any workspace crate except `world_model/test_fixtures`.
  - `api`'s dependency tree has no wgpu, png, rkyv or flate2.
  - `git grep -nE '_v2|src/v2|crate::v2|website-(api|frontend|map-engine|graphics-engine)'` finds nothing in live files.
- **New gates** each have a recorded perturbation proof: `cargo xtask verify crate-layers`, `crate-anatomy`, `frontend-layering` and `refactor relocate --verify`.
- **Builds:** `docker build -f deploy/Dockerfile .`, `cargo xtask deploy website --dry-run` and `cargo xtask deploy staging --dry-run` succeed.
- **Live walkthrough:** the Mission Creator, the mortar page offline, and the event slotting page.
- **Mod:** operator-run `mod compile`, `mod world-boot`, and a playtest of the capture, destroy and hold-until objectives.
