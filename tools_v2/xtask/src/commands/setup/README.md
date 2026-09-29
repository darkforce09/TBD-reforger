# Setup command group

The `cargo xtask setup` group: one-time preparation of a developer machine or a server for the
[mod](/documentation_v2/glossary/g_to_m.md#mod): the dedicated-server profile, the base-game link
[Workbench](/documentation_v2/glossary/n_to_z.md#workbench) asks for under Proton, the pak folder the
[Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) MCP tools read, and the client addon staging
link. The folder also holds the staging host bootstrap that `cargo xtask mod bootstrap-staging`
runs.

## Contents

```text
tools_v2/xtask/src/commands/setup/
├── cli.rs              the `SetupCmd` clap enum: four subcommands and their arguments
├── client_addons.rs    `setup client-addons`: the client addon staging link and Steam launch options
├── dispatch.rs         routes each `SetupCmd` to its module
├── mcp_game_root.rs    `setup mcp-game-root`: a flat folder of links to every game pak
├── mod.rs              the module tree
├── server_profile.rs   `setup server-profile`: the dedicated-server profile and its backend config
├── staging_server.rs   `mod bootstrap-staging`: discovery and directory creation on the staging host
├── tests/              unit tests for every command on throwaway homes and trees
└── workbench_linux.rs  `setup workbench`: links the Steam base game to a short home path for Proton
```

## How it works

`tools_v2/xtask/src/cli/mod.rs` mounts `SetupCmd` as the `setup` group and `dispatch::run` calls
each module's `run`. Every command writes only under the profile directory, `$HOME` or the paths
its arguments name, prints what it made, and returns its exit code. Each module has a
`run_with_root` or `run_with_paths` entry that takes its roots as arguments, so the tests run
against throwaway trees instead of the operator's Steam install or home.

`staging_server.rs` is not a `setup` subcommand: `tools_v2/xtask/src/commands/mod_ops/dispatch.rs`
sends `cargo xtask mod bootstrap-staging` to it. It reads `TBD_SSH_HOST`, `TBD_REMOTE_DIR`,
`TBD_PROFILE_DIR`, `TBD_ADDONS_STAGING`, `TBD_SSH_PASS` and `TBD_SSH_IDENTITY_FILE` from
`tools_v2/xtask/deploy/deploy.env` through `crate::core::deploy_environment` (the file decides every
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
  `apps/mod/.local-test-profile`.
- Does: creates `<PROFILE>/profile/` (mode 700) and copies
  `apps/mod/tbd-framework/Data/backend.example.json` to `profile/TBD_BackendConfig.json` (mode
  600); the config holds two keys, `backendUrl` and `machineCredential`. It writes
  `machineCredential` from `TBD_MACHINE_CREDENTIAL` when that is set. It then copies
  `apps/mod/tbd-framework/Data/registry.json` to `profile/TBD_Registry.json`, best effort: the mod
  reads that copy only when its own `Data/registry.json` is missing. A profile without a
  [machine credential](/documentation_v2/glossary/g_to_m.md#machine-credential) boots no
  [mission](/documentation_v2/glossary/g_to_m.md#mission), since the mission comes from the server's
  [mission deployment](/documentation_v2/glossary/g_to_m.md#mission-deployment). The Workbench checklist
  it prints last does not match that boot path: it names a
  [mission header](/documentation_v2/glossary/g_to_m.md#mission-header) and two components to set up by
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
- Does: links `apps/mod/tbd-framework/` into `$HOME/.local/share/tbd-server-addons/` (the link is
  made even when the target is missing) and prints the Steam launch options that load it and a
  Direct Join hint: `Direct Join → <host> (<IPv4 address>) port 2001` for the host of
  `TBD_SSH_HOST` in `deploy.env`, the host with the reason when it has no IPv4 address from here, or
  `the staging host, port 2001 (set TBD_SSH_HOST in <path> to print its address)`.
- Exit codes: 0 linked; the exit code of `mkdir` or `ln` when either fails.
- Example: `cargo xtask setup client-addons`

## Boundaries

- Depends on: `crate::core::repository_root`, `crate::core::repository_layout` (the staging
  runbook) and `crate::core::deploy_environment` (the deploy host and the remote folders); `verification_core::proc` for `ssh` and `sshpass`; `mkdir`, `ln` and
  `whoami`; `serde_json` for the backend config.
- Used by:
  - `tools_v2/xtask/src/cli/dispatch.rs`, which mounts the group, and
    `tools_v2/xtask/src/commands/mod_ops/dispatch.rs`, for `mod bootstrap-staging`;
  - `cargo xtask deploy staging`, whose remote payload runs `setup server-profile` on the staging
    host (`tools_v2/xtask/src/commands/deploy/staging/payloads.rs`), and `cargo xtask staging fleet
    --record`, whose `mod_runtime` credential promotion runs the same payload commands;
  - the mod, whose registry loader names `cargo xtask setup server-profile` when no registry file
    exists (`apps/mod/tbd-framework/Scripts/Game/TBD/Core/TBD_Registry.c`);
  - people setting up a machine, following the runbooks below.
- Rules:
  - The backend config is written mode 600 inside a mode 700 folder, and the credential is written
    in place, keeping the other keys and their order (`clean_tree_writes_modes_and_no_mission` and
    `machine_credential_is_written_into_the_backend_config_in_place` in
    `tests/server_profile/tests.rs`).
  - A missing input exits 1 rather than succeeding (`missing_backend_exits_1`,
    `missing_gproj_exits_1`, `missing_addons_dir_exits_1`, `arm_missing_host_exits_1`).
  - The Direct Join hint never fails the command and names the host, its address or the setting
    to fill (`the_join_hint_names_the_host_its_address_or_the_setting_to_fill`).
  - Tests use the `run_with_*` entries on throwaway roots and never touch the real Steam tree or
    home.

## Related documentation

- [Game server staging](/documentation_v2/runbooks/game_server_staging/README.md) — the staging
  host that `mod bootstrap-staging` prepares and `setup server-profile` configures.
- [Host preparation](/documentation_v2/runbooks/game_server_staging/host_preparation.md) — the
  one-time host steps around `mod bootstrap-staging`.
- [Client join and mod updates](/documentation_v2/runbooks/game_server_staging/client_join_and_mod_updates.md)
  — where `setup client-addons` fits when a client joins.
- [Enfusion MCP tooling](/documentation_v2/runbooks/enfusion_mcp_tooling.md) — where the pak folder
  of `mcp-game-root` is used.
- [Two-client playtest](/documentation_v2/runbooks/two_client_playtest/README.md) — the client
  addon staging `client-addons` sets up.
