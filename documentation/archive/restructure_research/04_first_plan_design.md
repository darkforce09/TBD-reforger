**Status:** archived — see [the restructure program](/documentation/archive/restructure/README.md)

# First planning pass

# Restructuring program design: TBD-reforger workspace

The blueprint's 12 crates split the map engine by subject: world, overlay, streaming, frame. The code doesn't support that split, because each subject holds CPU code, fetch code and GPU code together. The design below cuts by phase instead: a model layer, then decoding, then queries, then streaming, then rendering. The GPU half of every subject moves into one renderer crate.

## 1. Corrected crate topology

**Verified findings (beyond your list):**
- **`RenderEngine` is one struct.** It is defined at `frame/engine.rs:82` and all its wgpu fields are `pub(crate)` (lines 84–190).
- **The impl blocks sit in about 30 GPU files.** These are `overlay/symbology/instances/bridge_{1,2,3}.rs`, `instances/lanes.rs`, `overlay/lanes_prefs.rs`, `overlay/symbology/atlas/gpu.rs`, `world/environment/{buildings,vegetation}/buffers.rs`, `world/terrain/satellite/textures.rs`, `spatial/los/terrain/overlay.rs`, `camera/viewport.rs`, plus `frame/*` and `diagnostics/*`.
  - **Resolution:** move those files whole into `map_renderer`, which owns `RenderEngine`. This avoids E0116 and needs no extension traits. The `pub(crate)` fields stay crate-private.
- **The streaming→world parsers are really world decoding.**
  - `streaming/loaders/chunk.rs:10-12` needs only `world::environment`.
  - `streaming/loaders/{store,chunk_bin,manifest,prefab}.rs` need only world and io.
  - developer-tools uses exactly these files (`streaming::loaders::{store,chunk,chunk_bin,manifest}`). After the move it needs no streaming crate at all.
- **The world→streaming edges are really fetching halves.** `world/terrain/dem/loader.rs:6-13` and the `*/loader.rs` files under environment use fetch, progress and budget.
- **Small cycles each have a single cause:**
  - `world/environment/locations/{peaks.rs:234, towns.rs:14}` build overlay `LabelSpec`s.
  - `vegetation/canopy.rs:6-8` uses `overlay::lod` (127 lines, no deps).
  - `spatial/indexing/world.rs:10` needs only `NO_CLASS`.
  - `editing/tools/selection/gesture.rs:37` re-exports `frame::EngineHandle` (`frame/mod.rs:148`). This is the editor's only edge into frame.
- **Small GPU helpers:** `frame/boot.rs:458 instance_descriptor` is generic wgpu, and doll uses it. `streaming/bridge/statistics.rs:313 publish_engine` is the only `RenderEngine` use in streaming.
- **Render-side code never names `editing` or `data` outside tests.**
- **Cross-module test fixtures:** `spatial/los/interior/tests/{walker,wash}.rs` import `world::architecture::blueprint::model::tests::*` fixtures.

**Final crates** (`crates/<name>`, package = directory name):

