**Status:** live

# Restructure program plan

The plan of the program that rebuilds the workspace from the foundations up: what exists now,
what research verified, the binding decisions, the execution model, the stages with their agents,
and the risks. The goal tree is in
[target_file_tree.md](/documentation_v2/restructure/target_file_tree.md), the crates in
[crate_catalogue.md](/documentation_v2/restructure/crate_catalogue.md), the gates in
[laws_and_gates.md](/documentation_v2/restructure/laws_and_gates.md), and live state in
[progress.md](/documentation_v2/restructure/progress.md).

Abbreviations in code spans: `me/` = `apps/website/map-engine/src/`, `ge/` =
`apps/website/graphics-engine/src/`, `api/` = `apps/website/api_v2/src/`, `fe/` =
`apps/website/frontend/src/v2/`, `xt/` = `tools_v2/xtask/src/`, `dt/` =
`tools_v2/developer-tools/src/`, `te/` = `tools_v2/ticket-engine/src/`, `vc/` =
`tools_v2/verification-core/src/`.

## 1. Context

About 690k lines of Rust live in 11 crates. The map engine (127k lines) draws its boundaries with
a nine-feature Cargo matrix instead of crates. The API (172k), the frontend (167k) and xtask
(115k) are monoliths, and transition names (`_v2`, `src/v2`) run through the tree. The operator's
[architecture blueprint](/documentation_v2/archive/restructure_research/00_architecture_blueprint_draft.md) set the direction: a
Zed/Rerun-style workspace with a flat `apps/`, many crates and per-crate standards. The operator
then widened it to a rebuild from the foundations up, with far more than twelve crates. Code moves
and is refactored into the new design; it is never retyped.

## 2. Verified findings

Each finding was checked against the code. The evidence is in the
[restructure research](/documentation_v2/archive/restructure_research/README.md).

1. **Map engine cycle.** `data` and `io` are leaves and `editing` is a sink.
   `{camera, frame, overlay, world, spatial, streaming, diagnostics, doll}` form one strongly
   connected component. About 18 re-export shim modules fake part of it; `me/world/terrain/dem/sample/`
   re-exports 21 line-of-sight items backwards. The real causes:
   - `RenderEngine` (`me/frame/engine.rs:82`, 76 fields, 27 of them counters) has inherent impls
     in six modules;
   - loaders are split between world and streaming;
   - labels ↔ locations;
   - `WorldChunk` sits above its users;
   - a scheduler, buffers and loaders three-way cycle;
   - console macros in diagnostics.
2. **Unused JS exports.** The map engine's 143 `#[wasm_bindgen]` attributes are exports no
   JavaScript calls. Every caller is Rust, through `EngineHandle` (`me/frame/mod.rs:148`) and
   `DollEngine`. `me/frame/boot.rs:442` is a start hook that duplicates the frontend's panic hook.
   The real users that stay are the offline service worker's lifecycle exports and one start hook
   in the editor bridge.
3. **Feature consumers.** The API uses only `data::scenario`. developer-tools uses world, io,
   spatial, streaming and scenario natively. The frontend uses everything. `earcutr` is unused in
   the map engine only.
4. **Mission data already points one way.** The mission chain (ast → extensions, payload,
   validator, flatten) and the ballistics chain (model ← solver ← planning ← agreement cases;
   calibration → model and solver) have no back-edges. Placement geometry is pure. The mission
   document needs only the CRDT layer in production. The two operations halves depend on each
   other and stay one crate. `COMPILER_PACKAGE_VERSION` is stored in the database.
5. **API: nine domain cycles.** All nine come from about 1.5k lines of shared services: audit,
   user stats and leaderboard view, machine caller identity, session authorization, participation
   and reevaluation, the ORBAT template parser, and `TerrainType`. `AppState` owns three domain
   services. `fail_point!` resolves through `$crate::core`. 154 integration binaries each keep
   their own database. Zero `query!` macros exist, so `SQLX_OFFLINE` in CI is vestigial.
6. **Frontend inversions.** These edges point the wrong way:
   - the api↔auth and auth↔ui cycles;
   - core → editor (UI tokens, logout purge);
   - core → crate root (route guard);
   - pages → editor (review workspace, byte formatting);
   - administration → mission review.

   The 45k-line editor holds about 78 back-edge references between its folders. Tailwind scans one
   source glob. The frontend is on edition 2021, the rest on 2024. The frontend DTOs hold 141
   primitive-typed IDs, the map engine 135, and the API 152 `Uuid` IDs.
