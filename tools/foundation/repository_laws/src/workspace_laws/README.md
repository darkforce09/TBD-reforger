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
├── crate_firewalls.rs           rule 6 of the crate-tier law: the external-crate firewalls
├── crate_layout.rs              categories, the judged set, the apps and tool binaries outside it, declared targets and the category edge matrix
├── crate_tiers.rs               rules 1 to 7 of the crate-tier law and its report
├── frontend_layering.rs         the layer table and order types, the in-crate mode and the hard layering report
├── frontend_layering/           the layering law's crate-edge mode: the edges between frontend crates
├── mod.rs                       the law reports' shared shape: `LawOutcome` and `WorkspaceLawReport`
├── public_reexports.rs          the crates every public re-export statement of a source names, in any form
├── rust_module_references.rs    the in-crate module paths a source file names, comments and strings blanked
├── tailwind_sources.rs          one exact `@source` line per leptos member, and no stale line
├── test_file_reachability.rs    every `.rs` file in a member's test folders reached from one of its targets
├── test_file_reachability/      the reachability law's module declaration reader
└── tests/                       the fixture workspace and one test file per law and reader
```

## How it works

Every law reads the members with `workspace_members::read_workspace_members` and judges the
**judged set**: every member that declares `[package.metadata.layout]` plus every member under
`crates/<category…>/<name>` or `tools/<category>/<name>`. Outside the set stand only the apps
under `apps/` and the tool binaries of `crate_layout::TOOL_BINARY_PATHS` (`tools/xtask`,
`tools/developer_tools`), named in a note; any other member outside it is a rule 2 finding of
the crate-tier law. A report prints `==> <law> — <summary>`, its notes, one `FAIL:` line per finding and a
verdict line `<LAW>: PASS`, `<LAW>: FAIL (<n> finding(s))` (exit 1) or `<LAW>: FAIL (did not run)`
(exit 2, an input missing or unreadable).

| Law | Function | Judges |
|---|---|---|
| Crate tiers | `crate_tiers::check_crate_tiers(root, sweep_roots)` | 1 every `Cargo.toml` under the sweep roots (outside `tests`, `fixtures`, `test_fixtures`, `target`, `node_modules`) is a member; 2 each judged member declares its layout, and every member outside the judged set is an app or a tool binary; 3 category equals the parent folder, package equals the folder name; 4 declared tier equals 1 plus the highest judged dependency tier (0 with none), edges strictly down; 5 the category matrix, and a wasm-only crate reached from an `any` crate only through a wasm32 table; 6 the firewalls; 7 no dev-dependency onto `apps/` |
| Crate anatomy | `crate_anatomy::check_crate_anatomy(root)` | each judged library crate: `lib.rs` ≤ 80 lines of doc comments, attributes, `mod` and `pub use` lines; `pub mod prelude`; a `thiserror` `error.rs` when a `pub fn` returns `Result`; no `anyhow`; a README Contents block; inherited `edition`, `rust-version`, `[lints]` and dependencies; only `test_fixtures` and `failpoints`, enabled only by dev-dependencies; no primitive public `id` / `*_id` outside `generated/` and `#[wasm_bindgen]`; no public re-export of another workspace crate outside the crate's prelude module, in any form `public_reexports` reads — an item, a module or the crate root: `pub use <crate>::…`, `pub use <crate>;`, an alias (`pub use <crate> as x;`), a leading `::`, a group at the top or nested (`pub use {<crate> as x};`, `pub use <crate>::{self as x};`), a statement over several lines, `pub extern crate <crate> as x;` |
| Test-file reachability | `test_file_reachability::check_test_file_reachability(root)` | every `.rs` file of a member in a `tests` folder — under `src/` or the member's own `tests/` — is loaded by the module tree of one of its targets (the manifest's `[lib]` and `[[bin]]` paths, `src/lib.rs`, `src/main.rs`, `src/bin/`, `tests/*.rs`, `tests/*/main.rs`, `benches/`, `examples/`) through `mod` declarations and their `#[path]`, or named as a trybuild case (`compile_fail` / `pass`) by a loaded file; the module tree follows the Rust reference's path rules (see the [declaration reader README](/tools/foundation/repository_laws/src/workspace_laws/test_file_reachability/README.md)); an integration target is never a finding |
| Frontend layering | `frontend_layering::check_frontend_layering(root, layering)` | in-crate mode: import edges where a lower layer names a higher one, pages and workspaces name each other, one page area names another, a sub-area of an ordered folder names a sibling above its own tier or a peer of its tier (a mutual-group tier excepted), or a production file names a test-only sub-area; one finding per (file, target place), production and test apart. Crate-edge mode: the same layer order and orders over every dependency edge between frontend crates (normal, dev, build), page crates as peers, a test-only crate only through dev-dependencies, two orders of one layer folder independent, and a frontend crate in no layer folder or no order of its folder a finding. Hard at zero, so any edge fails, as does a source no row maps or a child of an ordered folder in no tier |
| Tailwind sources | `tailwind_sources::check_tailwind_sources(root, stylesheet)` | every member with a non-dev `leptos` dependency is named by exactly one `@source` line whose glob, resolved from the stylesheet's folder, is `<member>/src/**/*.rs` (an ancestor or wildcard folder names no member); every line naming no leptos member's sources is stale, a finding |

