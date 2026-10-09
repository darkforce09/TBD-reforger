# Xtask command line

The top of the `cargo xtask` command tree: the clap parser that names every command group, and
the dispatch that hands each parsed group to the folder under `tools/xtask/src/commands/` that
owns it.

## Contents

```text
tools/xtask/src/cli/
├── command_vocabulary.rs  the command tree, recipe names and task names the documentation link check judges citations against
├── dispatch.rs            `run`: preprocess the arguments, parse them, route each group to its handler
├── mod.rs                 the `Cli` parser and the `TopCmd` enum of every top-level command
└── tests/                 parse tests of the documentation gate arguments and `mcp wb-logs --file`, and the link check over this tree
```

## How it works

`main` in `tools/xtask/src/main.rs` calls `dispatch::run`, which passes the raw arguments
through `enfusion_mcp::preprocess_cli_args` (`tools/commands/enfusion_mcp/src/workbench_logs.rs`;
it keeps an empty `mcp wb-logs --file` value distinguishable), parses them into `Cli`, and matches the
`TopCmd` variant. A group variant holds the group's own clap `Subcommand` enum, declared in that
group's `cli.rs`, and dispatch calls the group's `dispatch::run`. Handlers return `Result<u8>`:
`main` exits with the code, or prints `xtask: <error chain>` and exits 1 on an error; a clap usage
error exits 2 before any handler runs.

`disable_help_subcommand` frees the name `help` for `cargo xtask help`, the task list; `--help`
and `-h` work on every level that does not turn them off. Three commands take their arguments
raw instead of as a clap tree: `mk` (the target list lives in
`tools/commands/ci_task_catalog/src/build_lane/recipes.rs`), `ci` (one optional task name) and
`slice-collisions`.

| Command | What it holds | Module in `tools/xtask/src/commands/` |
|---|---|---|
| `ticket` | the ticket registry commands | `ticket` |
| `mcp` | the Enfusion MCP bridge: daemon, calls, selftest, Workbench logs | none: the `enfusion_mcp` crate (`tools/commands/enfusion_mcp/`) |
| `debug` | server-join probes and their primitives | `debug` |
| `repro` | mission upload reproduction helpers | `reproduction` |
| `mod` | mod compile, servers, mission tests, world boot, mod wave | none: the `mod_operations` crate (`tools/commands/mod_operations/`) |
| `deploy` | website and staging deploys, database backup and restore | `deploy` |
| `db` | the local Postgres lane | `db` |
| `setup` | local and dedicated-server profile setup | `setup` |
| `fetch` | vanilla source and API reference mirrors | `fetch` |
| `map` | map-asset pipeline helpers | `map` |
| `registry-get` | prints one top-level field of the ticket registry | inline in `dispatch.rs`, via `load_registry` |
| `schema` | contract codegen, contract and map-asset gates, mission-file tools | `schema` |
| `verify` | the repository verifications | `verify` |
| `gen` | code generators | `generate` |
| `slice-collisions` | the largest file-disjoint set of tickets | `wave` (`collisions`) |
| `wave` | the wave lock: `repack` and `check` | `wave` |
| `platform` | the platform factory: slice worktrees, slice runs, platform waves | `platform` |
| `ai` | the agent tool-call guard and the filtered command runner | `agent_context` |
| `mk` | build and development-server recipes | none: the `ci_task_catalog` crate (`tools/commands/ci_task_catalog/`, `build_lane`) |
| `ci` | the CI, composite and map task table | none: the `ci_task_catalog` crate (`task_runner`) |
| `help` | lists every `ci` task by group, and points at `mk` and `db --help` | none: the `ci_task_catalog` crate (`task_runner::help`) |

The documentation link check judges every cited `cargo xtask` command against this binary's own
tree without reading the command line itself: `command_vocabulary.rs` hands it `Cli`'s clap
tree, the `mk` recipe names and the `ci` task names. `verify link-check` passes that vocabulary
to the gate, and the `ci` arm of `dispatch.rs` hands the tree to the task runner, whose in-process
link-check step builds the same vocabulary.

## Boundaries

- Depends on: clap's derive API; each group's `cli.rs` and `dispatch.rs` under
  `tools/xtask/src/commands/`; the recipe and task tables and the link check's
  `CommandVocabulary` for `command_vocabulary.rs`; `find_repository_root` in `crates/foundation/repository_root/src/root_marker_walk.rs` (through `repository_layout::prelude`)
  and `load_registry` for `registry-get`.
- Used by: `tools/xtask/src/main.rs`, the only caller of `dispatch::run`; people and the CI
  workflows run the binary through the `cargo xtask` alias in `.cargo/config.toml`, the backup
  units in `deploy/systemd/` through `cargo run -q -p xtask --`, and the agent
  hook in `.claude/settings.json` through the built binary.
- Rules: a command group's subcommands stay in that group's `cli.rs`, and this folder holds only
  the top-level names and the routing; a top-level name is stable, because workflows, units,
  documents and `link-check` (which checks every cited `cargo xtask` command against this tree)
  name it.

## Related documentation

- [Local development](/documentation/runbooks/local_development.md) — the everyday commands.