7. **Tools.** Repository-root discovery exists seven times and path layout four times. 113 raw
   `Command::new` calls bypass the process runner. anyhow sits in library APIs. xtask links tokio,
   axum, reqwest and resvg through developer-tools. Dependency versions drift (sha2, toml, png),
   and the root manifest has no workspace dependencies or lints.
8. **Path coupling.** About 6,000 of 16,209 tracked files spell a path that moves:
   - 757 ticket spec and plan entries, existence-checked by the ticket engine;
   - 357 cross-crate relative paths;
   - runtime path fallbacks, systemd, Docker, compose, Caddy, rsync and LFS rules.

   The file-length law's root walker skips missing roots silently.
9. **Website improved layout.** Valid: deploy consolidation, shell extraction, transport rename,
   the inversions. Invalid: a shared DTO crate replacing the frontend DTOs (they differ on
   purpose), merging the API test binaries, a cruft folder that does not exist, and several counts.
10. **Mod improved layout.**
    - `Scripts/Game/TBD/` is the namespace that keeps our files from shadowing vanilla's, so it
      stays.
    - The proposed objective type classes do not exist, and there is no HVT kind.
    - Kind dispatch is enum switches in 11 files.
    - Only `TBD_ObjectivesComponent` and `TBD_ObjectiveHud` are bound by assets.
11. **Container tooling.** Chromium sits under `/opt/pw-browsers`, and the gate harness needs
    `CHROME_HEADLESS_SHELL` pointed at it. trunk 0.21.14 and wasm-bindgen-cli 0.2.126 must be
    installed.

## 3. Binding decisions (operator, 2026-10-01 and 2026-10-02)

| Id | Decision |
|---|---|
| D1 | Commits go to the session branch `claude/compassionate-cannon-nz3hts`, one green commit per stage; the operator merges to `main`. For this program this overrides CLAUDE.md law 2. |
| D2 | Full standards as each crate is born: thin `lib.rs` and `prelude`, `thiserror` `Error` and `Result` (fallible public API only), newtype IDs at boundaries, anyhow only in binaries. |
| D3 | Crate folders and package names are snake_case with no organisation prefix; descriptive qualifiers (`api_`, `frontend_`, `mission_creator_`) are allowed. |
| D4 | Mod: no missions addon; `Scripts/Game/TBD/` kept; a References folder; Objectives Engine and real `TBD_ObjectiveKindBehaviour` subclasses (Capture, Destroy, HoldUntil); bound class names frozen. |
| D5 | Frontend: `src/v2` removed; apps → workspaces, core → foundation, api → transport; a shell; inversions fixed. |
| D6 | One root deploy folder: Dockerfile, both compose files, Caddyfile, systemd units, deploy environment example. |
| D7 | Rejected: a shared DTO crate replacing the frontend DTOs (a serde-only contract crate for byte-identical shapes such as fleet commands is allowed); merging API test binaries; a missions addon. |
| D8 | Dev-only features only for `test_fixtures` and `failpoints`; addon folders stay kebab-case; trunk, wasm-bindgen-cli and the Chromium variable are set up in the container. |
| D9 | About 148 workspace members, accepted as designed. |
| D10 | Strip the unused `#[wasm_bindgen]` exports; no JS facade crate; proven by a JS-glue export diff. |
| D11 | Extracted frontend crates move to edition 2024 in their own wave. |
| D12 | The dying monoliths park in a top-level legacy folder from S2; no new crate may depend on it. |
| D13 | The parking folder keeps the name `legacy/`; the repository's prose rule exempts that folder name (the path segment and identifiers naming the folder), while the word stays banned as history vocabulary. |
| D14 | Tests may call `unwrap()`: a root `clippy.toml` sets `allow-unwrap-in-tests`; production code under the workspace lint policy still may not. |

## 4. Execution model: foundations up

New crates are born tier by tier from the bottom, at their final path, already refactored to
their design and to the standards. A new crate never depends on a legacy crate. Each stage runs
four steps:

1. **Cut wave.** Back-edges that would make a new crate need legacy code are cut in place.
2. **Birth wave.** xtask refactor relocate moves the files. Legacy keeps `pub use` shims at the
   old paths, logged in a shim ledger. Every widened `pub(crate)` item is listed for API review.
3. **Switch wave.** One scripted agent rewrites every consumer's paths from the ledger and deletes
   every shim.
4. **Commit.** The strangler law requires an empty ledger, so no shim survives a commit.

Legacy crates shrink to nothing: the website folder goes in S2, the graphics and map engines in
S8, the ticket engine in S11.

## 5. Stages

