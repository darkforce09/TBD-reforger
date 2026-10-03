# Playtest server

The implementation of `cargo xtask mod playtest`: a local dedicated server that loads the
checkout's `tbd-framework` addon, registers a joinable backend room, and runs a
[mission](/documentation/glossary/g_to_m.md#mission) that the platform deploys to it or that a
compiled document supplies offline.

## Contents

```text
tools/commands/mod_operations/src/playtest_server/
├── boot/                   the launch, the wait for a verdict, the join banner and the Ctrl-C stop
├── boot.rs                 boot context and verdict types; declares the launch half in boot/
├── host.rs                 re-export of the shared host bridge, `process_runner::host_execution::Host`
├── lifecycle/              the group probe, kill_run, the run lock and `--selftest`
├── lifecycle.rs            run paths, probe states and lock types; re-exports the lifecycle functions
├── logread.rs              reads of server.out and console.log: boot phase, error dump, local-addon gate
├── platform_deployment.rs  provision, confirm and release the deployment, or stage the offline artifact
├── render.rs               profile seeding, backend-config patch, admin list and server.json render
├── telemetry_check.rs      the runtime's telemetry queue reading, the seen matches' events, the verdict
├── tests/                  unit tests for boot, lifecycle, log reading, rendering and the telemetry check
└── usage_fail.rs           flag parsing, preflight checks and the run order from staging to boot
```

## How it works

`tools/commands/mod_operations/src/playtest_server.rs` holds the help text and `Opts`, and
declares every module here.

```text
usage_fail::run ─ parse flags (a token acts where it stands: `--help` exits 0 when reached)
  ├─ --selftest ────────────────────────────▶ lifecycle::selftest
  ├─ exactly one of --mission / --artifact-file; --require-telemetry only with --mission;
  │  --port ≠ --a2s-port; admin ids valid; --timeout a duration ─ else 2
  ├─ host bridge, server binary, dev profile, addon GUID, scenarioId, LAN IP ─ else 3 (or 1)
  ├─ lifecycle::claim_lock, assert_no_live_server
  ├─ render::setup_server_profile   (`xtask setup server-profile <run dir>/profile`)
  ├─ platform_deployment::provision (--mission)  or  stage_offline_artifact (--artifact-file)
  ├─ render::patch_backend_config, symlink addons/tbd-framework, render::render_server_json
  ├─ --dry-run: print the engine command line and the advertised address; exit 0
  └─ boot::boot_and_wait ─▶ platform_deployment::confirm once up
                            ─▶ drain_telemetry on Ctrl-C or --timeout, server still up ─▶ release
     (telemetry_check: first reading, then the drain wait, then the verdict)
```

- `render_server_json` starts from `tools/xtask/dedicated_server_profiles/tbd-dev-server.config.json`
  and sets the bind and public address and ports, the A2S port, name, `scenarioId`, `maxPlayers`,
  `visible`, `admins` and one `TBD_Framework` mod entry whose id is the GUID read from
  `apps/mod/tbd-framework/addon.gproj`.
- With `--mission`, `platform_deployment` logs in through the
  [dev login](/documentation/glossary/a_to_f.md#dev-login) as an administrator. It takes the mission's
  approved [artifact](/documentation/glossary/a_to_f.md#artifact), submitting and approving the current
  version when there is none, and makes sure the artifact's terrain has a
  [fleet scenario](/documentation/glossary/a_to_f.md#fleet-scenario). It then uses `--server` or the
  "TBD Playtest" server row, issues a `mod_runtime`
  [machine credential](/documentation/glossary/g_to_m.md#machine-credential) for this run, and
  requests the [deployment](/documentation/glossary/a_to_f.md#deployment). Once the server is up it
  waits up to 180 s for the runtime to confirm the deployment and cancels the unclaimed transition
  command; when the server stops it revokes the credential. The profile's backend config holds
  `backendUrl` and this run's `machineCredential`, and no other secret.
- `telemetry_check` follows the deployment with the same administrator bearer. After the
  confirmation it reads `GET /api/v1/servers/{id}/status` every 5 s for up to 90 s until
  `status.telemetry_queue` appears, prints its backlog, capacity, drop total and oldest age, and
  remembers every non-empty `status.current_match_id`. On Ctrl-C, before the stop and while the
  server still heartbeats, `boot` runs the `before_stop` hook (also when `--timeout` expires): it
  reads the status again and, under `--require-telemetry`, polls up to 60 s until the backlog is
  zero. A server that exits on its own (a crash) gets no drain wait; its last stored reading
  stands. Once the
  server has stopped, the check reads `GET /api/v1/matches/{matchId}/events?limit=1` for each
  match seen. The check fails when no
  reading ever arrived, or when a match was seen and either a match holds no acknowledged event
  or the reading taken before the stop still holds a backlog (a server that crashed could not
  drain, so its backlog is not judged). Without `--require-telemetry` the verdict is only
  printed; with it a failed check turns exit 0 into 1. A run whose server never became ready is
  not checked.
- `logread::assert_local_addon_won` is a hard gate. The engine's `Loaded addons:` block must name
  `<run dir>/addons/tbd-framework/addon.gproj` for the addon GUID. Otherwise the stale Workshop
  copy published under the same GUID won, and the run prints the cached copy to delete, kills the
  server and exits 1.
- Exit codes: 0 the server booted, the local addon won and the room registered; 1 the server
  died, refused its config, loaded the wrong addon copy, could not be confirmed stopped, or failed
  the telemetry check under `--require-telemetry`; 2
  usage; 3 environment (no host bridge, no server binary, no dev profile, no LAN address, or a
  failed profile seed or platform provisioning).

## Public surface

- `run`: the `cargo xtask mod playtest` entry, called by
  `tools/commands/mod_operations/src/mod_dispatch.rs` and by
  `tools/commands/mod_operations/src/development_server.rs` (`mod dev-server`).

## Boundaries

- Depends on: `process_runner::host_execution` (the host bridge), `repository_layout`
  (`DEV_SERVER_PROFILE`), `crate::website_api_client` (login, missions, fleet,
  deployments, artifact cache, server status and match event reads), `verification_core` (`Pattern`), `process_runner::Run`, the `serde_json`,
  `regex` and `libc` crates; the dedicated server under
  `$HOME/.local/share/Steam/steamapps/common/Arma Reforger Server`; the website
  [API](/documentation/glossary/a_to_f.md#api) for `--mission`.
- Used by: `cargo xtask mod playtest` and `cargo xtask mod dev-server`; people running a local
  two-client playtest.
- Rules: the help lists exactly the flags the parser accepts
  (`help_text_matches_the_options_we_parse` in
  `tools/commands/mod_operations/src/tests/playtest_server/tests.rs`); admin ids follow the
  engine's two patterns (`admin_schema_matches_the_engines_two_patterns`); the addon GUID is read
  from the gproj, never hard-coded (`guid_is_read_out_of_a_real_gproj_shape`); the local addon must
  win (`the_local_addon_must_win_or_the_gate_fails` in `tests/logread/tests.rs`); a failed
  telemetry check fails only a successful run and only under `--require-telemetry`
  (`a_failed_check_fails_a_successful_run_only_when_required` in
  `tests/telemetry_check/tests.rs`); every exit returns through the lock guard so its drop always
  runs.

## Related documentation

- [Two-client playtest](/documentation/runbooks/two_client_playtest/README.md) — running a
  playtest a second client can join.
- [Playtest server runbook](/documentation/runbooks/two_client_playtest/playtest_server.md) —
  the dry run, the admin restart, the stop and the join checks, step by step.
- [Game server staging](/documentation/runbooks/game_server_staging/README.md) — the staging
  server this lane mirrors locally.
