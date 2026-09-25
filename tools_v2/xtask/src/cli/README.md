# Xtask command line

The top of the `cargo xtask` command tree: the clap parser that names every command group, and
the dispatch that hands each parsed group to the folder under `tools_v2/xtask/src/commands/` that
owns it.

## Contents

```text
tools_v2/xtask/src/cli/
├── dispatch.rs  `run`: preprocess the arguments, parse them, route each group to its handler
└── mod.rs       the `Cli` parser and the `TopCmd` enum of every top-level command
```

## How it works

`main` in `tools_v2/xtask/src/main.rs` calls `dispatch::run`, which passes the raw arguments
through `preprocess_cli_args` from `tools_v2/xtask/src/commands/mcp/workbench_logs.rs` (it keeps
an empty `mcp wb-logs --file` value distinguishable), parses them into `Cli`, and matches the
`TopCmd` variant. A group variant holds the group's own clap `Subcommand` enum, declared in that
group's `cli.rs`, and dispatch calls the group's `dispatch::run`. Handlers return `Result<u8>`:
`main` exits with the code, or prints `xtask: <error chain>` and exits 1 on an error; a clap usage
error exits 2 before any handler runs.

`disable_help_subcommand` frees the name `help` for `cargo xtask help`, the task list; `--help`
and `-h` work on every level that does not turn them off. Three commands take their arguments
raw instead of as a clap tree: `mk` (the target list lives in
`tools_v2/xtask/src/commands/build/recipes.rs`), `ci` (one optional task name) and
`slice-collisions`.

| Command | What it holds | Module in `tools_v2/xtask/src/commands/` |
|---|---|---|
| `ticket` | the ticket registry commands | `ticket` |
| `mcp` | the Enfusion MCP bridge: daemon, calls, selftest, Workbench logs | `mcp` |
| `debug` | server-join probes and their primitives | `debug` |
| `repro` | mission upload reproduction helpers | `reproduction` |
| `mod` | mod compile, servers, mission tests, world boot, mod wave | `mod_ops` |
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
| `mk` | build and development-server recipes | `build` |
| `ci` | the CI, composite and map task table | `ci` |
| `help` | lists every `ci` task by group, and points at `mk` and `db --help` | `ci` (`help`) |

## Boundaries

- Depends on: clap's derive API; each group's `cli.rs` and `dispatch.rs` under
  `tools_v2/xtask/src/commands/`; `find_repo_root` in `tools_v2/xtask/src/core/repository_root.rs`
  and `load_registry` for `registry-get`.
- Used by: `tools_v2/xtask/src/main.rs`, the only caller of `dispatch::run`; people and the CI
  workflows run the binary through the `cargo xtask` alias in `.cargo/config.toml`, the backup
  units in `tools_v2/xtask/deploy/systemd/` through `cargo run -q -p xtask --`, and the agent
  hook in `.claude/settings.json` through the built binary.
- Rules: a command group's subcommands stay in that group's `cli.rs`, and this folder holds only
  the top-level names and the routing; a top-level name is stable, because workflows, units,
  documents and `link-check` (which checks every cited `cargo xtask` command against this tree)
  name it.

## Related documentation

- [Local development](/documentation_v2/runbooks/local_development.md) — the everyday commands.
