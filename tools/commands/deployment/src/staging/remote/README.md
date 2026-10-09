# Staging deploy pipeline

The live half of `cargo xtask deploy staging`: the website API check, the host's secret files, the
rsync, the remote steps of every fleet instance over ssh, the unit install and restart, the boot
verdict per instance, the relay and host agents, and the final log check per instance.
`tools/commands/deployment/src/staging/remote.rs` declares the files here and holds
`Runner`, which prints each ssh call under a dry run instead of spawning it; the transport it
drives, `SshBase` (plain ssh, `sshpass -e` or an identity file) and the ssh argv, lives in
`tools/foundation/process_runner/src/secure_shell_transport.rs`, which the staging harness shares.

## Contents

```text
tools/commands/deployment/src/staging/remote/
├── deployed_scenario.rs         each instance's live `game.scenarioId`, read on the host, kept over `TBD_SCENARIO`
├── fleet_deploy.rs              `deploy`, the pipeline for every instance, and the `--dry-run` plan
├── instance_boot_verdict.rs     the boot progress probe, the wait for each new boot, the verdict and log check per instance
├── ssh_argv.rs                  the rsync argv, the not-run exit code, the log verdict map
└── website_api_health_check.rs  asks the host for the website API's `/healthz` and stops the deploy without it
```

## How it works

```text
deploy(paths, cli)
  ├─ Env::load + validate (deploy.env) ─ --render-only renders locally and stops here
  ├─ --migrate-host-agent-name ─▶ ssh bash -s < host agent name migration_payload and stop
       (under --dry-run: print the plan line and the exact script)
  ├─ --dry-run ─▶ print dry_run_plan (every step, every instance) and stop
  ├─ ssh bash -s < website_api_health_payload          curl -sSf <TBD_BACKEND_URL>/healthz on the host
  ├─ ssh bash -s < fleet_secret_files_check_payload    join password + two credentials per instance
  ├─ without --migrate-single-instance: refuse while tbd-reforger.service or fleet-host-agent.service is installed
  ├─ ssh bash -s < retired_names_absent_payload        refuse while a fleet_host_agent name is left
  ├─ ssh bash -s < the API .env probe                   refuse unless <TBD_REMOTE_DIR>/crates/api/api_server/.env is readable
  ├─ rsync -avz --delete <checkout>/ <host>:<TBD_REMOTE_DIR>/   (exclusions below)
  ├─ with --migrate-single-instance: ssh bash -s < migration_payload (kebab-case single-instance names)
  ├─ per instance: scenario read, local render, ssh bash -s < instance_files_payload, < smoke_payload
  ├─ ssh bash -s < units_install_payload; probe the newest logs; restart every tbd-reforger@N
  ├─ instance_boot_verdicts: poll every 10 s up to TBD_BOOT_VERIFY_TIMEOUT, then a verdict per instance
  ├─ relay_install_payload (the relay instance), host_agents_install_payload (every instance)
  └─ per instance: pull console.log, cargo run -q -p xtask -- mod remote-logs --file <copy>
       ─▶ v6_verdict: 0 and 2 pass, 1, 3 and others fail
```

Every step that exits non-zero stops the deploy with that code; a tool that is not installed
exits 127. Every instance gets its boot verdict before a failed one stops the deploy with exit 1,
and the log check runs for every instance before its failure does. The pipeline starts nothing of
the website and runs no compose command: the API, its Postgres and Caddy belong to
`cargo xtask deploy website`. `require_website_api` therefore runs first: it asks the host for
`/healthz` under `TBD_BACKEND_URL` (trailing slashes dropped), where the mods call the API, and
stops the deploy with exit 1 and a message naming `cargo xtask deploy website` when the probe
fails; ssh's own failure keeps its code, 255. The ssh password, with `TBD_SSH_PASS`, reaches
`sshpass -e` through the spawned process's `SSHPASS` variable, never an argument.

The rsync excludes `.git/`, `target/`, the
untracked reference trees under `mod/` (the Coalition framework, the vanilla scripts, the
playable selector), a `Tbd_framework` folder and the local test profile, the
`mod/tbd-export/` and `mod/tbd-emcp/` addons, `node_modules`, the API's `.env` and
`.tools/`, `deploy.env`, the `assets` terrain, scratch and equipment trees, and
`crates/frontend/shell/frontend_application/dist/`, the app the website deploy built in the same
checkout (the three host-owned paths of `tools/commands/deployment/src/host_owned_paths.rs`, which
the website deploy excludes too); after them
come the patterns the website deploy excludes too, from
`tools/commands/deployment/src/development_machine_only_paths.rs`: what only a development
machine holds, such as the cargo target folders beside `target/`, worktrees and the local files
of its agents and tools. With `--delete` and no `--delete-excluded`, each exclusion also keeps
rsync from deleting that path on the server.

`instance_boot_verdicts` probes every instance's newest `logs_*` folder and its room registration
in one round trip, before the restart and every 10 s after it. A folder that was the newest before
the restart never counts, since the previous boot's log also says `Server registered with
address:`; each instance passes only on a new folder whose `console.log` the pull returned whole,
judged by `boot::verify_boot_log` with the Workshop rival pak measured on the host.
`deployed_scenario` extracts a valid `game.scenarioId` from the live config on the host, since the
[game server host agent](/documentation/glossary/g_to_m.md#game-server-host-agent) rewrites it to restart
missions and the config also holds the instance's passwords; `TBD_SCENARIO` seeds only an
instance without one.

## Boundaries

- Depends on: `super::config::Env`, `super::fleet_instances`, `super::fleet_server_config`,
  `super::payloads`, `super::fleet_units`, `super::host_agent`, `super::acknowledgement_relay`,
  `super::legacy_single_instance_migration` and `super::boot`;
  `crate::development_machine_only_paths` for the exclusions both deploys share; `process_runner` for spawns; ssh, sshpass and rsync on
  the development machine, and bash, curl, systemd user units and cargo on the host, with the
  website API that `cargo xtask deploy website` runs there.
- Used by: `run` in `tools/commands/deployment/src/staging.rs`.
- Rules: every spawn's argv is pure and pinned (`ssh_argv_plain_identity_and_sshpass`,
  `rsync_argv_keeps_every_exclude_in_order`,
  `rsync_argv_excludes_every_development_machine_only_path` in
  `tools/commands/deployment/src/staging/tests/remote/tests.rs`); no argv holds the ssh
  password (`the_ssh_password_is_in_no_argv_and_only_in_the_child_environment`); a dry run spawns
  nothing and its plan names every instance and no secret (`dry_run_never_spawns`,
  `the_website_api_check_never_spawns_on_a_dry_run`,
  `the_dry_run_plan_walks_every_instance_and_prints_no_secret`); a stale log never passes a new
  boot; the API check names
  the health route and the website deploy
  (`the_website_api_check_names_the_health_route_and_the_website_deploy`); the log verdict reads all
  four outcomes and never guesses (`v6_maps_all_four_outcomes_and_refuses_to_guess`); a missing
  tool never reads as success (`not_run_never_reads_as_success`); no production file of the
  staging deploy holds a compose command (`cargo xtask verify staging-compose-paths` audits
  `tools/commands/deployment/src/staging.rs` and every file under
  `tools/commands/deployment/src/staging/` outside its `tests/` folders, and exits 1 with a
  "did not run" cause when `fleet_deploy.rs`, the pipeline, is missing).
