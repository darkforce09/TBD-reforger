# Workspace laws

The five laws that judge the members of the root `Cargo.toml`: crate tiers, crate anatomy,
test-file reachability, frontend layering and Tailwind sources. Each law returns a report that
`cargo xtask verify crate-tiers`, `crate-anatomy`, `test-file-reachability`, `frontend-layering`
and `tailwind-sources` print line for line.

## Contents

```text
tools/foundation/repository_laws/src/workspace_laws/
├── crate_anatomy.rs             the anatomy law's manifest half and README check; the law's report
├── crate_anatomy_sources.rs     the anatomy law's source half: lib.rs, prelude, error.rs, typed ids, re-exports
├── crate_firewalls.rs           the crate-tier law's external-crate firewalls and the rendering-stack clause
├── crate_layout.rs              categories, the judged set and declared targets
├── crate_tiers.rs               the crate-tier law: the application boundary, the firewalls and its report
├── frontend_layering.rs         the layer table and order types, the in-crate mode and the hard layering report
├── frontend_layering/           the layering law's crate-edge mode: the edges between frontend crates
├── mod.rs                       the law reports' shared shape: `LawOutcome` and `WorkspaceLawReport`
├── public_reexports.rs          the crates every public re-export statement of a source names, in any form
├── rust_module_references.rs    the in-crate module paths a source file names, comments and strings blanked
├── tailwind_sources.rs          one exact `@source` line per leptos member, and no stale line
├── test_file_reachability.rs    every `.rs` file in a member's test folders reached from one of its targets
├── test_file_reachability/      the reachability law's module declaration reader
└── tests/                       the fixture workspace and the crate-tier, crate-anatomy and Tailwind tests
```

## How it works

Every law reads the members with `workspace_members::read_workspace_members` and judges the
**judged set**: every member that declares `[package.metadata.layout]` plus every member under
`crates/<category…>/<name>` or `tools/<category>/<name>`, the applications included. A report prints `==> <law> — <summary>`, its notes, one
`FAIL:` line per finding and a verdict line `<LAW>: PASS`, `<LAW>: FAIL (<n> finding(s))` (exit 1) or `<LAW>: FAIL (did not run)`
(exit 2, an input missing or unreadable).

| Law | Function | Judges |
|---|---|---|
| Crate tiers | `crate_tiers::check_crate_tiers(root, configuration)` (`CrateTierConfiguration`: the application packages) | no member depends on an application package in any table (normal, build, dev, target-specific; a crate's edge onto itself excepted), and every application package is a member; the external-crate firewalls hold |
| Crate anatomy | `crate_anatomy::check_crate_anatomy(root)` | each judged library crate: `lib.rs` ≤ 80 lines of doc comments, attributes, `mod` and `pub use` lines; `pub mod prelude`; a `thiserror` `error.rs` when a `pub fn` returns `Result`; no `anyhow`; a README Contents block; inherited `edition`, `rust-version`, `[lints]` and dependencies; only `test_fixtures` and `failpoints`, enabled only by dev-dependencies; no primitive public `id` / `*_id` outside `generated/` and `#[wasm_bindgen]`; no public re-export of another workspace crate outside the crate's prelude module, in any form `public_reexports` reads — an item, a module or the crate root: `pub use <crate>::…`, `pub use <crate>;`, an alias (`pub use <crate> as x;`), a leading `::`, a group at the top or nested (`pub use {<crate> as x};`, `pub use <crate>::{self as x};`), a statement over several lines, `pub extern crate <crate> as x;` |
| Test-file reachability | `test_file_reachability::check_test_file_reachability(root)` | every `.rs` file of a member in a `tests` folder — under `src/` or the member's own `tests/` — is loaded by the module tree of one of its targets (the manifest's `[lib]` and `[[bin]]` paths, `src/lib.rs`, `src/main.rs`, `src/bin/`, `tests/*.rs`, `tests/*/main.rs`, `benches/`, `examples/`) through `mod` declarations and their `#[path]`, or named as a trybuild case (`compile_fail` / `pass`) by a loaded file; the module tree follows the Rust reference's path rules (see the [declaration reader README](/tools/foundation/repository_laws/src/workspace_laws/test_file_reachability/README.md)); an integration target is never a finding |
| Frontend layering | `frontend_layering::check_frontend_layering(root, layering)` | in-crate mode: import edges where a lower layer names a higher one, pages and workspaces name each other, one page area names another, a sub-area of an ordered folder names a sibling above its own tier or a peer of its tier (a mutual-group tier excepted), or a production file names a test-only sub-area; one finding per (file, target place), production and test apart. Crate-edge mode: the same layer order and orders over every dependency edge between frontend crates (normal, dev, build), page crates as peers, a test-only crate only through dev-dependencies, two orders of one layer folder independent, and a frontend crate in no layer folder or no order of its folder a finding. Hard at zero, so any edge fails, as does a source no row maps or a child of an ordered folder in no tier |
| Tailwind sources | `tailwind_sources::check_tailwind_sources(root, stylesheet)` | every member with a non-dev `leptos` dependency is named by exactly one `@source` line whose glob, resolved from the stylesheet's folder, is `<member>/src/**/*.rs` (an ancestor or wildcard folder names no member); every line naming no leptos member's sources is stale, a finding |

