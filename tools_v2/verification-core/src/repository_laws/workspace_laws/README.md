# Workspace laws

The five laws of the [laws and gates](/documentation_v2/restructure/laws_and_gates.md#new-laws)
that judge the members of the root `Cargo.toml`: crate tiers, crate anatomy, the strangler rule,
frontend layering and Tailwind sources. Each law returns a report that
`cargo xtask verify crate-tiers`, `crate-anatomy`, `strangler`, `frontend-layering` and
`tailwind-sources` print line for line.

## Contents

```text
tools_v2/verification-core/src/repository_laws/workspace_laws/
├── crate_anatomy.rs             the anatomy law's manifest half and README check; the law's report
├── crate_anatomy_sources.rs     the anatomy law's source half: lib.rs, prelude, error.rs, typed ids, re-exports
├── crate_firewalls.rs           rule 6 of the crate-tier law: the external-crate firewalls
├── crate_layout.rs              categories, the judged set, declared targets and the category edge matrix
├── crate_tiers.rs               rules 1 to 8 of the crate-tier law and its report
├── frontend_layering.rs         the layer table types, the ratchet ceiling and the layering report
├── mod.rs                       the law reports' shared shape: `LawOutcome` and `WorkspaceLawReport`
├── rust_module_references.rs    the in-crate module paths a source file names, comments and strings blanked
├── strangler.rs                 dependency edges onto `legacy/` and re-export shims
├── tailwind_sources.rs          `@source` coverage of every leptos member
└── tests/                       the fixture workspace and one test file per law
```

## How it works

Every law reads the members with `workspace_members::read_workspace_members` and judges the
**judged set**: every member that declares `[package.metadata.layout]` plus every member under
`crates/<category…>/<name>` or `tools/<category>/<name>`. Members outside the set are named in a
note while `crate_layout::UNJUDGED_MEMBERS_FAIL` is false; the close stage of the restructure
turns it on. A report prints `==> <law> — <summary>`, its notes, one `FAIL:` line per finding and a
verdict line `<LAW>: PASS`, `<LAW>: FAIL (<n> finding(s))` (exit 1) or `<LAW>: FAIL (did not run)`
(exit 2, an input missing or unreadable).

| Law | Function | Judges |
|---|---|---|
| Crate tiers | `crate_tiers::check_crate_tiers(root, sweep_roots)` | 1 every `Cargo.toml` under the sweep roots (outside `tests`, `fixtures`, `test_fixtures`, `target`, `node_modules`) is a member; 2 the layout is declared; 3 category equals the parent folder, package equals the folder name; 4 declared tier equals 1 plus the highest judged dependency tier (0 with none), edges strictly down; 5 the category matrix, and a wasm-only crate reached from an `any` crate only through a wasm32 table; 6 the firewalls; 7 no edge onto a member under `legacy/`; 8 no dev-dependency onto `apps/` or `legacy/` |
| Crate anatomy | `crate_anatomy::check_crate_anatomy(root)` | each judged library crate: `lib.rs` ≤ 80 lines of doc comments, attributes, `mod` and `pub use` lines; `pub mod prelude`; a `thiserror` `error.rs` when a `pub fn` returns `Result`; no `anyhow`; a README Contents block; inherited `edition`, `rust-version`, `[lints]` and dependencies; only `test_fixtures` and `failpoints`, enabled only by dev-dependencies; no primitive public `id` / `*_id` outside `generated/` and `#[wasm_bindgen]`; no `pub use` of another workspace crate outside the crate's prelude module |
| Strangler | `strangler::check_strangler(root)` | no member outside `legacy/` (apps and `tools/xtask`, `tools/developer_tools` excepted) depends on a member under `legacy/`; no source under `legacy/` holds a `pub use` of a workspace crate outside `legacy/` |
| Frontend layering | `frontend_layering::check_frontend_layering(root, crates, ceiling)` | import edges where a lower layer names a higher one, pages and workspaces name each other, or one page area names another, counted per (file, target place), production and test apart, against `FRONTEND_LAYERING_CEILING` |
| Tailwind sources | `tailwind_sources::check_tailwind_sources(root, stylesheet)` | every member with a `leptos` dependency has an `@source` glob, resolved from the stylesheet's folder, ending in `/**/*.rs` over its `src` folder or an ancestor |

The category matrix (`crate_layout::category_edge_allowed`): foundation → foundation; contracts →
foundation, contracts; mission → foundation, mission, `crates/geometry`; ballistics → foundation,
ballistics; graphics → foundation, graphics; the other engine categories → foundation, contracts,
engine; map rendering and paper doll → foundation, contracts, mission, ballistics, engine, map
rendering, paper doll; mission editing → foundation, mission, ballistics, engine, mission editing;
api → foundation, contracts, mission, ballistics, api; frontend → any `crates/` category but api;
tools → foundation, contracts, mission, ballistics, engine crates whose `targets` is `any`, and
tools, never a wasm-only crate, with `tools/staging/staging_fixtures` also reaching api.

The frontend layer table is the caller's: xtask passes the table for
`apps/website/frontend` (`src/v2/core` foundation, `src/v2/pages` pages, `src/v2/apps`
workspaces, the entry point, route table, platform frame and crate-level tests the shell). Module
paths are read from `crate::` and `super::` paths after comments and string literals are blanked,
braced `use` groups included.

## Public surface

- `WorkspaceLawReport` (`exit_code`, `lines`, `from_outcome`) and `LawOutcome`.
- `crate_tiers`: `check_crate_tiers`, `crate_tier_outcome`, `SWEEP_SKIPPED_FOLDERS`.
- `crate_anatomy`: `check_crate_anatomy`, `crate_anatomy_outcome`, `is_library`,
  `ALLOWED_FEATURES`, `INHERITED_PACKAGE_KEYS`.
- `crate_layout`: `CategoryClass`, `TargetPlatforms`, `EdgeEnd`, `category_class`,
  `category_edge_allowed`, `declared_targets`, `effective_category`, `is_judged`,
  `sits_in_layout_folder`, `is_under`, `is_wasm32_only_cfg`, the root and category constants and
  `UNJUDGED_MEMBERS_FAIL`.
- `strangler`: `check_strangler`, `strangler_outcome`, `legacy_dependency_findings`,
  `reexported_crate`, `LEGACY_DEPENDENT_TOOL_BINARIES`.
- `frontend_layering`: `check_frontend_layering`, `frontend_layering_outcome`, `layering_edges`,
  `FrontendLayer`, `FrontendLayerRow`, `FrontendCrateLayers`, `LayeringCeiling`, `LayeringEdge`,
  `FRONTEND_LAYERING_CEILING`.
- `tailwind_sources`: `check_tailwind_sources`, `tailwind_sources_outcome`, `source_globs`,
  `LEPTOS_PACKAGE`.

## Boundaries

- Depends on: `super::workspace_members`, `super::cargo_manifest`, `super::source_roots`,
  `super::engine_layers` (rule 2's map-noun matcher), `crate::scan`, `crate::pattern`,
  `crate::verdict`; `regex`.
- Used by: `tools_v2/xtask/src/verifications/architecture/workspace_laws.rs`.
- Rules:
  - a missing member folder, root manifest, stylesheet or crate source folder is "did not run",
    never a pass (`crate_tiers_a_missing_member_folder_did_not_run`,
    `tailwind_sources_a_missing_stylesheet_did_not_run`);
  - every law passes this checkout (`crate_tiers_this_checkout_passes` and its siblings), the
    layering law at its ceiling.

## Related documentation

- [Laws and gates](/documentation_v2/restructure/laws_and_gates.md) — the specification.
- [CI gates](/documentation_v2/standards/coding_standards/ci_gates.md#verify-workspace-laws) —
  rules WS-1 to WS-5 and where they run.
- [Target file tree](/documentation_v2/restructure/target_file_tree.md) — the crate anatomy and
  the category folders.
