# Xtask repository commands

The `xtask` crate: the `cargo xtask` command router of the repository. It runs the repository's
operations (database, deploys, mod servers, map pipelines,
[ticket](/documentation_v2/glossary/n_to_z.md#ticket) and [wave](/documentation_v2/glossary/n_to_z.md#wave)
commands, the platform factory) and orchestrates its verifications and CI tasks. Developers, AI agents, the GitHub workflows and the host's systemd
timers all run it.

## Contents

```text
tools_v2/xtask/
├── Cargo.toml                  the `xtask` package: one binary, path dependencies on three tooling crates
├── dedicated_server_profiles/  the dedicated-server profile the local mod servers start from
├── deploy/                     deploy settings template, Caddy site and systemd units for the hosts
├── fixtures/                   recorded tool output that the commands' selftests replay
└── src/                        the binary: command tree, command groups, verifications, shared core
```

## How it works

`cargo xtask` is the alias `run --package xtask --` in `.cargo/config.toml`, so every call builds
the crate if needed and runs `src/main.rs`. The binary parses the command line with clap
(`src/cli/`), finds the checkout by walking up from the working directory to `.ai/tickets/ROOT`,
and hands the command to its group under `src/commands/`, which does the work, runs checks from
`src/verifications/`, or calls a library crate. The data folders beside `src/` are what the
commands read: `deploy/` for the deploys, `dedicated_server_profiles/` for the local game servers,
`fixtures/` for the MCP selftest; `src/core/repository_layout.rs` names each of them once.

The crate owns repository operations and the orchestration of checks. Ticket storage and the wave
lock belong to `ticket-engine`, process and verdict primitives to `verification-core`, and
engine-backed map, world and blueprint work to `developer-tools`, which alone reaches the map
engine; xtask passes them the checkout root and keeps their results and exit codes.

## Getting started

From the repository root:

```bash
cargo xtask --help                              # every command group
cargo xtask help                                # every ci task, the mk targets and the db commands
cargo test --locked -p xtask -- --test-threads=1  # the crate's tests, one thread
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
  (`src/core/cargo_target_directory.rs`), and `mk rust-api` builds into `target-dev-api`.
- `.ai/tickets/ROOT`: the marker that identifies the checkout root (`find_repo_root` in
  `tools_v2/ticket-engine/src/repository.rs`).
- `deploy/deploy.env`: the deploy host, credentials and remote paths, copied from
  `deploy/deploy.env.example` and never committed; `cargo xtask deploy website` also accepts a
  `DEPLOY_ENV` variable naming another file. The deploy README lists every key.
- Command-level variables (`TBD_FETCH_DELAY`, `TBD_BACKUP_DIR`, `PORT` for `ci editor-api-boot`
  and the rest): each group's README lists the ones its commands read.

## Public surface

- The `xtask` binary and its command groups, listed in the
  [command line README](/tools_v2/xtask/src/cli/README.md); the crate has no library target.
- The files other tools read directly: `deploy/Caddyfile.website`, pinned by
  `apps/website/api_v2/tests/forwarded_for_trust.rs`, and `deploy/systemd/`, installed on the
  hosts.

## Boundaries

- Depends on: `ticket-engine`, `developer-tools` and `verification-core` by path; clap, serde,
  `jsonschema`, and `typify`, `schemars`, `syn` and `prettyplease` for the contract codegen; at
  run time cargo, trunk, podman, git and the other host tools each group names.
- Used by:
  - people and AI agents, through the alias;
  - the workflows in `.github/workflows/` (`ci.yml`, `contracts.yml`, `editor-gates.yml`,
    `mod-gates.yml`, `schema.yml`);
  - the backup and drill units in `deploy/systemd/`, which run `cargo run -q -p xtask --`;
  - the PreToolUse hook in `.claude/settings.json`, which runs the built binary's `ai guard`;
  - the ticketboard in `apps/ticketboard/`, which runs `cargo xtask ticket` commands.
- Rules:
  - xtask never depends on `website-map-engine` or `website-graphics-engine`, `developer-tools`
    never depends on xtask, and `ticket-engine` and `verification-core` depend on no workspace
    crate (`tooling_dependency_direction_is_enforced` and
    `foundational_engines_have_no_workspace_dependencies` in
    `src/tests/tooling_dependency_boundaries.rs`);
  - production files stay under 500 lines, test files under 1000 and `src/main.rs` under 150, and
    tests live in separate files (`tooling_source_files_stay_below_their_structural_limits`,
    `tooling_test_modules_live_in_separate_files`);
  - tests read fixtures from the checkout they run in: the execution receipts in
    `tools_v2/ticket-engine/tests/fixtures/execution_receipts/` and the blueprint fixtures in
    `tools_v2/developer-tools/test_fixtures/blueprint/`.

## Related documentation

- [Local development](/documentation_v2/runbooks/local_development.md) — the everyday command
  sequence.
- [Website deployment](/documentation_v2/runbooks/website_deployment.md) — the website deploy
  and the host setup.
- [Factory waves](/documentation_v2/runbooks/factory_waves/README.md) — the platform wave commands.
- [Tooling architecture](/documentation_v2/tools_v2/tooling_architecture.md) — the router's place
  among the tooling crates, and the verification surface it runs.
