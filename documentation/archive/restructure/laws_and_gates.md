**Status:** archived — see [crate boundary rules](/documentation/standards/crate_boundary_rules.md)

# Laws and gates

The repository laws the program adds, the gate set every stage passes, how today's gates evolve,
and the checks that prove the end state. Planned xtask subcommands appear without the `cargo`
prefix until the stage that builds them lands.

## New laws

S0 builds each of these in the repository-laws library, wires it into `cargo xtask ci ci-local`,
`.github/workflows/ci.yml` and the CI schema parity check, and proves it with a
[perturbation proof](/documentation/glossary/n_to_z.md#perturbation-proof). Laws that the
current tree cannot pass yet start in ratchet mode and turn hard at the stage named.

### Crate tiers (`cargo xtask verify crate-tiers`)

1. Every manifest under the apps, crates and tools folders (outside test trees, fixtures and build
   output) is a workspace member. The root `members` list reaches the library crates
   through one glob per category (`crates/<category>/*`, and `crates/frontend/*/*` for the
   frontend's layer folders), so a manifest under `crates/` that no glob reaches fails this rule.
2. Each judged member declares `[package.metadata.layout]` with `category`, `tier` and `targets`;
   every member outside the judged set is an app under `apps/` or one of the two tool binaries
   (`tools/xtask`, `tools/developer_tools`), and any other is a finding.
3. A crate's path is its category plus its name, and the package name equals the folder name.
4. Dependency edges point strictly down, and each declared tier equals 1 plus the highest
   dependency tier.
5. The category edge matrix holds:
   - foundation depends on foundation;
   - contracts depend on foundation and contracts;
   - mission depends on foundation, mission and any `crates/geometry` crate;
   - ballistics depends on foundation and ballistics;
   - graphics depends on foundation and graphics;
   - the other engine categories depend on foundation, contracts and any engine category,
     graphics included;
   - rendering depends on foundation, contracts, mission, ballistics, rendering and any engine
     category;
   - mission editing depends on foundation crates whose targets are not wasm32 only, and on
     mission, mission editing, geometry, world formats, terrain, world objects, line of sight and
     overlay crates (never ballistics, streaming or graphics);
   - api depends on foundation, contracts, mission, ballistics and api;
   - frontend depends on anything except api and tools;
   - tools depend on foundation, contracts, mission, ballistics, tools and engine crates whose
     targets are `any`, never on a wasm-only crate, and on api only from `staging_fixtures`.
6. Firewalls:
   - wgpu only in the GPU device, frame and core crates and in the rendering and paper doll
     renderer crates;
   - web-sys, js-sys, wasm-bindgen and gloo only in wasm-only crates, `time_source` (behind a
     cfg) and the frontend;
   - sqlx and axum only in api crates, with one category clause: axum is also allowed in
     crates whose category is `tools/browser_testing` or `tools/staging`, because those are test
     and staging harness servers (the gate's static server, the staging relay), not product code;
     sqlx also in `tools/staging/staging_fixtures`, the staging host tool (coordinator decision,
     operator review pending), the one tool the crate-tier law already lets depend on api crates,
     which seeds and cleans staging rows directly; no other crate has an exception;
   - leptos only in frontend crates;
   - no tokio, axum, reqwest, resvg or image in the dependency closure of xtask (this ban stays
     hard and keeps the harness servers out of xtask; the closure is walked from the xtask
     binary although it sits outside the judged `tools/<category>/<name>` layout);
   - no map nouns in the graphics category: no declaration keyword followed by a name holding
     `terrain`, `symbology`, `mission`, `orbat` or `arma` in a graphics crate's sources;
   - no browser crates in mission editing: no browser crate edge, and no `web_sys`, `leptos` or
     `wasm_bindgen` token, prose included, in any `.rs` file under `crates/mission_editing/`; the
     scan fails closed when that folder holds no `.rs` file (it replaced engine rule 5 in S7);
   - no `#[wasm_bindgen]` attribute (plain, path-qualified or under `cfg_attr`) in any `.rs` file
     of a workspace member outside `apps/frontend/`, `crates/foundation/browser_platform/` and
     `apps/offline_service_worker/`; the scan fails closed when it walks no `.rs` file (S8).
7. Dev-dependencies are exempt from the tier order but never point at apps.

The live statement of these rules, the matrix and the firewalls is
[crate boundary rules](/documentation/standards/crate_boundary_rules.md#5-the-workspace-laws).

### Crate anatomy (`cargo xtask verify crate-anatomy`)

For every library crate:
- `lib.rs` is at most 80 lines and holds only the module header, attributes, `mod` and `pub use`.
- A `pub mod prelude` exists.
- `error.rs` holds a `thiserror` `Error` and a `Result` when the public API is fallible.
- No anyhow dependency. Binaries are exempt.
- A README with a Contents block.
- Edition, rust-version, lints and every dependency come from the workspace.
- The only features are the dev-only `test_fixtures` and `failpoints`, each enabled only from
  `[dev-dependencies]`.
- No primitive-typed public `id` or `*_id` field or parameter. Generated code is exempt.
- No `pub use` of another workspace crate outside `prelude.rs`. This is what ends re-export
  shims.

### Other laws

- **Strangler**: retired in S12 with the parking folder it guarded; its subcommand, rule and
  tests are deleted.
- **Test-file reachability** (`cargo xtask verify test-file-reachability`, S12): every `.rs` file
  in a member's test folders is loaded by one of its targets.
- **Frontend layering** (`cargo xtask verify frontend-layering`), hard at zero, in two modes:
  - crate-edge mode: every normal, dev and build dependency edge between frontend crates is
    judged. A crate's layer is its folder `crates/frontend/<layer>/` (foundation below features
    below pages and workspaces) and the app `apps/frontend` is the shell; a lower layer never
    depends on a higher one, page crates never depend on each other, and pages and workspaces
    never depend on each other. The foundation crates keep their order: `frontend_ui` <
    `frontend_api_dtos` < {`frontend_transport`, `frontend_route_table`} < `frontend_session` <
    {`frontend_offline`, `frontend_map_view`}, with `frontend_test_support` reached only through
    dev-dependencies. The Mission Creator crates form the order `mission_creator_state` <
    `mission_creator_engine_bridge` < `mission_creator_session` < `mission_creator_arsenal` <
    `mission_creator_workspace`; `debug_benches` is an order of its own that no Mission Creator
    crate touches. A frontend crate outside every layer folder or order, and a crate an order
    names that no member carries, are findings;
  - in-crate mode: the module-level rules that remain inside one crate, through a layer table the
    caller passes; the app's table maps its entry point, route rendering, platform frame and
    tests onto the shell.
- **Tailwind sources** (`cargo xtask verify tailwind-sources`): the app's stylesheet holds exactly
  one `@source "<relative path>/src/**/*.rs";` line per workspace member that depends on leptos,
  the app included; a missing line and a stale line (naming no such member) are findings.
  Trunk's `[watch]` list covers `crates/frontend`. Hard from S10.
- **Relocation** (`cargo xtask refactor relocate --verify`): no retired spelling in a live file.
- **Fail-closed roots:** the file-length and law root walkers derive their roots from the
  workspace members and fail on a missing root instead of skipping it.

## Standard gate set

Two tiers (decision D20, which supersedes D15 and D19). The **stage gate** runs at the end of every stage and holds only fast checks: formatting, clippy `-D warnings` and the unit tests of every crate the stage touched, the relocation verify, the documentation gates, and a stage's one-second specifics (for example `cargo xtask mod compile`). The **full gate set GS** below runs once, at S12, and before a real deploy.

GS is run between waves and before every stage commit, one command per shell call, each into its
own log:

1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets --locked -- -D warnings`
3. wasm32 clippy over the frontend app, every frontend crate and every wasm-only crate
4. `cargo xtask ci ci-local`
5. `cargo xtask db up` then `cargo xtask db test-it`
6. `cargo xtask mk ci-local-leptos`
7. `cargo xtask mk leptos-gates`
8. `cargo xtask ci verify-documentation`
9. `cargo xtask ticket check`
10. the new laws: crate tiers, crate anatomy, test-file reachability, frontend layering, tailwind
    sources
11. `cargo xtask refactor relocate --verify`
12. Dependency drift probe: the external dependency set equals the baseline plus the stage's
    declared unifications.
13. Test-name census at or above the S0 baseline, so a moved test is counted, not lost.
14. A release check of the API package and a wasm release check of the frontend, so dev-only
    features cannot mask production builds.
15. A wasm distribution size probe.

Stage-specific extras are in the stage table of the
[program plan](/documentation/archive/restructure/program_plan.md#5-stages). Examples:
`cargo xtask deploy website --dry-run`, `cargo xtask deploy staging --dry-run`, a Docker build of
the deploy Dockerfile, and the operator-run `cargo xtask mod compile` and
`cargo xtask mod world-boot`.

## Gate evolution

| Today | After the program |
|---|---|
| Engine layer rules 1, 3a, 3b, 6 and 7, with source pins | Manifest firewalls and category edges of the crate tiers law; deleted with the legacy engines (done in S8) |
| Engine layer rule 2 (no map nouns in graphics) | Kept as a source regex over the graphics category in the crate firewalls; `verify engine-layers` deleted (done in S8) |
| The map engine's UI-framework ban | Deleted with the map engine; the crate firewalls keep leptos in frontend crates (done in S8) |
| No check on JavaScript exports | The crate firewalls' `#[wasm_bindgen]` source scan, failing closed on an empty walk (done in S8) |
| Engine layer rule 5 (no browser crates in editing), retired in S7 with the editing module | The narrowed mission editing matrix arm plus the crate firewalls' source scan over `crates/mission_editing/` (done in S7) |
| The forbidden-edge list in `tools/foundation/repository_laws/src/crate_dependencies.rs` | The allowed-edge category matrix and tier numbers for the library crates; the list keeps the three website applications, which carry no layout table (narrowed in S8) |
| Source roots skip missing folders silently | Fail-closed roots derived from the workspace members (S0) |
| The map engine's feature gate tripwire test | Replaced by the anatomy law's features rule; deleted with the map engine (done in S8) |
| No standards gate | Crate anatomy, test-file reachability, frontend layering, tailwind sources |
| 84 frontend paths pinned by the editor ORBAT coherency check | Rewritten by the relocation tool at every move |
| Frontend layering over the module paths of the single frontend crate | The crate-edge mode over the dependency edges between the frontend crates; the in-crate mode keeps the app shell's table (done in S10) |
| One ancestor `@source` glob over the frontend sources | One exact `@source` line per leptos member, a stale line a finding (done in S10) |

## End-state verification

The program closes when all of these hold, with logs named in
[progress.md](/documentation/archive/restructure/progress.md):

- **Gates:** GS is green, with the test census at or above the baseline. The tests deleted are the
  feature-gate tripwire, which a law replaces, the 36 engine-layer tests and two engineering-laws
  tests deleted with the rules they tested (F-S8-18), and the strangler law's tests deleted with
  that law. The API keeps 154 integration binaries: 150 in `apps/api/tests/` and the 4 database
  suites of `tools/staging/staging_fixtures`.
- **File tree:** the tree matches
  [target_file_tree.md](/documentation/archive/restructure/target_file_tree.md), checked by the close
  stage's tree diff.
- **Workspace shape:** 160 members (the 158 of S11, plus `crates/foundation/repository_root` and
  `tools/commands/agent_context_guards` in S12): 5 apps, 112 library crates, 41 tool crates and the
  two tool binaries; the 153 judged crates pass crate tiers and crate anatomy. The legacy and
  website folders are gone. The only features left in the library crates are the two dev-only
  ones (the ticketboard app keeps its `glow` feature).
- **Retired names:** `git grep` finds no retired spelling in live files: `_v2`, `src/v2`, the
  `website-` package prefix, or the website folder path.
- **Duplicates:** the repository-root marker, the deterministic generator, the clock, the SHA-256
  helper and glyph packing are each defined once.
- **JS exports:** `#[wasm_bindgen]` appears only in the frontend app (its start function),
  `browser_platform` and the offline service worker (the crate firewalls' scan,
  `cargo xtask verify crate-tiers`).
- **Dependency trees:**
  - the API's tree has no yrs, wgpu, rkyv, png or leptos;
  - xtask's tree has no tokio, axum, reqwest or resvg;
  - the frontend's wasm build has no sqlx or tokio.
- **Builds:** the Docker build passes, and so do `cargo xtask deploy website --dry-run` and
  `cargo xtask deploy staging --dry-run`.
- **Browser gates:** `cargo xtask mk leptos-gates` passes, plus a live walkthrough of the
  [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator), the offline mortar page
  and event slotting, online and with the API stopped behind the proxy.
- **Mod:** the operator compiles the mod, boots the world, and playtests capture, destroy and
  hold-until objectives.
- **Perturbation proofs:** every new law has one, recorded in the execution log.