| Tier | Crate | From | Workspace deps |
|---|---|---|---|
| 0 | `newtype_ids` (new) | `string_id!`/`integer_id!` macros (serde transparent, `Borrow<str>`, `Display`) | — |
| 0 | `map_coordinates` | `world/scene.rs` ANCHOR/EVERON_BOUNDS/INITIAL_*/`world_rect_rel`, `camera/math/shaping.rs`, `streaming/scheduler/chunk_math`, `camera/grid_reference.rs` | — |
| 0 | `camera` | `camera/{math,ortho,orbit}` (without viewport) | map_coordinates |
| 0 | `spatial_index` | `spatial/bvh`, `spatial/indexing/{cluster,picking,point_index}` | — |
| 0 | `terrain_formats` | `io/` | — |
| 0 | `graphics_core` | graphics CPU half: text, draw::{compose,geometry,grid,instances,triangulate,cull::oracle}, frame::{camera,damage,ids}, layout, shaders (WGSL text, so the oracle source test stays inside the crate) | — |
| 0 | `browser_console` | `diagnostics/platform/console.rs` (web_sys on wasm32, eprintln on native) | — |
| 0 | `mission_data` | `data/scenario` minus ballistics | newtype_ids |
| 0 | `game_ballistics` | `data/scenario/ballistics` | — |
| 0 | `http_url_guard` | the predicate plus `pub mod cases` (moved from `apps/website/shared/is_http_url_cases.rs`) | — |
| 0 | `offline_cache_policy` | offline-service-worker lib (cache_names, network_fallback, offline_pack, range_slicing, request_classification) | — |
| 1 | `mission_document` | `data/store`, plus the 2 `cfg(feature="store")` scenario round-trip tests | mission_data |
| 1 | `wgpu_backend` | graphics GPU half: device, pipeline, loop, draw::{encode,lines,polygons,cull::compute}, frame::{atlas,batch,buffers,packet,present,text}; `instance_descriptor` | graphics_core |
| 1 | `world_model` | world/{architecture, environment CPU, terrain CPU (dem without loader, relief CPU, roads, water), mesh}; `decoding/` (the 5 streaming parsers); `indexing/world.rs`; `level_of_detail` (overlay/lod.rs) | terrain_formats, spatial_index, map_coordinates, graphics_core (CPU layouts are allowed) |
| 2 | `line_of_sight` | `spatial/los/{terrain (without overlay.rs), interior, world}` | world_model, spatial_index, terrain_formats, map_coordinates |
| 2 | `map_overlay` | overlay CPU: lanes/LaneRole, fire_mission_marks, symbology {roles, labels, links, text_*, markers, atlas CPU, instances/{drag,patches,symbols}}; label builders from peaks/towns | world_model, spatial_index, graphics_core, map_coordinates |
| 3 | `map_streaming` | streaming (without publish_engine) plus the world `*/loader.rs` files | world_model, line_of_sight, map_overlay, terrain_formats, map_coordinates, browser_console |
| 3 | `mission_editor` | `editing/` with the EngineHandle re-export deleted | mission_data, mission_document, line_of_sight, map_overlay, world_model, spatial_index, camera, map_coordinates |
| 4 | `map_renderer` | frame/, diagnostics/, camera/viewport.rs, every `impl RenderEngine` file, satellite textures+quadtree, relief host, scene calibration/stress instances | wgpu_backend, graphics_core, map_streaming, map_overlay, line_of_sight, world_model, camera, map_coordinates, browser_console |
| 4 | `arsenal_preview` | doll/, shaders/doll.wgsl, diagnostics/readback/doll.rs | wgpu_backend, graphics_core, camera |

**Answers to your specific questions:**
- **LOS over world and buildings** goes into its own `line_of_sight` crate above `world_model`. `spatial_index` stays generic.
- **`game_ballistics`** becomes its own crate.
- **`world_model` → `graphics_core`** is allowed. The firewall is wgpu/web-sys, not byte layouts.
- **Benches and readbacks** go to `map_renderer::diagnostics`, because they are `RenderEngine` impls.
- **Fixtures for the LOS tests:** use `world_model::test_fixtures` behind a `test_fixtures` feature. It is the only feature the tier law allows, and only inside `[dev-dependencies]`. This needs your sign-off.
- **`http_url_guard` unifies the two ports.** The two predicate bodies are identical apart from comments. The "port" rationale (`url_guard.rs:13-34`) rejected only a new crate for a 12-line predicate, and a multi-crate workspace removes that cost.
- **Apps** (package = directory name):
  - `apps/{api, frontend, fleet_host_agent, ticketboard, offline_service_worker}`. `offline_service_worker` is now only the thin binary.
  - `fleet_host_agent` depends on no crate (the blueprint was wrong there).
- **Tools:** `tools/{xtask, verification_core, ticket_engine, developer_tools, enfusion_mcp_node_package}`.
  - Tooling libraries stay in `tools/`, not `crates/`.
  - The tier law allows `api` to dev-depend on `verification_core` and `ticketboard` to depend on `ticket_engine`.
- **Frontend:** rename `workspaces/editor/shell` to `workspaces/editor/session`, so the app doesn't have two "shell"s (law 4). `membership_status.rs` moves to `src/shell/`.

## 2. Stage order (one commit each, all on `claude/compassionate-cannon-nz3hts`)

