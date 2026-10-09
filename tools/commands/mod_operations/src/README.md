# Mod commands

The `cargo xtask mod` group: gates, launchers and workstation setup for the
[Enfusion](/documentation/glossary/a_to_f.md#enfusion) [mod](/documentation/glossary/g_to_m.md#mod). It
compiles the scripts and boots the world headless, runs a local playtest server, prepares a
[Workbench](/documentation/glossary/n_to_z.md#workbench) session, publishes the Workbench equipment
export, and drives the mod program's [waves](/documentation/glossary/n_to_z.md#wave). Mod developers,
mod slice agents and the `mod-gates` CI workflow run them.

## Contents

```text
tools/commands/mod_operations/src/
├── compile/                   the compile gate's server run, error triage and load-count guard
├── compile.rs                 `mod compile`: options, help and the selftest and probe addon templates
├── compile_help.txt           the help text `mod compile --help` prints
├── compile_host.rs            host bridge, process-group kill and signal cleanup for the compile gate
├── development_bootstrap.rs   `mod dev-bootstrap`: MCP package, game root, Workbench launch, MCP warm-up
├── development_server.rs      `mod dev-server`: the argument gate in front of `mod playtest`
├── equipment_gameplay/        the gameplay selection policy: table generation, projection, validation
├── equipment_vehicle_export/  validation and sealed publication of Workbench equipment exports
├── error.rs                   the crate's `Error` and `Result`, the refusal macros and the step context
├── game_runtime_api_smoke.rs  `mod test-game-runtime-api`: the game-runtime routes with a server credential
├── lib.rs                     the crate root: the module tree and the public entries
├── mission_test.rs            `mod test-mission`: the Workbench profile's cached mission artifact
├── mod_command.rs             the `ModCmd` clap enum: every subcommand, its flags and help line
├── mod_dispatch.rs            routes each `ModCmd` to its implementation
├── playtest_server/           the local playtest server's staging, boot, lifecycle and deployment
├── playtest_server.rs         `mod playtest`: help text, options and module wiring
├── prelude.rs                 the names a caller imports with one `use`
├── server_launcher.rs         a server launcher's line drain and its stop after the server's group is killed
├── tests/                     unit tests for the command modules in this folder
├── wave_execution/            the mod wave driver's status, prep, gate, land and push
├── wave_execution.rs          `mod wave`: help text, worktree states and module wiring
├── website_api_client/        the website API client the playtest, world boot and smoke tests use
├── world_boot/                the headless world boot driver and its compiled-mission lane
├── world_boot.rs              `mod world-boot`: options, run state and module wiring
└── world_boot_verdict.rs      the world boot's log verdict and its offline `--selftest` fixtures
```

## How it works

`tools/xtask/src/cli/mod.rs` mounts `ModCmd` as the `mod` group, and the crate's `run` maps each
variant to one entry function that returns the exit code. Five subcommands (`dev-server`,
`playtest`, `compile`, `world-boot`, `wave`) take their arguments raw, with clap's help flag
turned off, and parse them in their own module. The others parse with clap. Five delegate to
modules outside this folder: `remote-logs` to `remote_debugging::debug::remote_logs`,
`spawn-determinism` and `spawn-verify` to `mod_script_checks`,
`bootstrap-staging` to `workstation_setup::staging_server` and `seed-announcement` to
`database_operations::milestone_announcement`.

The game-facing gates share one exit contract: 0 pass, 1 a code failure in the mod, 2 usage or
no verdict, 3 environment (no host bridge, no dedicated server, no Workbench), so a machine fault
never reads as broken mod code. `compile`, `world-boot` and `playtest` run the Linux dedicated
server from `$HOME/.local/share/Steam/steamapps/common/Arma Reforger Server` through the
container-to-host bridge (`distrobox-host-exec` or `host-spawn`). Each starts it under `setsid`
and stops its whole process group.

```text
mod compile ──▶ mod world-boot ──▶ mod playtest
 scripts compile   world loads, roll-call    a joinable server
 (Game module)     and mission validate      running a deployed mission
```

An error that escapes an entry function prints `xtask: <cause>` and exits 1; a clap usage error
exits 2.

## Commands

Run each as `cargo xtask mod <subcommand>` from the repository root.

### generate-equipment-gameplay-policy

- Synopsis: `mod generate-equipment-gameplay-policy [--check]`
- Does: generates the export addon's gameplay selection tables in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Gameplay/Policy/Generated/`
  from `contracts/rules/equipment-gameplay/`; with `--check` it writes nothing and compares.
- Exit codes: 0 written, or unchanged under `--check`; 1 the policy could not be read, or with
  `--check` a table differs or an extra file is present.
- Example: `cargo xtask mod generate-equipment-gameplay-policy --check`

### project-equipment-gameplay

- Synopsis: `mod project-equipment-gameplay --input <INPUT> --output <OUTPUT>`
- Does: projects a complete source generation through the gameplay selection policy into a new
  gameplay generation folder, which must not exist yet.
- Exit codes: 0 projected; 1 the input is not a complete source generation, the output exists, or
  the projection failed.
- Example: `cargo xtask mod project-equipment-gameplay --input <generation folder> --output <new folder>`

### validate-equipment-vehicle-export

- Synopsis: `mod validate-equipment-vehicle-export --input <INPUT>`
- Does: validates a Workbench equipment and vehicle generation folder against
  `contracts/definitions/equipment-vehicle-export.schema.json` and its census rules, or a
  gameplay generation against the gameplay selection policy (`equipment_gameplay/`), and prints
  the report as JSON.
- Exit codes: 0 valid; 1 invalid, or the input could not be read.
- Example: `cargo xtask mod validate-equipment-vehicle-export --input <profile>/TBD_Export/equipment_vehicle_exports/generations/<id>`

### publish-equipment-vehicle-export

- Synopsis: `mod publish-equipment-vehicle-export --input <INPUT>`
- Does: validates again, then copies the generation into `published/<id>/` with a
  `manifest.json` and swaps `current.json` to it, under a lock and with every copy hash-checked.
- Exit codes: 0 published; 1 invalid, or publication refused or failed.
- Example: `cargo xtask mod publish-equipment-vehicle-export --input <generation folder>`

### remote-logs

- Synopsis: `mod remote-logs [--file <FILE>] [--selftest]`
- Does: reads the staging server's `console.log` over SSH from `TBD_SSH_HOST` in `deploy.env`
  (or `--file` locally) and classifies the boot; `--selftest` proves the verdict can fail.
- Exit codes: 0 healthy; 1 fail, or a missing or malformed setting; 2 partial; 3 environment.
- Example: `cargo xtask mod remote-logs --file console.log`

### spawn-determinism

- Synopsis: `mod spawn-determinism [--preflight] [--selftest] [RUNS] [WORLD]`
- Does: opens `WORLD` (default `worlds/TBD_Dev_POC.ent`) in Workbench through the MCP bridge,
  plays it `RUNS` times (default 5) and compares the spawn and equip lines of each run;
  `--selftest` checks the per-run verdict offline.
- Exit codes: 0 deterministic; non-zero on a failed run; `--preflight` exits 2 when the Workbench
  Net API is not listening.
- Example: `cargo xtask mod spawn-determinism --selftest`

### spawn-verify

- Synopsis: `mod spawn-verify [--selftest] [PATTERN]`
- Does: plays the open world in Workbench for 25 s through `mcp call wb_play` and `wb_stop`, then
  returns the verdict of `cargo xtask mcp wb-logs`, filtered for display by `PATTERN`.
- Exit codes: those of `mcp wb-logs`.
- Example: `cargo xtask mod spawn-verify --selftest`

### dev-bootstrap

- Synopsis: `mod dev-bootstrap [--api] [--server]`
- Does: sets up the MCP game root, installs the pinned `enfusion-mcp` package, launches Workbench
  on `apps/mod/tbd-export/addon.gproj` when its Net API port (`ENFUSION_WORKBENCH_PORT`, 5775) is
  closed, warms the MCP daemon, checks `wb_connect`, and validates both addons. `--api` runs
  `cargo xtask db up` (reporting a failure) and then a detached `cargo xtask mk rust-api`;
  `--server` runs `setup server-profile` and then `mod dev-server` with no arguments. Every step
  but the Net API check and `wb_connect` reports a failure and continues.
- Exit codes: 0 ready; 1 the Net API never opened, `wb_connect` failed, or the `tbd-emcp`
  handlers are missing.
- Example: `cargo xtask mod dev-bootstrap`

### dev-server

- Synopsis: `mod dev-server [ARGS]...`
- Does: with no arguments prints the playtest usage and the staging runbook path; with any,
  hands them to `mod playtest`.
- Exit codes: 2 with no arguments; otherwise those of `mod playtest`.
- Example: `cargo xtask mod dev-server --artifact-file=contracts/fixtures/missions/valid/bridgehead-at-levie.json --dry-run`

### test-mission

- Synopsis: `mod test-mission [TARGET]`
- Does: in the Workbench profile under Proton, shows the backend config and cached artifact (no
  target), stages the named mission fixture from `contracts/` as the cached artifact (with the
  framework registry copied beside it), or clears the cache (`backend`).
- Exit codes: 0 done; 1 no profile config, `HOME` unset, or no fixture of that name.
- Example: `cargo xtask mod test-mission bridgehead-at-levie`

### bootstrap-staging

- Synopsis: `mod bootstrap-staging`
- Does: discovers the staging host named by `TBD_SSH_HOST` in
  `deploy/deploy.env` and creates the folders the deploy expects there (by default
  under `/home/<user>/tbd/`).
- Exit codes: 0 done; 1 no host, a deploy file that does not load, or a folder that cannot be
  resolved or is refused; 127 no `ssh` or `sshpass`.
- Example: `cargo xtask mod bootstrap-staging`

### seed-announcement

- Synopsis: `mod seed-announcement`
- Does: inserts the pinned first-milestone announcement into the website database when it is not
  there, using `DATABASE_URL` from the environment or `crates/api/api_server/.env`.
- Exit codes: 0 inserted or present; 1 no `psql` and no database container, or no `DATABASE_URL`.
- Example: `cargo xtask mod seed-announcement`

### test-game-runtime-api

- Synopsis: `mod test-game-runtime-api`, configured by `TBD_MACHINE_CREDENTIAL` (required) and
  `TBD_API_BASE` (default `http://127.0.0.1:8080`).
- Does: calls `GET /api/v1/game-runtime/deployment` with the server's `mod_runtime`
  [machine credential](/documentation/glossary/g_to_m.md#machine-credential); with a deployment,
  checks that `GET /api/v1/game-runtime/artifacts/{artifactId}` returns bytes hashing to the
  deployment's SHA-256 and entity tag, and that an event's
  `GET /api/v1/game-runtime/events/{id}/roster` answers version 2; checks that the deployment read
  without a credential answers 401.
- Exit codes: 0 every check passed; 1 a check failed; 2 no credential; 3 the API gave no answer.
- Example: `TBD_MACHINE_CREDENTIAL=<credential> cargo xtask mod test-game-runtime-api`

### playtest

- Synopsis: `mod playtest --mission=<uuid> [options]`, `mod playtest --artifact-file=<p> [options]`
  or `mod playtest --selftest`; `--help` lists every option.
- Does: stages `$HOME/tbd-playtest`, deploys the mission through the platform (or stages the
  offline document), renders `server.json`, boots a joinable dedicated server with the local
  addon and prints its Direct Join details; it stays in the foreground until Ctrl-C.
  `--dry-run` boots nothing. With `--mission` it prints the game runtime's telemetry queue
  reading once the deployment is confirmed and, when the server stops, checks that every match
  it saw holds acknowledged events and that the queue drained; `--require-telemetry` makes a
  failed check fail the run.
- Exit codes: 0 booted and stopped cleanly; 1 died, refused, wrong addon copy, not confirmed
  stopped, or a failed telemetry check under `--require-telemetry`; 2 usage; 3 environment.
- Example: `cargo xtask mod playtest --artifact-file=contracts/fixtures/missions/valid/bridgehead-at-levie.json --dry-run`

### compile

- Synopsis: `mod compile [--selftest] [--keep-logs] [--probe=DIR]`
- Does: compiles the Game scripts of `apps/mod/tbd-framework` with the headless dedicated server
  and prints each error as `file:line`; `--probe` also compiles a throwaway addon of `.c` files.
- Exit codes: 0 clean; 1 compile errors, or `tbd-framework/Scripts/WorkbenchGame` exists; 2 no
  verdict; 3 environment, including the engine loading fewer script files than the tree holds (a
  missing or unreadable `resourceDatabase.rdb`).
- Example: `cargo xtask mod compile`

### compile-selftest

- Synopsis: `mod compile-selftest`
- Does: runs `mod compile --selftest` in-process and passes only when the gate rejects the
  deliberately broken source.
- Exit codes: 0 the gate exited 1; 1 any other gate exit.
- Example: `cargo xtask mod compile-selftest`

### compile-preflight

- Synopsis: `mod compile-preflight`
- Does: checks that the dedicated server binary and a non-empty
  `apps/mod/tbd-framework/resourceDatabase.rdb` exist, printing GitHub error annotations.
- Exit codes: 0 both present; 1 either missing.
- Example: `cargo xtask mod compile-preflight`

### world-boot

- Synopsis: `mod world-boot [--selftest] [--keep-logs] [--mission=<file|name> | --compiled[=<uuid>]]`
- Does: boots the dedicated server headless with the local addon and asserts the world loads, the
  framework roll-call is clean and no TBD script errors; with a mission, also that it validated
  within its warning budget; `--compiled` seeds a mission through the API and checks the
  four-weapon equip; `--selftest` runs the verdict against fixtures with no engine.
- Exit codes: 0 pass; 1 code failure; 2 usage; 3 environment.
- Example: `cargo xtask mod world-boot --mission=bridgehead-at-levie`

### wave

- Synopsis: `mod wave [status | gate | land | prep [N] | push]` (default `status`)
- Does: reports the mod program's current wave and each slice's worktree state; runs the mod wave
  gate; lands (merge, gate, reap, push); creates a wave's worktrees; pushes `main`.
- Exit codes: 0 done; 1 a refusal or failed gate, merge or push; 2 a missing lock or an unknown
  subcommand, which prints the help.
- Example: `cargo xtask mod wave status`

## Boundaries

- Depends on:
  - `repository_layout`, `process_runner::host_execution`, and `tool_test_support` in its tests;
  - `remote_debugging::debug::remote_logs`, `workstation_setup` (`staging_server`,
    `mcp_game_root`), `database_operations::milestone_announcement`,
    `enfusion_mcp::daemon` and `platform_execution::slice_worktree`;
  - `mod_script_checks` (`spawn_determinism`, `spawn_verification`);
  - the `ticket_model`, `ticket_wave_lock`, `ticket_registry`, `process_runner` (every child
    process) and `verification_core` crates;
  - the Arma Reforger dedicated server and Workbench under Steam, `curl`, `git`, `npm`, and the
    website API for the platform lanes.
- Used by:
  - `tools/xtask/src/cli/mod.rs` and `tools/xtask/src/cli/dispatch.rs`, which mount the group;
  - `.github/workflows/mod-gates.yml` (`compile-preflight`, `compile-selftest`, `compile`,
    `world-boot`) and `.github/workflows/ci.yml` (`world-boot --selftest`);
  - `mod wave gate`, which runs `compile`, `compile-selftest` and `world-boot`, and
    `mod dev-bootstrap --server`, which runs `mod dev-server`;
  - mod developers and slice agents.
- Rules:
  - The game-facing gates keep the 0/1/2/3 contract, and an environment fault never exits 1
    (`no_server_is_rc3` in `tests/compile/tests.rs`).
  - `compile-selftest` passes only on the gate's exit 1 (`run_selftest` in
    `compile/execution.rs`).
  - `mod dev-server` with no arguments exits 2 (`no_args_is_rc2` in
    `tests/development_server/tests.rs`).
  - `mod playtest --help` lists exactly the parsed flags (`help_text_matches_the_options_we_parse`
    in `tests/playtest_server/tests.rs`).
  - Unit tests live under `tests/`, never inline.

## Related documentation

- [Mod slice workflow](/documentation/runbooks/mod_slice_workflow.md) — the slice and wave
  cycle `mod wave` automates.
- [Two-client playtest](/documentation/runbooks/two_client_playtest/README.md) — a playtest a
  second client joins, with `mod playtest`.
- [Playtest server](/documentation/runbooks/two_client_playtest/playtest_server.md) and
  [headless preflight](/documentation/runbooks/two_client_playtest/headless_preflight.md) —
  running `mod playtest` step by step, and reading a `mod world-boot` of a loadout-carrying
  mission.
- [Game server staging](/documentation/runbooks/game_server_staging/README.md) — the staging
  server `bootstrap-staging` and `remote-logs` serve.
- [Boot and log verification](/documentation/runbooks/game_server_staging/boot_and_log_verification.md)
  — `remote-logs` by hand and the log line prefixes to match.
- [Machine credentials and mission deployment](/documentation/runbooks/game_server_staging/machine_credentials_and_mission_deployment.md)
  — the credential `test-game-runtime-api` needs, and what it checks.
- [Enfusion MCP tooling](/documentation/runbooks/enfusion_mcp_tooling.md) — the MCP bridge
  `dev-bootstrap` and the spawn checks use.
- [Spawn determinism](/documentation/runbooks/spawn_determinism.md) — running
  `mod spawn-determinism`.
- [Mod suite](/apps/mod/README.md) — the addons these commands compile, boot and export from.
