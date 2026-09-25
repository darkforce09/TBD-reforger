# Playtest server

The implementation of `cargo xtask mod playtest`: a local dedicated server that loads the
checkout's `tbd-framework` addon, registers a joinable backend room, and runs a
[mission](/documentation_v2/glossary.md#mission) that the platform deploys to it or that a
compiled document supplies offline.

## Contents

```text
tools_v2/xtask/src/commands/mod_ops/playtest_server/
├── boot/                   the launch, the wait for a verdict, the join banner and the Ctrl-C stop
├── boot.rs                 boot context and verdict types; declares the launch half in boot/
├── host.rs                 re-export of the shared host bridge, `crate::core::host_execution::Host`
├── lifecycle/              the group probe, kill_run, the run lock and `--selftest`
├── lifecycle.rs            run paths, probe states and lock types; re-exports the lifecycle functions
├── logread.rs              reads of server.out and console.log: boot phase, error dump, local-addon gate
├── platform_deployment.rs  provision, confirm and release the deployment, or stage the offline artifact
├── render.rs               profile seeding, backend-config patch, admin list and server.json render
├── tests/                  unit tests for boot, lifecycle, log reading and rendering
└── usage_fail.rs           flag parsing, preflight checks and the run order from staging to boot
```

## How it works

`tools_v2/xtask/src/commands/mod_ops/playtest_server.rs` holds the help text and `Opts`, and
declares every module here.

```text
usage_fail::run ─ parse flags (a token acts where it stands: `--help` exits 0 when reached)
  ├─ --selftest ────────────────────────────▶ lifecycle::selftest
  ├─ exactly one of --mission / --artifact-file; --port ≠ --a2s-port; admin ids valid ─ else 2
  ├─ host bridge, server binary, dev profile, addon GUID, scenarioId, LAN IP ─ else 3 (or 1)
  ├─ lifecycle::claim_lock, assert_no_live_server
  ├─ render::setup_server_profile   (`xtask setup server-profile <run dir>/profile`)
  ├─ platform_deployment::provision (--mission)  or  stage_offline_artifact (--artifact-file)
  ├─ render::patch_backend_config, symlink addons/tbd-framework, render::render_server_json
  ├─ --dry-run: print the engine command line and the advertised address; exit 0
  └─ boot::boot_and_wait ─▶ platform_deployment::confirm once up ─▶ release when it stops
```

- `render_server_json` starts from `tools_v2/xtask/dedicated_server_profiles/tbd-dev-server.config.json`
  and sets the bind and public address and ports, the A2S port, name, `scenarioId`, `maxPlayers`,
  `visible`, `admins` and one `TBD_Framework` mod entry whose id is the GUID read from
  `apps/mod/tbd-framework/addon.gproj`.
- With `--mission`, `platform_deployment` logs in through the
  [dev login](/documentation_v2/glossary.md#dev-login) as an administrator. It takes the mission's
  approved [artifact](/documentation_v2/glossary.md#artifact), submitting and approving the current
  version when there is none, and makes sure the artifact's terrain has a
  [fleet scenario](/documentation_v2/glossary.md#fleet-scenario). It then uses `--server` or the
  "TBD Playtest" server row, issues a `mod_runtime`
  [machine credential](/documentation_v2/glossary.md#machine-credential) for this run, and
  requests the [deployment](/documentation_v2/glossary.md#deployment). Once the server is up it
  waits up to 180 s for the runtime to confirm the deployment and cancels the unclaimed transition
  command; when the server stops it revokes the credential.
- `logread::assert_local_addon_won` is a hard gate. The engine's `Loaded addons:` block must name
  `<run dir>/addons/tbd-framework/addon.gproj` for the addon GUID. Otherwise the stale Workshop
  copy published under the same GUID won, and the run prints the cached copy to delete, kills the
  server and exits 1.
- Exit codes: 0 the server booted, the local addon won and the room registered; 1 the server
  died, refused its config, loaded the wrong addon copy, or could not be confirmed stopped; 2
  usage; 3 environment (no host bridge, no server binary, no dev profile, no LAN address, or a
  failed profile seed or platform provisioning).

## Public surface

- `run`: the `cargo xtask mod playtest` entry, called by
  `tools_v2/xtask/src/commands/mod_ops/dispatch.rs` and by
  `tools_v2/xtask/src/commands/mod_ops/development_server.rs` (`mod dev-server`).

## Boundaries

- Depends on: `crate::core::host_execution` (the host bridge), `crate::core::repository_layout`
  (`DEV_SERVER_PROFILE`), `crate::commands::mod_ops::website_api_client` (login, missions, fleet,
  deployments, artifact cache), `verification_core` (`Pattern`, `proc::Run`), the `serde_json`,
  `regex` and `libc` crates; the dedicated server under
  `$HOME/.local/share/Steam/steamapps/common/Arma Reforger Server`; the website
  [API](/documentation_v2/glossary.md#api) for `--mission`.
- Used by: `cargo xtask mod playtest` and `cargo xtask mod dev-server`; people running a local
  two-client playtest.
- Rules: the help lists exactly the flags the parser accepts
  (`help_text_matches_the_options_we_parse` in
  `tools_v2/xtask/src/commands/mod_ops/tests/playtest_server/tests.rs`); admin ids follow the
  engine's two patterns (`admin_schema_matches_the_engines_two_patterns`); the addon GUID is read
  from the gproj, never hard-coded (`guid_is_read_out_of_a_real_gproj_shape`); the local addon must
  win (`the_local_addon_must_win_or_the_gate_fails` in `tests/logread/tests.rs`); every exit
  returns through the lock guard so its drop always runs.

## Related documentation

- [Two-client playtest](/documentation_v2/runbooks/two_client_playtest/README.md) — running a
  playtest a second client can join.
- [Game server staging](/documentation_v2/runbooks/game_server_staging/README.md) — the staging
  server this lane mirrors locally.