**Why this order:**
- **Renames first.** Every later prompt then cites final paths, and the rewriter is proven on simple moves before the engine carve.
- **Leaves next, before untangling.** `data`, `io`, graphics, camera (without viewport) and bvh are already separable. Carving them first gets `api` off the map engine early, deletes four features, and leaves a smaller strongly connected component to untangle with fewer feature gates to keep alive.
- **The engines stay at `apps/website/{map-engine,graphics-engine}` until carved.** That way `crates/` only ever holds finished crates, and the anatomy law needs no transitional exemption.

| Stage | Content |
|---|---|
| S0 | Tooling and baselines |
| S1 | Top-level renames |
| S2 | App flattening and `deploy/` |
| S3 | Frontend restructure, shared crates, and the new laws |
| S4 | Graphics split |
| S5 | Mission data crates |
| S6 | Geometry leaves |
| S7 | Untangle the strongly connected component in place |
| S8 | Extract the remaining crates and delete the feature matrix |
| S9 | Documentation close |
| M1–M3 | Mod lane, running in parallel after S2 |

## 3. Stage details

**Standard gate set (GS)**, one command per call, each to its own log:
1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets --locked -- -D warnings`
3. `cargo clippy -p frontend --target wasm32-unknown-unknown -- -D warnings`
4. `cargo build -p frontend --target wasm32-unknown-unknown --release`
5. `cargo xtask ci ci-local`
6. `cargo xtask db up` then `cargo xtask db test-it`
7. `cargo xtask ci verify-documentation`
8. `cargo xtask ticket check`
9. A dependency drift probe: the set of external package ids from `cargo metadata` must equal the baseline.

**Operator checkpoint OC-web:** `cargo xtask mk ci-local-leptos` and `mk leptos-gates` on the operator's machine. The container has no trunk or Chrome.

### S0 — tooling and baselines
- **T1 (L): the rewriter.** `cargo xtask refactor relocate --manifest <tsv> --dry-run|--apply|--verify`, with tests.
  - It does `git mv` per row.
  - It rewrites repository-root and `/`-absolute spellings across tracked text, skipping binaries (`.rdb`).
  - It resolves every `../` occurrence (string literals, `#[path]`, `include*!`, Cargo `path =`, markdown links) against the file's old directory and re-relativizes it against the new one.
  - It rewrites crate-scoped Rust paths, e.g. `crate::v2::core::` → `crate::foundation::`, or `crate::world_model::` → `world_model::` outside the crate and `crate::` inside it. It first normalizes `super::` chains to `crate::` paths using a module-tree walker that honours `#[path]`.
  - It leaves frozen-history backticks alone.
  - `--verify` fails on any retired spelling in live files.
- **T2 (M): root manifest and fail-closed roots.**
  - Hoist `[workspace.package]`, `[workspace.dependencies]` and `[workspace.lints]`. Only identical specs are hoisted, so `Cargo.lock` must stay unchanged.
  - Make `source_roots.rs` fail closed: derive the roots from the workspace members, and treat a member without `src/` as `NotRun`.
- **T3 (S): build output under `target/`.** `cargo_target_directory.rs` and `wave_execution` switch to `target/<name>`, and `.gitignore` drops its `target-*`/`dist-*` lines.
- **Waves:** T1 ∥ T2 ∥ T3 (file-disjoint).
- **Gate:** GS plus perturbation proofs for T1 `--verify` and the fail-closed roots.
- **Baselines** go in the program record at `documentation_v2/restructure/`: GS counts, `git lfs ls-files | wc -l`, and the test-binary list.

### S1 — top-level renames
- **What moves:** `assets_v2`, `contracts_v2`, `documentation_v2` (including its mirror directories), `tools_v2` and the tool directories/packages become snake_case.
- **Archived:** `refactor_*` files go to `documentation/archive/refactor_v2/`, and both `improved_layout/` READMEs go to `archive/`.
- **R1 (M), alone:** writes the manifest and runs `relocate --apply`. It also rewrites the 757 ticket spec/plan references in the same pass.
- **Then in parallel, file-disjoint:**
  - **R2 (M):** the repository layout modules, xtask literals, and `engine_layers/rules.rs`/`crate_dependencies.rs` paths.
  - **R3 (S):** `.gitattributes` (11 LFS rules), workflows (`paths:`, lfs include, plus `ci-schema-parity`), `.editorconfig-checker.json`, `gate-env.json`, and regenerating the codegen header.
  - **R4 (M):** link-check RETIRED/HISTORICAL spelling lists and README Contents blocks.