Agent budgets: S = 150k, M = 250k, L = 350k tokens. "∥" marks agents that run in parallel.
Mechanical renames always run as one scripted agent. A gate that pins paths changes in the same
commit as the paths. The gate set GS is defined in
[laws_and_gates.md](/documentation_v2/restructure/laws_and_gates.md#standard-gate-set).

| Stage | Agents and content | Extra gates, checkpoints |
|---|---|---|
| P0 Program documents | Orchestrator: this folder, the research archive (with the blueprint draft), pointers in CLAUDE.md, AGENTS.md and the documentation README. | `cargo xtask ci verify-documentation` |
| S0 Tooling | Orchestrator first: `git fetch --unshallow`, so permalinks resolve and the link check measures real breaks. **T1** (L) xtask refactor relocate: `git mv`, path spellings, every `../` re-relativised against its anchor, `#[path]`, `include*!`, Cargo `path =`, markdown links, and Rust paths via a module walker; `--verify` fails on retired spellings. ∥ **T2** (M) all manifests: workspace package, dependencies and lints, hoisting identical specs only; xtask drops `png` and makes `toml` a dev-dependency; `earcutr` removed from the map engine. ∥ **T3** (M) build output under `target/<name>`; trunk and wasm-bindgen-cli installed; `find_chromium` honours `PLAYWRIGHT_BROWSERS_PATH`. ∥ **T4** (L) the new laws, plus fail-closed source roots. ∥ **T5** (S) `SQLX_OFFLINE` and the `rust-sqlx-prepare` recipe removed. | Perturbation proof per law; baselines recorded in progress.md |
| S1 Global renames | **R1** (M, alone):<br>• assets, contracts, documentation and tools folders renamed;<br>• tool packages renamed to snake_case;<br>• the ticket engine parked in legacy;<br>• the old refactor records and the improved_layout docs archived;<br>• the 757 ticket spec and plan rewrites.<br>Then **R2** (M) layout modules and xtask literals ∥ **R3** (S) LFS rules, workflows, editorconfig, gate environment, codegen header ∥ **R4** (M) link-check retired spellings, READMEs, CLAUDE.md, AGENTS.md, the workspace layout doc, and the documentation mirror rule. | LFS count equals the baseline; `git lfs fsck`; codegen freshness |
| S2 Apps and deploy | **A1** (M, alone):<br>• api, frontend and service worker under `apps/`;<br>• map and graphics engines parked in legacy;<br>• snake_case packages;<br>• deploy folder;<br>• the API's nested toolchain and rustfmt files removed;<br>• API goldens moved to contracts.<br>Then **A2** (M) xtask deploy, staging and db ∥ **A3** (M) wave execution ∥ **A4** (S) Dockerfile, systemd, env example, API runtime fallbacks, Caddy mount ∥ **A5** (M) `http_url_guard` and `offline_cache_policy` born. | Docker build; deploy dry runs; **OC-deploy**: the operator moves the gitignored env files and the server environment file |
| M1, M2 Mod | **M1** (S) the References folder and every tool path to it, failing closed. **M2** (S) `Objectives/Engine/` grouping and the seven pinned xtask paths. | **OC-mod**: move the ignored folders; Workbench regenerates `resourceDatabase.rdb`; compile; world boot |
| S3 Frontend in place | **F1** (M, alone) the `src/v2` split into foundation, pages and workspaces; api → transport. Then **F2** (M) shell extraction, editor shell → session ∥ **F3** (S) UI tokens, logout hooks, route table ∥ **F4** (S) review workspace, byte formatting, mission review feature. | frontend layering at zero |
| S4 Tier 0–1 | **B0** (S) cuts. Then **B1** (L) tool foundations ∥ **B2** (M) foundation crates ∥ **B3** (M) geometry ∥ **B4** (M) render primitives ∥ **B5** (S) world file formats ∥ **B6** (M) contract crates. Then **X4** (M, alone) switch. **B7** (S) engine rules retargeted. | xtask tree: no duplicated sha2 |
| S5 Mission and ballistics | **D1** (L) five authoring crates ∥ **D2** (M) five ballistics crates. Then **D3** (L) CRDT, document, formation geometry, operations. **X5** (M) switch. **D4** (S) retire the data rules; delete `scenario` and `store`. | API tree: no map engine, yrs, wgpu, rkyv, png |
| S6 World CPU | **P0c** (L, alone) cuts. Then **P1** (M) spatial, prefab, chunks ∥ **P2** (L) overlay CPU ∥ **P3** (L) elevation, relief, satellite, roads. Then **P4** (M) water, vegetation, interiors. Then **P5** (M) place names, world store. Then **P6** (L) line of sight. **X6** (M) switch. | `bvh` and `io` features deleted |
| S7 Streaming CPU and editor | **Q0** (M) cuts. **Q1** (M) scheduler and draw buffers ∥ **Q2** (L) editing session, commands, persistence. Then **Q3** (M) editing tools. **X7** (M) switch. | `editing` feature deleted |
| S8 Rendering | **V0** (L, alone):<br>• strip the exports;<br>• `RenderStats`, `LaneSink`, typed layers, `FrameHook`, `GpuContext`.<br>**V1** (L) GPU device, frame and core. Then **V2** (M) symbology layers ∥ **V3** (M) paper doll ∥ **V4** (L) asset loading and streaming host. Then **V5** (M) world layers. Then **V6** (L) renderer and diagnostics. **X8** (M) switch; legacy engines deleted. **V7** (S) engine layer rules deleted. | **OC-web**: walkthrough online and with the API stopped behind the proxy |
| S9 API | **K0** (L) kernel cuts. **K1** (M) infrastructure crates. **K2** (M) state and kernel crates. Then **K3a–h** (M) eight domains. **K4** (M) workers, thin app, staging fixtures to tools. | Integration count at or above baseline; Docker build |
| S10 Frontend crates | **H0** (L) editor untangle with a module ratchet. **H0b** (S) pins, `@source` lines, test support. **H1** (L) foundation and feature crates. Then **H2a–g** (S/M) seven page crates ∥ **H3** (L) six workspace crates. **H4** (M) thin app. **H5** (M) edition 2024. | Walkthrough again |
| S11 Tools | **J0** (M) tool cycles. **J1** (L) ticket crates; legacy ticket engine deleted. Then **J2a–f** (M) command and check crates ∥ **J3a–d** (M) developer-tools crates. **J4** (S) thin binaries; anyhow out of every library. | xtask tree: no tokio, axum, reqwest, resvg; legacy empty |
| S12 Close | **G1…** closing fixes. CLAUDE.md §1.6 and §2 rewritten; crate boundary rules closed; tree diffed against the target; this folder archived; pointers removed. | Final sweep, perturbation proofs, walkthrough |

**M3 objective behaviours** (any time after M2):
- **M3a** (S) explorer: catalogue the 11 switch sites.
- **M3b** (M): the behaviour base class and the Capture, Destroy and HoldUntil types with a kind lookup; NONE maps to a no-op base.
- **M3c ∥ M3d** (M): M3c rewires Registry; M3d rewires Runtime, the objective text, the zone volume and the win-condition checks.
- Gates: `cargo xtask verify enfusion-comments` and `cargo xtask verify file-length`, then an operator playtest of each kind.

The per-stage detail each agent prompt needs (exact files, cuts and edges) is in
[crate_catalogue.md](/documentation_v2/restructure/crate_catalogue.md) and the research reports.

## 6. Shared brief essentials

The orchestrator writes these into the session brief before the first launch of each stage.

- **Git:** agents never commit, stage, branch, stash or reset.
- **Files:** each agent edits only the files it owns. Shared registration lines (legacy `lib.rs`,
  `mod.rs` and manifests, README Contents blocks) are own-lines-only.
- **Code:** every crate compiles at every save point. Laws 7, 8 and 10 apply. Tests are never
  weakened.
- **Proof:** every new check gets a
  [perturbation proof](/documentation_v2/glossary/n_to_z.md#perturbation-proof).
- **Commands:** one command per shell call, each to its own log. Export `CHROME_HEADLESS_SHELL`
  to the container's Chromium. All builds share one target folder.
- **Moves:** every relocation goes through xtask refactor relocate; paths are never rewritten by
  hand.
- **Reports:** at most 250 words, in the runbook's six-section format.

## 7. Risks

| Risk | Mitigation |
|---|---|
| `pub(crate)` widening (frame 122, streaming 85, API 152, frontend 732) | Each birth lists widened items, reviewed as API and placed in the prelude |
| Newtype ID ripple | serde- and sqlx-transparent newtypes keep goldens byte-equal; converted bottom-up |
| The editor tangle resists the split | Ratchet, state lift, injected callbacks; any pair still cyclic merges, and the merge is recorded |
| Edition 2024 drop-order changes | Own wave, gated by the browser gates |
| Dev-only features unified by `--all-targets` | Release checks of the API and the frontend in GS |
| Tailwind classes lost when code moves | The tailwind sources law |
| Gitignored state, LFS, `.rdb`, server paths | OC-deploy and OC-mod checkpoints, LFS gates, fail-closed path messages |
| A stored version string changes | `COMPILER_PACKAGE_VERSION` pinned as a literal |
| Program length (13 stages, about 100 agents) | One green commit per stage; any stage boundary is a resume point; progress.md pushed at every wave boundary |
