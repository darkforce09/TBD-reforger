# Engine layer walls

The seven engine-layer rules of the
[engine boundary rules](/documentation/standards/engine_boundary_rules.md) §5 (numbers 1, 2, 3a,
3b, 5, 6 and 7; number 4 is unassigned), judged over a
checkout, with the report text `cargo xtask verify engine-layers` prints; and the map engine's
whole-crate UI-framework ban, which the `engineering_laws` test binary of `api` asserts.

## Contents

```text
tools/foundation/repository_laws/src/engine_layers/
├── crate_walks.rs        walks the graphics layer, the map engine and the frontend, splits the map engine into its rule subsets, writes the scanned counts
├── evaluation.rs         judges the seven rules in report order and records each rule's finding count
├── matcher_probes.rs     compiles every matcher and proves it on subjects with known answers
├── mod.rs                the rule catalogue and why each rule is shaped so; `check_engine_layers` and its report types
├── report_text.rs        rule headlines, remedy paragraphs and refusal blocks
├── rules.rs              matchers, pinned residues and scanned roots
├── scanning.rs           shared helpers: report lines, build-output pruning, pin judgement, refusals
├── tests/                the fixture checkout and the rule, matcher, result and ban tests
└── ui_framework_ban.rs   the map engine's whole-crate UI-framework ban
```

## How it works

`check_engine_layers(repo_root)` proves every matcher, walks the graphics layer —
`legacy/graphics_engine` and every workspace member whose category is `crates/graphics`
(sources and `Cargo.toml` each; the members come from the root manifest) —
`legacy/map_engine/src` and `apps/frontend` (sources and `Cargo.toml`), then judges:

| Rule | Subject | Must hold |
|---|---|---|
| 1 | `legacy/graphics_engine` and each `crates/graphics` member | no `map_engine::` path or `extern crate map_engine` in source, no `map_engine` edge in `Cargo.toml` |
| 2 | `legacy/graphics_engine` and each `crates/graphics` member | no declared name (after `struct`, `enum`, `trait`, `type`, `fn`, `const`, `static`, `mod`) containing terrain, symbology, mission, orbat or arma, case-insensitive |
| 3a | `legacy/map_engine/src` | `graphics_engine::frame` appears only in `frame/mod.rs`, exactly 5 times |
| 3b | `legacy/map_engine/src` | `graphics_engine::` followed by `device`, `pipeline`, `shaders` or `r#loop` appears only at the pinned sites: 3 in `frame/mod.rs`, 2 in `frame/pump.rs` |
| 5 | `editing` | no `web_sys`, `leptos` or `wasm_bindgen`, prose included |
| 6 | `apps/frontend` | no `graphics_engine::` path or `extern crate`, no `graphics_engine` edge in `Cargo.toml`; the same for each `crates/graphics` member declaring `targets = "wasm32"` (a `targets = "any"` member is CPU code the frontend may link) |
| 7 | `world` | no `yrs::` path, no `mission_crdt::`, `mission_document::` or `mission_operations::` path, no `crate::editing` and no `super::` chain ending on `editing` |

The mission crates' isolation from the world, streaming and graphics code is a dependency edge,
which no line matcher here judges; the crate-tier law holds it from the manifests: a
`crates/mission` crate depends on foundation, mission and geometry crates only and on nothing
under `legacy/` (`crate_tiers_a_mission_crate_reaching_world_or_graphics_is_rule_5` in
`workspace_laws/tests/crate_tiers.rs`).

A graphics category with no member, or a member whose `src` holds no `.rs` file, is a failure
like any other empty root; a root manifest whose members cannot be read is "did not run". The
rule 6 matcher over the wasm-only members is the one matcher built at run time, from their crate
names, and it is probed before it judges like the constants.

A pin is a ratchet: an unpinned file that matches fails, and so does a pinned file whose count
rises or falls or that no longer exists. Any folder below the root named `target` or starting
with `target-` is build output and is pruned from the walks.

The report is an `EngineLayerReport`: the exit code (0 every rule held; 1 a breach, a matcher
that answered a probe wrongly, or a root or subset with no file; 2 a root, manifest or file that
could not be read), the report lines, and one `EngineLayerRuleResult` per judged rule with its
finding count and finding lines.

`map_engine_ui_framework_findings(repo_root)` reads the map engine's manifest (every dependency
table, renamed edges under their real package) for a UI framework package — `leptos`, `yew`,
`dioxus`, `sycamore`, `egui`, `eframe`, `iced`, `slint`, `tauri`, `relm4`, `gtk4` and their
suffixed companions — and its sources for a `<framework>::` path or `extern crate`. `web-sys` and
`wasm-bindgen` are browser bindings the renderer and streaming host use on purpose, not UI
frameworks.

## Public surface

- `check_engine_layers`, `EngineLayerReport`, `EngineLayerRuleResult`, `EngineLayerRule`.
- `map_engine_ui_framework_findings`, `UiFrameworkScan`, `UI_FRAMEWORK_PACKAGE_RE`,
  `UI_FRAMEWORK_IMPORT_RE`.

## Boundaries

- Depends on: `crate::scan`, `crate::gate`, `crate::Pattern`, `crate::Verdict` and `NotRun`;
  `crate::repository_laws::cargo_manifest` for the UI-framework ban; `crate::workspace_members`
  and `crate::workspace_laws::crate_layout` for the graphics category's members and targets.
- Used by: `tools/xtask/src/verifications/architecture/engine_layer_boundaries.rs`, which
  prints the report; `apps/api/tests/engineering_laws.rs`.
- Rules:
  - the report text is the gate's output contract and is byte-stable;
  - a root that is missing is "did not run" (exit 2) and an empty root or subset is a failure
    (`inputs_that_were_never_read_do_not_pass`, `an_absent_world_tree_is_not_a_clean_rule_7`,
    `an_absent_editing_tree_is_not_a_clean_rule_5`,
    `an_absent_graphics_category_is_not_a_clean_wall`);
  - every pin carries its file, exact count and reason, and changing one is a reviewed edit
    (`the_rule_3a_pin_is_a_ratchet_in_both_directions`,
    `the_rule_3b_pin_is_a_ratchet_in_both_directions`);
  - rule 7 covers the mission document crates and the `editing` module
    (`the_world_naming_the_document_breaches_the_wall`,
    `the_world_naming_the_editing_module_breaches_the_wall`).

## Related documentation

- [Engine boundary rules](/documentation/standards/engine_boundary_rules.md) — the rules and
  the dependency direction they enforce.
- [Architecture verifications](/tools/xtask/src/verifications/architecture/README.md) — the
  gate that prints this report.