- **Gate:** GS; LFS count equals the baseline; `git lfs fsck --pointers`; `cargo xtask verify codegen-fresh`.

### S2 — app flattening and `deploy/`
- **What moves:**
  - `api_v2` → `apps/api` (package `api`).
  - `frontend` → `apps/frontend`.
  - `offline-service-worker` → `apps/offline_service_worker`.
  - `fleet_host_agent` and `ticketboard` keep their directories; only the package names change.
- **`deploy/` gets:** the Dockerfile, the dev and staging compose files, the Caddyfile, `systemd/`, and `deploy.env.example`.
- **Removed:** api's nested `rust-toolchain.toml`. It differs from the root one and lacks the wasm target.
- **A1 (M), alone:** the relocate run. The api's depth changes from 3 to 2, so its 49 `../../../contracts_v2` references (including the production `mission_default_overrides.rs`) depend on the relative-path rewrite.
- **Then in parallel:**
  - **A2 (M):** xtask deploy, staging and db (`cd apps/api`, seeds), plus `staging_compose_paths.rs`.
  - **A3 (M):** wave_execution (`trunk.rs`, `schema.rs`, `gate_dispatch.rs`, `touch.rs`, `-p` names).
  - **A4 (S):** the Dockerfile member list and COPYs, systemd `WorkingDirectory`/`EnvironmentFile`/`MAP_ASSETS_DIR`, `.env.example`, and the api runtime fallbacks (`http_router.rs:109-117`, `configuration/mod.rs`).
- **Gate:** GS; `docker build -f deploy/Dockerfile .`; `cargo xtask deploy website --dry-run`; OC-web.
- **Operator checkpoint:** move the local gitignored `.env`, `deploy.env` and frontend `dist`, and the server's EnvironmentFile.

### S3 — frontend restructure, shared crates, new laws
- **F1 (M), alone:** relocate `src/v2/*` → `src/{workspaces, foundation, pages}`; `foundation/api` → `foundation/transport`; merge `v2/tests` into `src/tests` and the two READMEs; delete `mod v2;`. Covers the 2,464 `crate::v2::` paths, 305 `#[path]` attributes, and `editor_orbat_coherency.rs`.
- **Then in parallel:**
  - **F2 (M):** extract `src/shell/`; rename the editor's shell to `session`.
  - **F3 (S):** create `foundation/ui/tokens.rs`; add `foundation/auth/logout_hooks.rs`, registered at app boot by `workspaces/editor`.
  - **F5 (L):** `verification_core` laws:
    - **tier DAG:** replaces the forbidden table in `crate_dependencies.rs`.
    - **crate-anatomy:** for every library crate in `crates/` and `tools/`:
      - `lib.rs` is at most 80 lines and holds only docs, attributes, `mod` and `pub use`;
      - `prelude.rs` exists and is declared `pub mod`;
      - `error.rs` has `pub enum Error` deriving `thiserror::Error` plus `pub type Result`;
      - no `anyhow` in `[dependencies]`;
      - README present, package name equals the directory, no `[features]` except `test_fixtures`;
      - no primitive-typed public `id`/`*_id` items (`#[wasm_bindgen]` ABI items are excluded by rule).
    - **frontend layering:** `foundation` may not import `shell`, `pages` or `workspaces`.
- **Then, after F2:**
  - **F4 (M):** create `http_url_guard` and `offline_cache_policy`; the 9 `include!` sites become `use http_url_guard::cases`.
  - **F6 (L):** anatomy for `ticket_engine` (anyhow out; ticketboard updated) and `verification_core`.
- **Gate:** GS; OC-web; perturbation proofs for each new law.

### S4 — graphics split
- **G1 (L), alone:** split graphics-engine into `graphics_core` and `wgpu_backend` (moving `instance_descriptor` in), with their anatomy work; update map-engine imports.
- **G2 (S):** retarget engine-layers rules 1, 2 and 6 at the new crates; rules 3a/3b become "`wgpu_backend` only in the manifests of the renderer-side crates".
- **Waves:** G1 then G2.

