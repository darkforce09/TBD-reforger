# Workstation setup

The `workstation_setup` crate, behind the `cargo xtask setup` group: one-time preparation of a developer machine or a server for the
[mod](/documentation/glossary/g_to_m.md#mod): the dedicated-server profile, the base-game link
[Workbench](/documentation/glossary/n_to_z.md#workbench) asks for under Proton, the pak folder the
[Enfusion](/documentation/glossary/a_to_f.md#enfusion) MCP tools read, and the client addon staging
link. The crate also holds the staging host bootstrap that `cargo xtask mod bootstrap-staging`
runs.

## Contents

```text
tools/commands/workstation_setup/
├── Cargo.toml  the `workstation_setup` library package: `clap`, `deploy_settings`, `process_runner`, `repository_layout`, layout tier 3
└── src/        the four setup commands, the staging host bootstrap, the clap subcommand, the dispatch and the errors
```

## How it works

`tools/xtask/src/cli/mod.rs` mounts `SetupCmd` as the `setup` group and `tools/xtask/src/cli/dispatch.rs`
hands it to `workstation_setup::run`, which calls each module's `run`. Every command writes only under the profile directory, `$HOME` or the paths
its arguments name, prints what it made, and returns its exit code. Each module has a
`run_with_root` or `run_with_paths` entry that takes its roots as arguments.

`staging_server` is not a `setup` subcommand: `tools/commands/mod_operations/src/mod_dispatch.rs`
sends `cargo xtask mod bootstrap-staging` to it. It reads `TBD_SSH_HOST`, `TBD_REMOTE_DIR`,
`TBD_PROFILE_DIR`, `TBD_ADDONS_STAGING`, `TBD_SSH_PASS` and `TBD_SSH_IDENTITY_FILE` from
`deploy/deploy.env` through `deploy_settings` (the file decides every
key it assigns, the environment fills the rest, and an absent file leaves everything to the
environment); the three folders default to `/home/<user>/tbd/repo`, `…/profile` and
`…/addons-staging` under the user of `TBD_SSH_HOST`. Over SSH it prints the host's disk, the
listeners on 5432, 8080 and 2001 and the container runtime, creates the three remote directories,
and prints the manual next steps (among them the API's `.env`, which needs `JWT_SECRET` and
`OBSERVABILITY_TOKEN`, and `sudo loginctl enable-linger "$USER"` on the host). It exits 1 without
a host, with a deploy file that does not load, with a folder it cannot resolve, or with a remote
directory containing `prairielearn`, and 127 when `ssh` or `sshpass` is missing.

## Commands

Run each as `cargo xtask setup <subcommand>` from the repository root. An error that escapes a
command prints `xtask: <cause>` and exits 1; a clap usage error exits 2.

### server-profile

- Synopsis: `setup server-profile [PROFILE]`; `PROFILE` defaults to `TBD_PROFILE`, then
  `mod/.local-test-profile`.
- Does: creates `<PROFILE>/profile/` (mode 700) and copies
  `mod/tbd-framework/Data/backend.example.json` to `profile/TBD_BackendConfig.json` (mode
  600); the config holds two keys, `backendUrl` and `machineCredential`. It writes
  `machineCredential` from `TBD_MACHINE_CREDENTIAL` when that is set. It then copies
  `mod/tbd-framework/Data/registry.json` to `profile/TBD_Registry.json`, best effort: the mod
  reads that copy only when its own `Data/registry.json` is missing. A profile without a
  [machine credential](/documentation/glossary/g_to_m.md#machine-credential) boots no
  [mission](/documentation/glossary/g_to_m.md#mission), since the mission comes from the server's
  [mission deployment](/documentation/glossary/g_to_m.md#mission-deployment). The Workbench checklist
  it prints last does not match that boot path: it names a
  [mission header](/documentation/glossary/g_to_m.md#mission-header) and two components to set up by
  hand.
- Exit codes: 0 profile written; 1 the backend example is missing (`cp: cannot stat …`).
- Example: `cargo xtask setup server-profile`

### workbench

- Synopsis: `setup workbench`
- Does: links `$STEAM_BASE/addons/data` (by default the Arma Reforger folder of the Steam library
  in `$HOME`) to `$HOME/ArmaReforger-Base/data`, replacing an older link, and prints the Linux and
  Proton paths to give Workbench's "Locate base game" dialog.
- Exit codes: 0 linked; 1 `ArmaReforger.gproj` is not in the base game folder.
- Example: `cargo xtask setup workbench`

### mcp-game-root

- Synopsis: `setup mcp-game-root [GAME] [FAKE]`; `GAME` defaults to
  `$HOME/.local/share/Steam/steamapps/common/Arma Reforger` and `FAKE` to
  `$HOME/.cache/enfusion-mcp-root`; with `HOME` unset, both must be given.
- Does: deletes `FAKE`, then links every `*.pak` found at any depth under `GAME/addons/` into
  `FAKE/addons/` under a flat name (each `/` of its path becomes `_`), because the enfusion-mcp
  file system reads only paks directly in `addons/`.
- Exit codes: 0 `Linked N pak files into <FAKE>/addons/`; 1 `GAME/addons` is not a folder, or
  `HOME` is unset and an argument is left out.
- Example: `cargo xtask setup mcp-game-root`

### client-addons

- Synopsis: `setup client-addons`
- Does: links `mod/tbd-framework/` into `$HOME/.local/share/tbd-server-addons/` (the link is
  made even when the target is missing) and prints the Steam launch options that load it and a
  Direct Join hint: `Direct Join → <host> (<IPv4 address>) port 2001` for the host of
  `TBD_SSH_HOST` in `deploy.env`, the host with the reason when it has no IPv4 address from here, or
  `the staging host, port 2001 (set TBD_SSH_HOST in <path> to print its address)`.
- Exit codes: 0 linked; the exit code of `mkdir` or `ln` when either fails (1 when either is
  killed by a signal), with the tool's own message passed through.
- Example: `cargo xtask setup client-addons`

## Boundaries

- Depends on: `repository_root` (the checkout root), `repository_layout` (the shared locations; the framework addon's folder and folder name
  from its `enfusion_mod_folders`), `deploy_settings` (the deploy host and the
  remote folders), `process_runner` (`ssh`, `sshpass`, `mkdir`, `ln` and `whoami`),
  `verification_core` (`NotRun`), `clap` (the subcommand), `serde_json` (the backend config) and
  `thiserror`.
- Used by:
  - `tools/xtask/src/cli/dispatch.rs`, which dispatches the group, and
    `tools/commands/mod_operations/src/mod_dispatch.rs`, for `mod bootstrap-staging`;
  - `cargo xtask deploy staging`, whose remote payload runs `setup server-profile` on the staging
    host (`tools/commands/deployment/src/staging/payloads.rs`), and `cargo xtask staging fleet
    --record`, whose `mod_runtime` credential promotion runs the same payload commands;
  - the mod, whose registry loader names `cargo xtask setup server-profile` when no registry file
    exists (`mod/tbd-framework/Scripts/Game/TBD/Core/TBD_Registry.c`);
  - people setting up a machine, following the runbooks below.
- Rules:
  - The backend config is written mode 600 inside a mode 700 folder, and the credential is written
    in place, keeping the other keys and their order.
  - A missing input exits 1 rather than succeeding.
  - The Direct Join hint never fails the command and names the host, its address or the setting
    to fill.

## Getting started

Run from the repository root:

```bash
cargo build -p workstation_setup   # every `setup` command
```

## Related documentation

- [Command crates](/tools/commands/README.md) — the command crates and their tiers.

- [Game server staging](/documentation/runbooks/game_server_staging/README.md) — the staging
  host that `mod bootstrap-staging` prepares and `setup server-profile` configures.
- [Host preparation](/documentation/runbooks/game_server_staging/host_preparation.md) — the
  one-time host steps around `mod bootstrap-staging`.
- [Client join and mod updates](/documentation/runbooks/game_server_staging/client_join_and_mod_updates.md)
  — where `setup client-addons` fits when a client joins.
- [Enfusion MCP tooling](/documentation/runbooks/enfusion_mcp_tooling.md) — where the pak folder
  of `mcp-game-root` is used.
- [Two-client playtest](/documentation/runbooks/two_client_playtest/README.md) — the client
  addon staging `client-addons` sets up.
