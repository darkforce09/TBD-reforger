# Architecture verifications

Source checks for three structural contracts: the layer walls between the map engine, the graphics
engine and the frontend; the `@route` doc tags of the [API](/documentation_v2/glossary/a_to_f.md#api)
against its route tables; and the [ORBAT](/documentation_v2/glossary/n_to_z.md#orbat) and placement
guarantees of the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator). Each gate is
its own `cargo xtask verify` verb.

## Contents

```text
tools_v2/xtask/src/verifications/architecture/
├── editor_orbat_coherency/     the ORBAT coherency runner: static checks, cargo test pins, output
├── editor_orbat_coherency.rs   the ORBAT coherency gate's target files, bans, colour pins and test pins
├── engine_layer_boundaries/    the engine-layer gate's runner and rule evaluation
├── engine_layer_boundaries.rs  the engine-layer gate: its eight rules and why each is shaped so
├── engine_layer_rules.rs       the engine-layer matchers, pinned residues and report text
├── engine_layer_scan.rs        engine-layer helpers: output lines, build-output pruning, pins, refusals
├── mod.rs                      the module tree
├── route_tags/                 route-tag extraction, report collation and the runner
├── route_tags.rs               the route-tag gate's paths, patterns, sentinels and report text
├── tests/                      unit tests for the three gates
└── wave_gate_sources.rs        checks that the wave gate facade declares and exports both implementations
```

## How it works

Each gate reads source text from the working tree (untracked files included) with
`verification_core::scan`, compiles its matchers in, and treats an input it could not read as a
check that did not run, never as a pass.

| Verb | Reads | Fails when | Exit codes |
|---|---|---|---|
| `engine-layers` | `apps/website/graphics-engine` (sources and `Cargo.toml`), `apps/website/map-engine/src`, `apps/website/frontend` (sources and `Cargo.toml`) | a rule below is broken, a matcher self-probe answers wrongly, or a walk finds no files | 0 pass, 1 breach, 2 a root or manifest missing |
| `route-tags` | `apps/website/api_v2/src`: the domain `routes.rs` tables, `core/http_router.rs`, every `@route` tag | a tag names no registered route, a route has no tag, or the parse, mount or sentinel guards fail | 0 pass, 1 mismatch, 2 source unreadable |
| `editor-orbat-coherency` | named editor, store and symbology files; `cargo test` runs | a ban matches, a pin is absent, or a test pin fails or runs no test | 0 pass, 1 every failure |

### Engine layers

The rules, with their matchers in `engine_layer_rules.rs`:

| Rule | Subject | Must hold |
|---|---|---|
| 1 | `apps/website/graphics-engine` | no `website_map_engine` in source, no `website-map-engine` edge in `Cargo.toml` |
| 2 | `apps/website/graphics-engine` | no declared name (after `struct`, `enum`, `trait`, `type`, `fn`, `const`, `static`, `mod`) containing terrain, symbology, mission, orbat or arma, case-insensitive |
| 3a | `apps/website/map-engine/src` | `website_graphics_engine::frame` appears only in `frame/mod.rs`, exactly 8 times |
| 3b | `apps/website/map-engine/src` | `website_graphics_engine::` followed by `device`, `pipeline`, `shaders`, `r#loop` or `text::gpu` appears only at the pinned sites: 3 in `frame/mod.rs`, 2 in `frame/pump.rs` |
| 4 | `data/scenario` | names none of `crate::` `camera`, `diagnostics`, `doll`, `frame`, `io`, `overlay`, `spatial`, `streaming`, `world`, `data::store`, nor `website_graphics_engine`, nor a `super::` chain ending on one of them (`diagnostics` excepted, which names a module inside the tree), outside two pinned `cfg(feature = "store")` test files |
| 5 | `editing` | no `web_sys`, `leptos` or `wasm_bindgen`, prose included |
| 6 | `apps/website/frontend` | no `website_graphics_engine::` path or `extern crate`, no `website-graphics-engine` edge in `Cargo.toml` |
| 7 | `data` and `world` | `data/` names none of the nine sibling modules of rule 4 nor the graphics engine; `world/` names neither `crate::data` nor `yrs::` |

A pin is a ratchet: an unpinned file that matches fails, and so does a pinned file whose count
rises or falls or that no longer exists. Any folder below the root named `target` or starting
with `target-` is build output and is pruned from the walks.

### Route tags

Direction A requires every `/// @route METHOD PATH` tag to name a route that a domain table
registers on that method for that handler; direction B requires every registered route to carry
that tag. The route side is the union of the `apps/website/api_v2/src/<domain>/routes.rs` tables
that `api_v1_routes` merges under `/api/v1`; the `route_tags/` README describes the guards.

### ORBAT coherency

Three bans: `ensure_default_squad` on the placement path (the Mission Creator's arming and
context files in `apps/website/frontend/src/v2/apps/editor/` and the map engine's
`data/store/operations/` and `editing/hosted_commands/`), `loadout: String::new()` in the slot
template derive, and the strings Standardization, IFAK or Grenade Complement in the ORBAT manager
modal and the editor chrome. Three pins require the BLUFOR, OPFOR and INDFOR side colours in
`apps/website/map-engine/src/overlay/symbology/roles/classify.rs`. Then 25 `cargo test` pins run
named selectors: `website-map-engine --lib` with the `scenario store` or the `render` features,
and `website-frontend` with none; each must exit 0 and pass at least one test. The gate stops at
the first failure.

## Public surface

- `engine_layer_boundaries::verify_engine_layers`, `route_tags::verify_route_tags` and
  `editor_orbat_coherency::verify_editor_orbat_coherency`: the three gates, each taking the
  repository root and returning the exit status.
- `wave_gate_sources::WAVE_CHILDREN` and `wave_children_are_linked`: the two implementation
  modules of `tools_v2/xtask/src/commands/platform/wave_execution/gate.rs` (`checkrun` with
  `gate_slice`, `gate_dispatch` with `cmd_gate`) and the `syn` check that the facade declares each
  module and publicly re-exports its function.

## Boundaries

- Depends on: `verification-core` (patterns, gates, scans, verdicts, `proc::Run`); the `regex`
  and `syn` crates; `cargo` for the ORBAT test pins.
- Used by:
  - `tools_v2/xtask/src/commands/verify/dispatch.rs`, for `cargo xtask verify engine-layers`,
    `cargo xtask verify route-tags` and `cargo xtask verify editor-orbat-coherency`;
  - `tools_v2/xtask/src/commands/ci/task_definitions/verification_dispatch.rs`, for the
    `verify-engine-layers` step of `ci-local` and the `route-tags` step of
    `verify-coding-standards`;
  - the [wave](/documentation_v2/glossary/n_to_z.md#wave) gate, whose `VERIFY_STEPS` in
    `tools_v2/xtask/src/commands/platform/wave_execution/gate.rs` runs `verify route-tags`, and
    the `language-gates` job of `.github/workflows/ci.yml`, which runs `verify engine-layers`;
  - `tools_v2/xtask/src/verifications/ci/schema_parity/source_audit.rs` and
    `tools_v2/xtask/src/verifications/database/faction_library_seeds/extract_fn_body.rs`, for
    `wave_gate_sources.rs`.
- Rules:
  - a check that could not read its input never reads as a pass: engine-layers and route-tags
    exit 2 (`inputs_that_were_never_read_do_not_pass` in both test files), and the ORBAT gate
    exits 1 with the cause named (`a_missing_target_never_reads_as_a_pass`);
  - every matcher is a compiled constant, probed on known subjects before it judges source;
  - every pin in `engine_layer_rules.rs` carries its file, exact count and reason, and changing
    one is a reviewed edit (`naming_the_frame_vocabulary_outside_the_boundary_fails`,
    `a_new_gpu_module_import_in_the_map_engine_fails`,
    `the_scenario_tree_reaching_outside_itself_fails`).

## Related documentation

- [Coding standards](/documentation_v2/standards/coding_standards/README.md) — the GO-7 rule that
  every handler carries its `@route` tag.
- [Documentation standards](/documentation_v2/standards/documentation_standards.md) — the grammar
  of the `@route` tag.