### S5 — mission data crates
- **D1 (L):** `mission_data`, `game_ballistics` and `newtype_ids`, consumed by `api` and `frontend`. The `api` manifest drops the map engine.
- **D2 (M):** `mission_document`, after D1.
- **D3 (S):** retire rule 4 (with its pins) and the data side of rule 7 in favour of the tier DAG; delete the `scenario`/`store` features and update the tripwire.

### S6 — geometry leaves
- **L0 (S):** in-place prerequisites. Move `camera/viewport.rs` into `frame/`; move the scene calibration/stress instances into `diagnostics/`; delete the 12 dead `dem/sample` re-exports.
- **Then in parallel:**
  - **L1 (M):** `map_coordinates` and `camera`.
  - **L2 (M):** `spatial_index`, after moving `indexing/world.rs` into `world/`.
  - **L3 (M):** `terrain_formats` and `browser_console`.
- **Shared files:** the orchestrator owns the map-engine `Cargo.toml`/`lib.rs` hunks.
- **Features removed:** `bvh` and `io`.

### S7 — untangle in place
- **Goal:** the top-level modules of `map-engine/src` become exactly the future crates.
- **Movers run sequentially,** because every move rewrites import lines all over the crate. Each finishes with a relocate pass:
  1. **U1 (L):** world decoding, world loaders → streaming, label builders → overlay, lod → world.
  2. **U2 (M):** LOS → the `line_of_sight` module.
  3. **U3 (L):** renderer gather, i.e. every GPU file into `map_renderer/{frame,layers/<subject>,viewport,diagnostics}`.
- **U4 (S):** doll → `arsenal_preview`, run alongside U2.
- **U5 (S), alone:** module renames via relocate.
- **U6 (M):** a module-mode evaluator of the same tier DAG table, sharing the walker with T1. It runs first in ratchet mode, then hard-zero once U5 lands.
- **Gate:** GS plus the module-layer law. Also report how many `pub(crate)` items are used across module boundaries; this is the upper bound on what S8 must widen to `pub`.

### S8 — extract the remaining crates and delete the features
- **X1 (L), alone:** scripted extraction of all 7 crates. Delete `features`, the tripwire, the map engine and `apps/website/`. Rewrite the frontend and developer_tools manifests with no `--all-features`.
- **Anatomy, in tier order** (parallel within a tier only when the consumer files are disjoint):
  1. X2 `world_model` (L)
  2. X3 `line_of_sight` (M) ∥ X4 `map_overlay` (M)
  3. X5 `map_streaming` (M) ∥ X6 `mission_editor` (L)
  4. X7 `map_renderer` (L) ∥ X8 `arsenal_preview` (S)
- **X9 (M):** engine-layers becomes `verify crate-layers`, with ci.yml, `task_definitions.rs` and `ci-schema-parity` updated in the same commit.
- **Gate:** GS; Docker build; deploy dry-run; OC-web plus a live walkthrough.

### S9 — documentation close
- **Rewritten:** CLAUDE.md §1.6 and §2.
- **Replaced:** the blueprint, by `documentation/architecture/workspace_layout.md`; the draft is archived.
- **Done earlier:** `engine_boundary_rules.md` → `crate_boundary_rules.md` is updated section by section in S4–S8 (law 10).
- **Records agent:** closes the record and updates tickets T-1070 and T-1130.

### Mod lane (starts after S2; commits interleave with the other stages)
- **M1 (S): reference folders.** Consolidate into `References/`, covering every tool path and `.gitignore`.
  - Operator checkpoint: move the gitignored folders locally. Git won't move ignored folders.
  - The tooling must fail closed and name the new path.
