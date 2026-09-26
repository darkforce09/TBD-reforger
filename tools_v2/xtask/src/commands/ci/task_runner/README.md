# CI task runner

The behaviour behind the `cargo xtask ci` task table: finding a task, running its steps, the
`help` listing, and the gate list that `schema list-gates` prints.
`tools_v2/xtask/src/commands/ci/task_runner.rs` declares the types and re-exports what this folder
defines.

## Contents

```text
tools_v2/xtask/src/commands/ci/task_runner/
└── split_cmd.rs  the step runner and child environment, `help`, the gate list
```

## How it works

`run(Some(name))` finds the row in `TASKS` and runs its steps in order, stopping at the first
non-zero code, which it returns; `run(None)` prints `help` and returns 0; an unknown name
returns 2. Each step kind runs as follows:

| Step | Echo | Runs |
|---|---|---|
| `Task(name)` | `cargo xtask ci <name>` | the named row, through the same `run_task_in` |
| `Cmd { line }` | the line, unless silent | `split_cmd` splits a leading `cd <dir> && ` off and spawns the rest by whitespace, no shell |
| `Xtask { echo, run }` | the echo, unless silent | `run()` in process; an `Err` prints `xtask: <error chain>` and counts as 1 |
| `Native { run }` | nothing | `run()` in process |
| `Shell { script }` | the script, unless silent | `/bin/sh -c <script>`; `ignore_err` turns a failure into 0 |

`spawn` runs a child in the checkout (or the `cd` folder under it) with inherited stdio, sets
`PWD` to that folder, strips the variables `cargo run` injects (`CARGO_RUN_INJECTED` and every
`CARGO_PKG_*`), prepends `~/.cargo/bin`, `~/.local/go/bin` and `~/go/bin` to `PATH`, and exports
the primary checkout's `target/` as `CARGO_TARGET_DIR` unless one is already set. A child that
cannot start returns 127; one killed by a signal returns 128 plus the signal number.

`help` prints the rows grouped as CI, schema, verify, map, build and db, tags alias and borrowed
rows, and lists the `mk` targets and the `db` commands.
`schema_list_gates` prints the last word of each `cargo xtask schema …` step of the
`schema-validate` row.

## Boundaries

- Depends on: `Task`, `Step`, `Lane`, `TASKS` and `CARGO_RUN_INJECTED` in `task_runner.rs`;
  `verification_core` (`NotRun`); `TARGETS` in `tools_v2/xtask/src/commands/build/recipes.rs`
  and `LANE_COMMANDS` in `tools_v2/xtask/src/commands/db/operations.rs` for `help`.
- Used by: `task_runner.rs`, whose re-exports serve `tools_v2/xtask/src/cli/dispatch.rs`
  (`ci`, `help`), `tools_v2/xtask/src/commands/schema/dispatch.rs` (`schema list-gates`),
  `tools_v2/xtask/src/commands/platform/wave_execution/schema.rs` and
  `tools_v2/xtask/src/verifications/ci/schema_parity/source_audit.rs`.
- Rules: a composite stops at its first failing step and returns that step's code
  (`a_failing_leaf_fails_the_composite`); an unknown task named by a composite refuses with 2
  rather than skipping (`every_composite_step_resolves` keeps the table free of one); every row
  appears in `help` (`help_lists_every_task`).
