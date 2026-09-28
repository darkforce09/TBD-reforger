# Staging deploy pipeline

The live half of `cargo xtask deploy staging`: the website API check, the rsync, the remote steps
over ssh, the server config push, the unit restart, the wait for a boot verdict and the final log
check. `tools_v2/xtask/src/commands/deploy/staging/remote.rs` declares the four files and holds the
transport: `SshBase` (plain ssh, sshpass or an identity file) and `Runner`, which prints each ssh
call under `--dry-run` instead of spawning it.

## Contents

```text
tools_v2/xtask/src/commands/deploy/staging/remote/
├── deployed_scenario.rs         the live server config's `game.scenarioId`, kept over `TBD_SCENARIO`
├── ssh_argv.rs                  `deploy`, the ssh and rsync argv, `ExecStart` per mode, the log verdict map
├── verify_boot_remote.rs        waits for room registration, pulls `console.log` and runs the boot verdict
└── website_api_health_check.rs  asks the host for the website API's `/healthz` and stops the deploy without it
```

## How it works

```text
deploy(paths, cli)
  ├─ Env::load + validate (deploy.env) ─ --render-only renders locally and stops here
  ├─ print ==> publicAddress <IPv4 address>
  ├─ ssh bash -s < website_api_health_payload   curl -sSf <TBD_BACKEND_URL>/healthz on the host
  ├─ rsync -avz --delete <checkout>/ <host>:<TBD_REMOTE_DIR>/   (exclusions below)
  ├─ ssh bash -s < profile_payload     setup server-profile, addon symlink, backendUrl
  ├─ ssh bash -s < smoke_payload       V2 to V4 game-runtime checks against the host's API
  ├─ config mode: render server.config.json locally (keeping the deployed scenario), push it
  ├─ ssh bash -s < unit_payload        write tbd-reforger.service with ExecStart, restart it
  ├─ verify_boot_remote                poll every 10 s up to TBD_BOOT_VERIFY_TIMEOUT, then verdict
  ├─ TBD_INSTALL_HOST_AGENT=1: ssh bash -s < install_payload (the fleet host agent)
  └─ DEPLOY_ENV=<this run's deploy.env> cargo run -q -p xtask -- mod remote-logs
       ─▶ v6_verdict: 0 and 2 pass, 1, 3 and others fail
```

Every step that exits non-zero stops the deploy with that code; a tool that is not installed
exits 127. `--dry-run` prints each step instead and opens no connection. The pipeline starts
nothing of the website and runs no compose command: the API, its Postgres and Caddy belong to
`cargo xtask deploy website`. `require_website_api` therefore runs first, before anything on the
host changes: it asks the host for `/healthz` under `TBD_BACKEND_URL` (trailing slashes dropped),
where the mod calls the API, and stops the deploy with exit 1 and a message naming
`cargo xtask deploy website` when the probe fails, a 503 from an API without its database
included; ssh's own failure keeps its code, 255.

`exec_start` gives config mode `-addonsDir <TBD_ADDONS_STAGING> -config <server config> -profile
<TBD_PROFILE_DIR>`, which loads the rsynced checkout and registers a backend room; addons mode
gives `-addonsDir`, `-addons <GUID>` and `-server <scenario>` with bind port and A2S port, which
loads the checkout and registers no room. The rsync excludes `.git/`, `target/`, the
untracked reference trees under `apps/mod/` (the Coalition framework, the vanilla scripts, the
playable selector), a `Tbd_framework` folder and the local test profile, the
`apps/mod/tbd-export/` and `apps/mod/tbd-emcp/` addons, `node_modules`, the API's `.env` and
`.tools/`, `deploy.env`, the `assets_v2` terrain, scratch and equipment trees, and
`apps/website/frontend/dist/`, the app the website deploy built in the same checkout; after them
come the patterns the website deploy excludes too, from
`tools_v2/xtask/src/commands/deploy/development_machine_only_paths.rs`: what only a development
machine holds, such as the cargo target folders beside `target/`, worktrees and the local files
of its agents and tools. With `--delete` and no `--delete-excluded`, each exclusion also keeps
rsync from deleting that path on the server.

`verify_boot_remote` finds the newest `logs_*` folder under the profile, waits for
`Server registered with address:`, pulls the log (a failed or empty pull fails the deploy), measures
the Workshop rival pak on the host, and runs `boot::verify_boot_log` in config mode; addons mode
runs only the addon check and says the room and admin checks were skipped.
`deployed_scenario` keeps a valid `game.scenarioId` of the live config, since the
[fleet host agent](/documentation_v2/glossary/a_to_f.md#fleet-host-agent) rewrites it to restart
missions, and `TBD_SCENARIO` seeds only a server without one.

## Boundaries

- Depends on: `super::config::Env`, `super::payloads`, `super::boot`, `super::render` and
  `super::host_agent`; `crate::commands::deploy::development_machine_only_paths` for the
  exclusions both deploys share; `verification_core::proc` for spawns; ssh, sshpass and rsync on
  the development machine, and bash, curl, systemd user units and cargo on the host, with the
  website API that `cargo xtask deploy website` runs there.
- Used by: `run` in `tools_v2/xtask/src/commands/deploy/staging.rs`.
- Rules: every spawn's argv is pure and pinned (`ssh_argv_plain_identity_and_sshpass`,
  `rsync_argv_keeps_every_exclude_in_order`,
  `rsync_argv_excludes_every_development_machine_only_path`,
  `exec_start_config_mode_carries_both_flags` in
  `tools_v2/xtask/src/commands/deploy/staging/tests/remote/tests.rs`); a dry run spawns nothing
  (`dry_run_never_spawns`, `the_website_api_check_never_spawns_on_a_dry_run`); the API check names
  the health route and the website deploy
  (`the_website_api_check_names_the_health_route_and_the_website_deploy`); the log verdict reads all
  four outcomes and never guesses (`v6_maps_all_four_outcomes_and_refuses_to_guess`); a missing
  tool never reads as success (`not_run_never_reads_as_success`); `ssh_argv.rs` holds no compose
  command (`cargo xtask verify staging-compose-paths`).