- **M2 (S): `Objectives/Engine/` grouping.** `git mv` `Model/`, `Registry/`, `Runtime/` and `Tasks/` under `Engine/`, and update the 7 pinned xtask paths.
  - Operator checkpoint: Workbench regenerates the binary `.rdb` (it can't be hand-edited), then compile and boot the world.
- **M3: kind behaviours.**
  - **M3a (S):** an explorer catalogues the 11 switch sites.
  - **M3b (M):** writes `Types/TBD_ObjectiveKindBehaviour.c` plus `Types/{Capture,Destroy,HoldUntil}/`, and the kind→behaviour lookup. NONE maps to a no-op base.
  - **M3c (M) ∥ M3d (M):** M3c rewires `Engine/Registry/`; M3d rewires `Runtime/`, Text, `TBD_ZoneVolume.c` and `TBD_MissionWinConditionChecks.c`.
  - **Frozen names:** only `TBD_ObjectivesComponent` and `TBD_ObjectiveHud` appear in prefabs/configs.
  - **Gate:** `verify enfusion-comments`, `verify file-length`, the xtask mod tests, then an operator checkpoint to compile and playtest each kind.

## 4. How the gates evolve

| Today | After |
|---|---|
| engine-layers rules 1, 3a, 3b, 4, 6, 7 (regex plus pins) | Manifest rules in the tier DAG table (S4/S5/S8). The pins disappear because `RenderEngine` legitimately lives in `map_renderer`. |
| Rule 2 (map nouns) | Kept, retargeted to `graphics_core` and `wgpu_backend`. |
| Rule 5 (no browser in editing) | Kept for `mission_editor`: the manifest bans web-sys, js-sys, wasm-bindgen, gloo and leptos, and the source regex stays. |
| `crate_dependencies.rs` forbidden list | One tier DAG table of allowed edges. Its module mode (S7) is deleted in S8. |
| GPU/browser firewall | Only `wgpu_backend`, `map_renderer` and `arsenal_preview` may take wgpu. The browser-crate allow-list adds `map_streaming`, `browser_console`, and js-sys for `mission_document` (wasm cfg). |
| `feature_gate_tripwire` | Deleted in S8, together with the "no `[features]`" anatomy rule. |
| `source_roots` silently skips missing roots | Fails closed and derives roots from the workspace members (S0). |

## 5. Risks and mitigations

- **`pub(crate)` explosion when splitting.** S7 moves code so items stay inside their future crate, and it measures the cross-module count first. Any `pub` widening in S8 is reviewed as API, and newly public types go into the prelude.
- **Newtype ID ripple.** There are 139 primitive `pub *_id` fields, mostly `String`.
  - `newtype_ids` uses `#[serde(transparent)]`, so the wire format and goldens are unchanged.
  - `Borrow<str>` keeps map lookups working.
  - Conversion runs bottom-up per crate.
- **Source-text tests across crates.** The draw-order tests (about 33 includes each) move to `map_renderer`. The wash_palette test that reads the relief host moves to `map_renderer` as well. Any cross-crate `include_str!` that remains is handled by the rewriter's relative mode and caught by `cargo test`.
- **Gitignored local state doesn't follow `git mv`.** This covers `.env`, `deploy.env`, `References/` and dist folders. xtask detects stale old paths and prints a move instruction, and each case is an operator checkpoint.
- **Server-side paths.** systemd units and the EnvironmentFile change in S2. The operator reviews the dry-run before the first real deploy.
- **LFS.** Rules and moves go in the same commit, and the count and `fsck` are gated.
- **The `.rdb` and Workbench.** Moves inside `TBD/` are Workbench checkpoints. Addon directory names (`tbd-framework`) stay as they are, because `.gproj` and the server `-addonsDir` bind them; this needs your confirmation.
- **wasm32 builds.** Run `cargo check -p <crate> --target wasm32-unknown-unknown` for every crate the frontend links, and keep the trunk build in OC-web.
- **Cargo.lock churn.** Only workspace package renames are expected; the drift probe catches any external version change.
- **`.sqlx`.** Nothing to move: no `query!` macros are used.
- **Frontend edition 2021.** Keep it as a per-crate override. An edition bump isn't part of this program.
- **Conflicts with `CLAUDE.md` law 2.** "Direct to main" conflicts with your decision 1. The brief must say that decision 1 overrides it for this program.

**Open questions for you:**
1. May `world_model` carry a `test_fixtures` dev-only feature?
2. Should the addon directories stay kebab-case?
3. Should trunk be installed in the container, or does OC-web stay an operator checkpoint?

### Critical files for implementation
- /home/user/TBD-reforger/apps/website/map-engine/src/frame/engine.rs
- /home/user/TBD-reforger/tools_v2/verification-core/src/repository_laws/crate_dependencies.rs
- /home/user/TBD-reforger/tools_v2/verification-core/src/repository_laws/source_roots.rs
- /home/user/TBD-reforger/tools_v2/verification-core/src/repository_laws/engine_layers/rules.rs
- /home/user/TBD-reforger/tools_v2/xtask/src/core/repository_layout.rs