The firewalls (`crate_firewalls`) read sources three times besides the manifests: no
declared name with a map noun in a graphics crate (a declaration keyword, then an identifier
holding `terrain`, `symbology`, `mission`, `orbat` or `arma`); no `web_sys`, `leptos` or
`wasm_bindgen` token, prose included, in any `.rs` file under `crates/mission_editing/`; and no
`#[wasm_bindgen]` attribute (plain, path-qualified or under `cfg_attr`, at the start of a line) in
any `.rs` file of a workspace member outside `crates/frontend/shell/frontend_application/`,
`crates/foundation/browser_platform/` and `crates/frontend/shell/offline_service_worker/`. The mission editing scan
runs whenever that folder exists or a member declares the category, and a folder holding no `.rs`
file is a finding; the export scan walking no `.rs` file is a finding too. The rendering-stack
clause reads every table, dev and target-specific included: the offline service worker and every
`crates/api` crate declare no edge onto a `crates/graphics`, `crates/map_rendering`,
`crates/paper_doll` or `crates/streaming` crate, nor onto `wgpu`.

The frontend-layering configuration (`FrontendLayering`) is the caller's, in two halves:

- **in-crate** (`FrontendCrateLayers`): xtask passes the layer table of `crates/frontend/shell/frontend_application`, the
  app shell: the entry point, the render form of the route table, the platform frame in
  `src/shell` and the crate-level tests are all the shell layer, and the app holds no sub-area
  order. The mode keeps its general form for a crate that holds several layers: a table row maps
  a folder onto a layer, and an order's tier is either peers that never import each other
  (`SubAreaTier::peers`) or one mutual group whose members may (`SubAreaTier::group`). Module
  paths are read from `crate::` and `super::` paths after comments and string literals are
  blanked, braced `use` groups included. A configured row or ordered folder that no longer exists
  is a note, while a missing crate `src` folder makes the law "did not run";
- **crate-edge** (`frontend_layering::crate_edges::FrontendCrateEdges`, see the
  [crate-edge README](/tools/foundation/repository_laws/src/workspace_laws/frontend_layering/README.md)):
  the layer folders `crates/frontend/<layer>/`, the app as the shell, and the crate orders, each a
  `SubAreaOrder` over package names: the foundation crates (`frontend_ui` < `frontend_api_dtos` <
  {`frontend_transport`, `frontend_route_table`} < `frontend_session` < {`frontend_offline`,
  `frontend_map_view`}, `frontend_test_support` test-only), the Mission Creator crates
  (`mission_creator_state` < `mission_creator_engine_bridge` < `mission_creator_session` <
  `mission_creator_arsenal` < `mission_creator_workspace`), `debug_benches` as an order of its
  own, and the shell crates (`crates/frontend/shell`: `frontend_application` and
  `offline_service_worker`, one tier of peers that never name each other). The Mission Creator is five crates, so its layering is judged by their dependency edges
  and no in-crate module order applies to it. A crate the orders name that no member carries is a
  finding.

## Public surface

- `WorkspaceLawReport` (`exit_code`, `lines`, `from_outcome`) and `LawOutcome`.
- `crate_tiers`: `check_crate_tiers`, `crate_tier_outcome`, `CrateTierConfiguration`.
- `crate_anatomy`: `check_crate_anatomy`, `crate_anatomy_outcome`, `is_library`,
  `ALLOWED_FEATURES`, `INHERITED_PACKAGE_KEYS`.
- `crate_layout`: `CategoryClass`, `TargetPlatforms`, `category_class`, `declared_targets`,
  `effective_category`, `is_judged`, `sits_in_layout_folder`, the category constants and
  `FRONTEND_LAYER_FOLDERS`.
- `test_file_reachability`: `check_test_file_reachability`, `test_file_reachability_outcome`,
  `unreachable_test_files`, `UnreachableTestFile`, `TEST_FOLDER`.
- `frontend_layering`: `check_frontend_layering`, `frontend_layering_outcome`, `layering_edges`,
  `FrontendLayering`, `FrontendLayer`, `FrontendLayerRow`, `SubAreaOrder`, `SubAreaTier`,
  `FrontendCrateLayers`, `LayeringEdge`, `LayeringScan`; `frontend_layering::crate_edges`:
  `crate_edge_scan`, `FrontendCrateEdges`, `FrontendLayerFolder`, `CrateEdgeScan`.
- `tailwind_sources`: `check_tailwind_sources`, `tailwind_sources_outcome`, `source_globs`,
  `LEPTOS_PACKAGE`.

## Boundaries

- Depends on: `super::workspace_members`, `super::cargo_manifest`, `super::source_roots`,
  `crate::scan`, `crate::pattern`,
  `crate::verdict`; `regex`.
- Used by: `tools/checks/repository_checks/src/architecture/workspace_laws.rs`.
- Rules:
  - a missing member folder, root manifest, stylesheet, crate source folder or app crate is "did
    not run", never a pass (`tailwind_sources_a_missing_stylesheet_did_not_run`);
  - an edge onto an application in any table is a finding
    (`crate_tiers_an_edge_onto_an_application_in_any_table_is_a_finding`);
  - the crate-tier, crate-anatomy and Tailwind laws pass this checkout
    (`crate_tiers_this_checkout_passes` and its siblings).

## Related documentation

- [Crate boundary rules](/documentation/standards/crate_boundary_rules.md) — the boundaries
  these laws hold.
- [CI gates](/documentation/standards/coding_standards/ci_gates.md#verify-workspace-laws) —
  rules WS-1 to WS-5 and where they run.
