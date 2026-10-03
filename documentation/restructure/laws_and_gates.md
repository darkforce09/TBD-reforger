**Status:** live

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

1. Every manifest under the apps, crates, tools and legacy folders is a workspace member.
2. Each member declares `[package.metadata.layout]` with `category`, `tier` and `targets`.
3. A crate's path is its category plus its name, and the package name equals the folder name.
4. Dependency edges point strictly down, and each declared tier equals 1 plus the highest
   dependency tier.
5. The category edge matrix holds:
   - foundation depends on nothing;
   - contracts depend on foundation;
   - mission and ballistics depend on foundation and `map_coordinates`;
   - engine CPU categories depend on lower engine categories and the graphics CPU primitives;
   - rendering may depend on any engine crate;
   - mission editing depends on foundation crates whose targets are not wasm32 only, and on
     mission, mission editing, geometry, world formats, terrain, world objects, line of sight and
     overlay crates (never ballistics, streaming or graphics);
   - api depends on foundation, contracts, mission, ballistics and api;
   - frontend depends on anything except api;
   - tools never depend on wasm-only, api or frontend crates, except that `staging_fixtures` may
     depend on api.
6. Firewalls:
   - wgpu only in the GPU device, frame and core crates and in the rendering and paper doll
     renderer crates;
   - web-sys, js-sys, wasm-bindgen and gloo only in wasm-only crates, `time_source` (behind a
     cfg) and the frontend;
   - sqlx and axum only in api crates, with one category clause: axum (never sqlx) is also
     allowed in crates whose category is `tools/browser_testing` or `tools/staging`, because
     those are test and staging harness servers (the gate's static server, the staging relay),
     not product code; there is no per-crate allowlist;
   - leptos only in frontend crates;
   - no tokio, axum, reqwest, resvg or image in the dependency closure of xtask (this ban stays
     hard and keeps the harness servers out of xtask);
   - no map nouns in the graphics category (today's engine rule 2);
   - no browser crates in mission editing: no browser crate edge, and no `web_sys`, `leptos` or
     `wasm_bindgen` token, prose included, in any `.rs` file under `crates/mission_editing/`; the
     scan fails closed when that folder holds no `.rs` file (it replaced engine rule 5 in S7).
7. Nothing outside the legacy folder depends on it, apart from apps and the two tool binaries
   while legacy exists.
8. Dev-dependencies are exempt from the tier order but never point at apps or legacy.

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

- **Strangler** (`cargo xtask verify strangler`): the shim ledger is empty at commit, and no new crate
  depends on legacy.
- **Frontend layering** (`cargo xtask verify frontend-layering`): foundation does not import features,
  pages, workspaces or the app shell; features do not import pages or workspaces; inside foundation
  a sub-area imports only sub-areas before it in the order ui, utils, transport, route_table, auth,
  then offline and map_view as peers (test_support only from test files). Hard from S3.
- **Tailwind sources** (`cargo xtask verify tailwind-sources`): every frontend crate has an `@source`
  line in the app's stylesheet. Hard from S10.
- **Relocation** (`cargo xtask refactor relocate --verify`): no retired spelling in a live file.
- **Fail-closed roots:** the file-length and law root walkers derive their roots from the
  workspace members and fail on a missing root instead of skipping it.

## Standard gate set

Two tiers (decision D20, which supersedes D15 and D19). The **stage gate** runs at the end of every stage and holds only fast checks: formatting, clippy `-D warnings` and the unit tests of every crate the stage touched, the relocation verify, the documentation gates, and a stage's one-second specifics (for example `cargo xtask mod compile`). The **full gate set GS** below runs once, at S12, and before a real deploy.

GS is run between waves and before every stage commit, one command per shell call, each into its
own log:

1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets --locked -- -D warnings`
3. wasm32 clippy over the frontend and every wasm-only crate
4. `cargo xtask ci ci-local`
5. `cargo xtask db up` then `cargo xtask db test-it`
6. `cargo xtask mk ci-local-leptos`
7. `cargo xtask mk leptos-gates`
8. `cargo xtask ci verify-documentation`
9. `cargo xtask ticket check`
10. the new laws: crate tiers, crate anatomy, strangler, frontend layering, tailwind sources
11. `cargo xtask refactor relocate --verify`
12. Dependency drift probe: the external dependency set equals the baseline plus the stage's
    declared unifications.
13. Test-name census at or above the S0 baseline, so a moved test is counted, not lost.
14. A release check of the API package and a wasm release check of the frontend, so dev-only
    features cannot mask production builds.
15. A wasm distribution size probe.

Stage-specific extras are in the stage table of the
[program plan](/documentation/restructure/program_plan.md#5-stages). Examples:
`cargo xtask deploy website --dry-run`, `cargo xtask deploy staging --dry-run`, a Docker build of
the deploy Dockerfile, and the operator-run `cargo xtask mod compile` and
`cargo xtask mod world-boot`.

## Gate evolution

| Today | After the program |
|---|---|
| Engine layer rules 1, 3a, 3b, 6 and 7, with source pins | Manifest firewalls and category edges of the crate tiers law; deleted with the legacy engines in S8 |
| Engine layer rule 2 (no map nouns in graphics) | Kept as a source regex over the graphics category |
| Engine layer rule 5 (no browser crates in editing), retired in S7 with the editing module | The narrowed mission editing matrix arm plus the crate firewalls' source scan over `crates/mission_editing/` (done in S7) |
| The forbidden-edge list in `tools/foundation/repository_laws/src/crate_dependencies.rs` | The allowed-edge category matrix and tier numbers |
| Source roots skip missing folders silently | Fail-closed roots derived from the workspace members (S0) |
| The map engine's feature gate tripwire test | Replaced by the anatomy law's features rule; deleted in S8 |
| No standards gate | Crate anatomy, strangler, frontend layering, tailwind sources |
| 84 frontend paths pinned by the editor ORBAT coherency check | Rewritten by the relocation tool at every move |

## End-state verification

The program closes when all of these hold, with logs named in
[progress.md](/documentation/restructure/progress.md):

- **Gates:** GS is green, with the test census at or above the baseline. The only test deleted is
  the feature-gate tripwire, which a law replaces. The API keeps 154 integration binaries.
- **File tree:** the tree matches
  [target_file_tree.md](/documentation/restructure/target_file_tree.md), checked by the close
  stage's tree diff.
- **Workspace shape:** 148 members pass crate tiers and crate anatomy. The legacy and website
  folders are gone. The only features left are the two dev-only ones.
- **Retired names:** `git grep` finds no retired spelling in live files: `_v2`, `src/v2`, the
  `website-` package prefix, or the website folder path.
- **Duplicates:** the repository-root marker, the deterministic generator, the clock, the SHA-256
  helper and glyph packing are each defined once.
- **JS exports:** `#[wasm_bindgen]` appears only in the frontend app, `browser_platform`, the
  offline service worker and the editor bridge's start hook.
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
