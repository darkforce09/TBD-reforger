# CI task table helpers

The three helper modules of the `cargo xtask ci` task table in
`tools/commands/ci_task_catalog/src/task_definitions.rs`: the macros that spell a step, the step
lists of the map-lane rows that rebuild Everon's basemap assets, and the zero-argument adapters
that let a table row call a verification in process.

## Contents

```text
tools/commands/ci_task_catalog/src/task_definitions/
├── map_asset_steps.rs        step lists of the `map-water-everon` and `map-cartographic-everon` rows
├── recipe_macros.rs          `sh!` for a subprocess step, `xt!` for an in-process xtask step
├── verification_dispatch.rs  argument-free adapters for the in-process verification steps
└── workspace_law_steps.rs    step list of the `verify-workspace-laws` row: the five workspace laws
```

## How it works

`task_definitions.rs` pulls the three files in with `#[path]` attributes, the macros first with
`#[macro_use]` so the step lists can spell their steps with them. `sh!(line)` expands to
`Step::Cmd`, a line that is both echoed and split into the argv that runs; `xt!(echo, silent,
run)` expands to `Step::Xtask`, which prints the echoed
`cargo xtask …` line and calls `run` in process instead of spawning a second xtask. A
`Step::Xtask` holds a plain `fn() -> Result<u8>`, which cannot capture an argument, so
`verification_dispatch.rs` wraps each verification that needs one: the checkout root from
`find_repository_root`; the terrain `everon`, which the CI lane always checks, and the `--strict` flag
for the strict terrain alignment row; no `--path` for `enfusion-comments`, so it judges the
pinned Enfusion script roots; and, for the three documentation gates of the
`verify-documentation` row, a default `GateRequest`: the whole repository, committed files only,
and for `link-check` the first breaks in full, as the bare `cargo xtask verify` verbs run.

## Boundaries

- Depends on: `Step` from `tools/commands/ci_task_catalog/src/task_runner.rs`; the checks of this
  crate (`map_asset_checks`, `workflow_checks`), the `database_operations` and `deployment`
  crates, and the check crates `repository_checks`, `mod_script_checks` and
  `documentation_checks`; `find_repository_root` in
  `tools/foundation/repository_layout/src/repository_root.rs`.
- Used by: `tools/commands/ci_task_catalog/src/task_definitions.rs` alone.
- Rules: an adapter calls the same function its `cargo xtask verify` or `cargo xtask schema`
  command calls, so a composite cannot drift from the command it echoes; a subprocess step stays
  free of shell syntax apart from a leading `cd <dir> && ` (`cmd_lines_are_shell_free` in
  `tools/commands/ci_task_catalog/src/tests/task_runner.rs`).
