# Engine layer walls

The eight engine-layer rules of the
[engine boundary rules](/documentation/standards/engine_boundary_rules.md) §5, judged over a
checkout, with the report text `cargo xtask verify engine-layers` prints; and the map engine's
whole-crate UI-framework ban, which the `engineering_laws` test binary of `website-api` asserts.

## Contents

```text
tools/verification_core/src/repository_laws/engine_layers/
├── crate_walks.rs        walks the three crates, splits the map engine into its rule subsets, writes the scanned counts
├── evaluation.rs         judges rules 1 to 7 in report order and records each rule's finding count
├── matcher_probes.rs     compiles every matcher and proves it on subjects with known answers
├── mod.rs                the rule catalogue and why each rule is shaped so; `check_engine_layers` and its report types
├── report_text.rs        rule headlines, remedy paragraphs and refusal blocks
├── rules.rs              matchers, pinned residues and scanned roots
├── scanning.rs           shared helpers: report lines, build-output pruning, pin judgement, refusals
├── tests/                the fixture checkout and the rule, matcher, result and ban tests
└── ui_framework_ban.rs   the map engine's whole-crate UI-framework ban
```

## How it works

`check_engine_layers(repo_root)` proves every matcher, walks
`apps/website/graphics-engine` (sources and `Cargo.toml`), `apps/website/map-engine/src` and
`apps/website/frontend` (sources and `Cargo.toml`), then judges:

| Rule | Subject | Must hold |
|---|---|---|
| 1 | `apps/website/graphics-engine` | no `website_map_engine` in source, no `website-map-engine` edge in `Cargo.toml` |
| 2 | `apps/website/graphics-engine` | no declared name (after `struct`, `enum`, `trait`, `type`, `fn`, `const`, `static`, `mod`) containing terrain, symbology, mission, orbat or arma, case-insensitive |
| 3a | `apps/website/map-engine/src` | `website_graphics_engine::frame` appears only in `frame/mod.rs`, exactly 8 times |
| 3b | `apps/website/map-engine/src` | `website_graphics_engine::` followed by `device`, `pipeline`, `shaders` or `r#loop` appears only at the pinned sites: 3 in `frame/mod.rs`, 2 in `frame/pump.rs` |
| 4 | `data/scenario` | names none of `crate::` `camera`, `diagnostics`, `doll`, `editing`, `frame`, `io`, `overlay`, `spatial`, `streaming`, `world`, `data::store`, nor `website_graphics_engine`, nor a `super::` chain ending on one of them (`diagnostics` excepted, which names a module inside the tree), outside two pinned `cfg(feature = "store")` test files |
| 5 | `editing` | no `web_sys`, `leptos` or `wasm_bindgen`, prose included |
| 6 | `apps/website/frontend` | no `website_graphics_engine::` path or `extern crate`, no `website-graphics-engine` edge in `Cargo.toml` |
| 7 | `data` and `world` | `data/` names none of the ten sibling modules of rule 4 nor the graphics engine; `world/` names neither `crate::data` nor `yrs::` |

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
  `crate::repository_laws::cargo_manifest` for the UI-framework ban.
- Used by: `tools/xtask/src/verifications/architecture/engine_layer_boundaries.rs`, which
  prints the report; `apps/website/api_v2/tests/engineering_laws.rs`.
- Rules:
  - the report text is the gate's output contract and is byte-stable;
  - a root that is missing is "did not run" (exit 2) and an empty root or subset is a failure
    (`inputs_that_were_never_read_do_not_pass`, `an_absent_half_of_the_wall_is_not_a_clean_wall`);
  - every pin carries its file, exact count and reason, and changing one is a reviewed edit
    (`the_rule_3a_pin_is_a_ratchet_in_both_directions`,
    `the_rule_3b_pin_is_a_ratchet_in_both_directions`,
    `the_rule_4_pin_is_a_ratchet_in_both_directions`);
  - rules 4 and 7 cover the `editing` module
    (`the_scenario_tree_naming_the_editing_module_fails`,
    `the_document_naming_the_editing_module_breaches_the_wall`).

## Related documentation

- [Engine boundary rules](/documentation/standards/engine_boundary_rules.md) — the rules and
  the dependency direction they enforce.
- [Architecture verifications](/tools/xtask/src/verifications/architecture/README.md) — the
  gate that prints this report.
