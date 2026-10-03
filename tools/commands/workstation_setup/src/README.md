# Workstation setup source

The four `cargo xtask setup` commands, the staging host bootstrap, the clap subcommand that names
them, the dispatch, and the errors they report.

## Contents

```text
tools/commands/workstation_setup/src/
├── client_addons.rs    `setup client-addons`: the client addon staging link and Steam launch options
├── error.rs            `Error` and `Result`, and the crate-private context trait
├── lib.rs              the crate root: module header, `mod` lines and the re-exports
├── mcp_game_root.rs    `setup mcp-game-root`: a flat folder of links to every game pak
├── prelude.rs          `SetupCmd` and `run` for glob import
├── server_profile.rs   `setup server-profile`: the dedicated-server profile and its backend config
├── setup_command.rs    the `SetupCmd` clap enum: four subcommands and their arguments
├── setup_dispatch.rs   `run`: routes each `SetupCmd` to its module
├── staging_server.rs   `mod bootstrap-staging`: discovery and directory creation on the staging host
├── tests/              unit tests for every command on throwaway homes and trees
└── workbench_linux.rs  `setup workbench`: links the Steam base game to a short home path for Proton
```

## How it works

- Each command module has a `run` entry that finds its roots (the checkout, `$HOME`, the deploy
  settings) and a `run_with_root`, `run_with_paths`, `run_in` or `run_with_environment` entry that
  takes them as arguments, so the tests run against throwaway trees.
- `client_addons` runs `mkdir -p` and `ln -sfn` through `process_runner` and passes their output
  through, so a failure shows the tool's own message and exit code; `workbench_linux` reads the
  login name from `whoami`; `staging_server` runs the discovery script over the `ssh` transport
  `deploy_settings` chooses.
- `error`: a refused input is an exit code the command returns; an `Error` is a failure the
  command cannot recover from, printed as `<step>: <cause>`.

## Boundaries

- Depends on: `repository_layout`, `deploy_settings`, `process_runner`, `verification_core`,
  `clap`, `serde_json` and `thiserror`.
- Used by: the crate root's re-exports and public modules, read by the `setup` and `mod` groups of
  `xtask`.
- Rules: the tests in `tests/` drive every command through its argument-taking entry, under
  `tool_test_support::lock_env` where they set `HOME`, `PATH` or a deploy key.
