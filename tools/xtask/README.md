# Xtask repository commands

The `xtask` crate: the `cargo xtask` command router of the repository. It runs the repository's
operations (database, deploys, mod servers, map pipelines,
[ticket](/documentation/glossary/n_to_z.md#ticket) and [wave](/documentation/glossary/n_to_z.md#wave)
commands, the platform factory) and orchestrates its verifications and CI tasks. Developers, AI agents, the GitHub workflows and the host's systemd
timers all run it.

## Contents

```text
tools/xtask/
├── Cargo.toml                  the `xtask` package: one binary, path dependencies on three tooling crates
├── dedicated_server_profiles/  the dedicated-server profile the local mod servers start from
├── fixtures/                   recorded tool output that the commands' selftests replay
├── src/                        the binary: command tree and command groups
└── staging/                    committed load workload and population the staging load receipt runs
```

## How it works

`cargo xtask` is the alias `run --package xtask --` in `.cargo/config.toml`, so every call builds
the crate if needed and runs `src/main.rs`. The binary parses the command line with clap
(`src/cli/`), finds the checkout by walking up from the working directory to `.ai/tickets/ROOT`,
and hands the command to its group under `src/commands/`, which does the work or calls a library
crate (the check crates under `tools/checks/`, the command crates under `tools/commands/`). The data folders beside `src/` are what the
commands read: `dedicated_server_profiles/` for the local game servers and `fixtures/` for the MCP
selftest; the deploys read the repository root's `deploy/`. The `repository_layout` crate
(`tools/foundation/repository_layout`) names each of them once, and `deploy_settings`
(`tools/foundation/deploy_settings`) reads `deploy/deploy.env`.

The crate owns repository operations and the orchestration of checks. Ticket storage and the wave
lock belong to the ticket crates in `tools/tickets/`, verdict primitives to `verification_core`, child processes, the
host bridge and the ssh transport to `process_runner`, the structural laws to `repository_laws`, and
building blueprints to `blueprint_compiler`, the map asset gates to `map_asset_verification`, the
terrain export driver and the map tile index to `world_export_pipeline`, and the world export
stages to the `world` binary of `developer_tools`, which that driver runs as a child process;
xtask passes them the checkout root and keeps their results and exit codes.

## Getting started

From the repository root:

```bash
cargo xtask --help                              # every command group
cargo xtask help                                # every ci task, the mk targets and the db commands
cargo xtask db up                               # local Postgres on :5434, needed by ci-local
cargo xtask ci ci-local                         # the CI replay: language gates, Rust, frontend, schema
cargo xtask mk leptos-gates                     # the browser gates, run apart from ci-local
```

`cargo xtask ci ci-local` waits on `db up`, because its `rust-ci` step runs the API's
integration tests against a scratch database in the `tbd_reforger_db` container.
`cargo xtask mk leptos-gates` needs the browser environment that its `gate doctor` step checks.
Commands that change something outside the checkout (deploys, restores, server starts) need their
services, tools and credentials; read a command's `--help` and run its `--dry-run` first where it
has one. `cargo check --locked -p xtask --all-targets` and `cargo fmt -p xtask --check` check the
crate alone.

## Configuration

The crate has no features and reads no configuration file of its own. What it reads:

- `CARGO_TARGET_DIR`: kept when set; otherwise the `mk` and `ci` children get the primary
  checkout's `target/`, shared by every linked worktree
  (`tools/commands/ci_task_catalog/src/cargo_target_pin.rs`), and `mk rust-api` builds into
  `target/dev-api`.
- `.ai/tickets/ROOT`: the marker that identifies the checkout root (`find_repository_root` in
  `crates/foundation/repository_root/src/root_marker_walk.rs`, which xtask reaches through
  `repository_layout::prelude`).
- `deploy/deploy.env`: the deploy host (`TBD_SSH_HOST`, the one place the staging host is named),
  credentials and remote paths, copied from `deploy/deploy.env.example` and never committed. Every
  command that reads it (`src/core/deploy_environment.rs`) lets the file decide every key it
  assigns and the process environment fill only the others, and takes a `DEPLOY_ENV` variable
  naming another file. The deploy README lists every key.
- Command-level variables (`TBD_FETCH_DELAY`, `TBD_BACKUP_DIR`, `PORT` for `ci editor-api-boot`
  and the rest): each group's README lists the ones its commands read.

## Public surface

- The `xtask` binary and its command groups, listed in the
  [command line README](/tools/xtask/src/cli/README.md); the crate has no library target.

## Boundaries

- Depends on: the tool crates `tools/xtask/Cargo.toml` lists (the ticket crates, the check and
  command crates, `blueprint_compiler`, `map_asset_verification`, `world_export_pipeline`,
  `enfusion_script_index` and the
  `tools/foundation` crates), all by workspace path; anyhow, clap and serde; at run time cargo, trunk, podman, git and the other host tools each group names. No
  tokio, axum, reqwest, resvg or image enters its dependency closure (rule 6 of
  `cargo xtask verify crate-tiers`).
- Used by:
  - people and AI agents, through the alias;
  - the workflows in `.github/workflows/` (`ci.yml`, `editor-gates.yml`, `mod-gates.yml`);
  - the backup and drill units in `deploy/systemd/`, which run `cargo run -q -p xtask --`;
  - the PreToolUse hook in `.claude/settings.json`, which runs the built binary's `ai guard`;
  - the ticketboard desktop viewer `tools/tickets/ticketboard_desktop/`, which runs
    `cargo xtask ticket` commands.
- Rules:
  - xtask and `developer_tools` are binary-only packages over tool crates alone (the checkout-root
    finder comes through `repository_layout`), neither depends on the other, and no member depends on either; a
    `tools/foundation` crate depends only on lower `tools/foundation` crates and `repository_root`,
    and a `tools/tickets` crate only on `tools/foundation` crates, four `crates/foundation` crates
    and lower ticket crates;
  - production files should stay under 500 lines (`cargo xtask verify file-length` warns), and
    tests live in separate files;
  - tests read fixtures from the checkout they run in: the blueprint fixtures in
    `tools/map_assets/blueprint_compiler/test_fixtures/blueprint/`.

## Related documentation

- [Local development](/documentation/runbooks/local_development.md) — the everyday command
  sequence.
- [Website deployment](/documentation/runbooks/website_deployment.md) — the website deploy
  and the host setup.
- [Factory waves](/documentation/runbooks/factory_waves/README.md) — the platform wave commands.
- [Tooling architecture](/documentation/tools/tooling_architecture.md) — the router's place
  among the tooling crates, and the verification surface it runs.