The category matrix (`crate_layout::category_edge_allowed`): foundation → foundation; contracts →
foundation, contracts; mission → foundation, mission, any crate of the `geometry` category;
ballistics → foundation, ballistics; graphics → foundation, graphics; the other engine categories
→ foundation, contracts, any engine category, graphics included; map rendering and paper doll →
foundation, contracts, mission, ballistics, any engine category, map rendering, paper doll;
mission editing → foundation crates whose `targets` is not `wasm32`,
mission, mission editing, and the geometry, world formats, terrain, world objects, line of sight
and map overlay categories (never ballistics, streaming or graphics);
api → foundation, contracts, mission, ballistics, api; frontend → any category but api and tools;
tools → foundation, contracts, mission, ballistics, engine crates whose `targets` is `any`, and
tools, never a wasm-only crate, with the staging fixtures tool (tools/staging/staging_fixtures) also reaching api.

The firewalls (`crate_firewalls`, rule 6) read sources three times besides the manifests: no
declared name with a map noun in a graphics crate (a declaration keyword, then an identifier
holding `terrain`, `symbology`, `mission`, `orbat` or `arma`); no `web_sys`, `leptos` or
`wasm_bindgen` token, prose included, in any `.rs` file under `crates/mission_editing/`; and no
`#[wasm_bindgen]` attribute (plain, path-qualified or under `cfg_attr`, at the start of a line) in
any `.rs` file of a workspace member outside `apps/frontend/`,
`crates/foundation/browser_platform/` and `apps/offline_service_worker/`. The mission editing scan
runs whenever that folder exists or a member declares the category, and a folder holding no `.rs`
file is a finding (`crate_tiers_an_empty_mission_editing_root_is_not_a_clean_scan`); the export
scan walking no `.rs` file is a finding too
(`crate_firewalls_a_workspace_without_rust_sources_is_not_a_clean_scan`).

The frontend-layering configuration (`FrontendLayering`) is the caller's, in two halves:

- **in-crate** (`FrontendCrateLayers`): xtask passes the layer table of `apps/frontend`, the
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
  `mission_creator_arsenal` < `mission_creator_workspace`) and `debug_benches` as an order of its
  own. The Mission Creator is five crates, so its layering is judged by their dependency edges
  and no in-crate module order applies to it. A crate the orders name that no member carries is a
  finding.

## Public surface

- `WorkspaceLawReport` (`exit_code`, `lines`, `from_outcome`) and `LawOutcome`.
- `crate_tiers`: `check_crate_tiers`, `crate_tier_outcome`, `SWEEP_SKIPPED_FOLDERS`.
- `crate_anatomy`: `check_crate_anatomy`, `crate_anatomy_outcome`, `is_library`,
  `ALLOWED_FEATURES`, `INHERITED_PACKAGE_KEYS`.
- `crate_layout`: `CategoryClass`, `TargetPlatforms`, `EdgeEnd`, `category_class`,
  `category_edge_allowed`, `declared_targets`, `effective_category`, `is_judged`,
  `sits_in_layout_folder`, `is_under`, `is_app_or_tool_binary`, `is_wasm32_only_cfg`, the root
  and category constants and `TOOL_BINARY_PATHS`.
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
    not run", never a pass (`crate_tiers_a_missing_member_folder_did_not_run`,
    `tailwind_sources_a_missing_stylesheet_did_not_run`,
    `frontend_crate_edges_a_missing_app_or_root_manifest_did_not_run`);
  - a manifest under `crates/` in a category no member glob of the root manifest lists is rule 1
    (`crate_tiers_a_crate_in_a_category_no_member_glob_lists_is_rule_1`);
  - a member outside the judged set that is neither an app nor a tool binary is rule 2
    (`crate_tiers_a_member_outside_the_judged_set_is_rule_2_unless_an_app_or_a_tool_binary`);
  - a test file no target reaches is a finding, a declaration from an unreached file or inside a
    string reaches nothing
    (`test_file_reachability_a_declaration_from_an_unreached_file_or_a_string_counts_for_nothing`);
  - every law passes this checkout (`crate_tiers_this_checkout_passes` and its siblings), the
    layering law with no edge.

## Related documentation

- [Crate boundary rules](/documentation/standards/crate_boundary_rules.md) — the boundaries
  these laws hold.
- [CI gates](/documentation/standards/coding_standards/ci_gates.md#verify-workspace-laws) —
  rules WS-1 to WS-5 and where they run.
