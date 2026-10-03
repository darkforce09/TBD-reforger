# Architecture verifications

Source checks for three structural contracts: the workspace laws over the members of the root manifest; the `@route` doc tags of the [API](/documentation/glossary/a_to_f.md#api)
against its route tables; and the [ORBAT](/documentation/glossary/n_to_z.md#orbat) and placement
guarantees of the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator). Each gate is
its own `cargo xtask verify` verb.

## Contents

```text
tools/checks/repository_checks/src/architecture/
├── editor_orbat_coherency/     the ORBAT coherency runner: static checks, cargo test pins, output
├── editor_orbat_coherency.rs   the ORBAT coherency gate's target files, bans, colour pins and test pins
├── mod.rs                      the module tree
├── route_tags/                 route-tag extraction, report collation and the runner
├── route_tags.rs               the route-tag gate's paths, patterns, sentinels and report text
├── tests/                      unit tests for the gates
├── wave_gate_sources.rs        the wave gate facade linkage check and the source spellings its audits share
├── workspace_law_locations.rs  the manifest sweep roots, the Tailwind stylesheet and the frontend layer table the laws read
└── workspace_laws.rs           the five workspace-law gates: print each library report
```

## How it works

Each gate reads source text from the working tree (untracked files included) with
`verification_core::scan`, compiles its matchers in, and treats an input it could not read as a
check that did not run, never as a pass.

| Verb | Reads | Fails when | Exit codes |
|---|---|---|---|
| `route-tags` | `crates/api/<crate>/src/routes.rs` tables, `apps/api/src/router.rs`, every `@route` tag under `apps/api/src` and `crates/api` | a tag names no registered route, a route has no tag, or the parse, mount or sentinel guards fail | 0 pass, 1 mismatch, 2 source unreadable |
| `crate-tiers`, `crate-anatomy`, `strangler`, `frontend-layering`, `tailwind-sources` | the root `Cargo.toml` and every member manifest; the judged crates' sources; the frontend's sources; `apps/frontend/style/aegis.css` | a law below is broken | 0 pass, 1 finding, 2 an input missing or unreadable |
| `editor-orbat-coherency` | named editor, store and symbology files; `cargo test` runs | a ban matches, a pin is absent, or a test pin fails or runs no test | 0 pass, 1 every failure |

### Workspace laws

Each of the five verbs prints the report of its law in
[`tools/foundation/repository_laws/src/workspace_laws/`](/tools/foundation/repository_laws/src/workspace_laws/README.md)
line for line and exits with its code. The paths that move with the tree (the manifest sweep
roots, the stylesheet, the frontend layer table) are the constants of
`tools/checks/repository_checks/src/architecture/workspace_law_locations.rs`. The `verify-workspace-laws` task row runs the five
in order as a step of `ci-local`.

### Route tags

Direction A requires every `/// @route METHOD PATH` tag to name a route that a domain table
registers on that method for that handler; direction B requires every registered route to carry
that tag. The route side is the union of the `crates/api/<crate>/src/routes.rs` tables
that `api_v1_routes` in `apps/api/src/router.rs` merges under `/api/v1`; the `route_tags/` README describes the guards.

### ORBAT coherency

Three bans: `ensure_default_squad` on the placement path (the Mission Creator's arming and
context files in `apps/frontend/src/workspaces/editor/`, `crates/mission/mission_operations/src/`
and the map engine's `editing/hosted_commands/`), `loadout: String::new()` in the slot
template derive, and the strings Standardization, IFAK or Grenade Complement in the ORBAT manager
modal and the editor chrome. Three pins require the BLUFOR, OPFOR and INDFOR side colours in
`crates/map_overlay/unit_symbology/src/classification.rs`. Then 26 `cargo test` pins run
named selectors: `mission_operations`, `mission_document`, `mission_model`, `mission_payload`
and `mission_compiler` with `--lib`, the map overlay crates `map_draw_lanes`, `unit_symbology`
and `overlay_instances` with `--lib`, and `frontend`, all with no features; each must exit 0 and
pass at least one test. The gate stops at the first failure.

## Public surface

- `route_tags::verify_route_tags` and
  `editor_orbat_coherency::verify_editor_orbat_coherency`: the two gates, each taking the
  repository root and returning the exit status.
- `workspace_laws::verify_crate_tiers`, `verify_crate_anatomy`, `verify_strangler`,
  `verify_frontend_layering` and `verify_tailwind_sources`: the five workspace-law gates over the
  checkout the command runs in; `workspace_law_report` and `verify_workspace_law` take a root.
- `wave_gate_sources::WAVE_CHILDREN` and `wave_children_are_linked`: the two implementation
  modules of `tools/commands/platform_execution/src/wave_execution/gate.rs` (`checkrun` with
  `gate_slice`, `gate_dispatch` with `cmd_gate`) and the `syn` check that the facade declares each
  module and re-exports its function to the crate (`pub` or `pub(crate)`; the gate lives in a
  private module of a library crate); `WAVE_EXPORT_VISIBILITY`, `wave_child_link_statements` and
  `wave_function_opener_pattern`: the facade statements and the function-opener pattern the audits
  pin, spelled once.

## Boundaries

- Depends on: `verification_core` (patterns, gates, scans, verdicts); `process_runner::Run`;
  `repository_laws::workspace_laws` for the rules; the
  `regex` and `syn` crates;
  `cargo` for the ORBAT test pins.
- Used by:
  - `tools/xtask/src/commands/verify/dispatch.rs`, for
    `cargo xtask verify route-tags` and `cargo xtask verify editor-orbat-coherency`;
  - `tools/commands/ci_task_catalog/src/task_definitions/verification_dispatch.rs`, for the
    `route-tags` step of
    `verify-coding-standards`, and `tools/commands/ci_task_catalog/src/task_definitions.rs`, for the
    five steps of `verify-workspace-laws`;
  - the [wave](/documentation/glossary/n_to_z.md#wave) gate, whose `VERIFY_STEPS` in
    `tools/commands/platform_execution/src/wave_execution/gate.rs` runs `verify route-tags`;
  - `tools/commands/ci_task_catalog/src/workflow_checks/schema_parity/source_audit.rs` and
    `tools/commands/database_operations/src/database_checks/faction_library_seeds/extract_fn_body.rs`, for
    `wave_gate_sources.rs`.
- Rules:
  - a check that could not read its input never reads as a pass: route-tags
    exits 2 (`inputs_that_were_never_read_do_not_pass`), and the ORBAT gate exits 1
    with the cause named (`a_missing_target_never_reads_as_a_pass`);
  - every matcher is a compiled constant, probed on known subjects before it judges source.

## Related documentation

- [Coding standards](/documentation/standards/coding_standards/README.md) — the GO-7 rule that
  every handler carries its `@route` tag.
- [Documentation standards](/documentation/standards/documentation_standards.md) — the grammar
  of the `@route` tag.
