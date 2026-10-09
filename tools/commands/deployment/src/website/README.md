# Website deploy steps

The pure parts of `cargo xtask deploy website`: the text of every command it sends to the server,
the rsync argument list and the help block. `tools/commands/deployment/src/website.rs`
declares these modules, reads `deploy.env` and runs the steps in order; keeping the text here lets
the dry run print exactly what a live run executes and lets the tests pin it.

## Contents

```text
tools/commands/deployment/src/website/
├── asset_preflight.rs  the remote map-asset probe, its three verdicts and the move a refusal prints
├── help_text.rs        the `--help` block: the settings file, its precedence rule and every key
├── remote_steps.rs     each remote step's shell, and the login-shell word ssh sends it as: compose, builds, restart
├── rsync_argv.rs       the rsync argv, whose exclude list is also the `--delete` guard, and its dry-run lines
└── systemd_unit.rs     the default API unit name, its template path and its one-time install command
```

## How it works

- `asset_preflight`: before the rsync, `probe_script` asks the server whether
  `assets/terrains/terrain-registry.json` exists in `TBD_REMOTE_DIR` (exit 0), whether only a
  `packages/map-assets` tree does (exit 10), or neither (exit 11). The deploy continues on the
  first, continues with a warning on the third (a host that serves only the mission library), and
  stops on the second or on any other answer, printing the `mv` commands for the second.
- The API `.env` probe (`tools/commands/deployment/src/api_environment_file_preflight.rs`) runs
  next, still before the rsync: the deploy sends its `probe_script` as the same `bash -lc` word as
  every step, continues only on exit 0 (a readable `crates/api/api_server/.env` in
  `TBD_REMOTE_DIR`) and stops with exit 1 on exit 20 (missing or unreadable) or any other answer,
  naming the operator's step; `--dry-run` prints the probe before the rsync lines.
- `rsync_argv`: `rsync -e <ssh> -avz --delete` of the checkout root with exclusions for `.git/`,
  build output (`target/`, which holds the gates' private folders too, `node_modules`), the
  server's `deploy.env`, the served terrain tree `assets/terrains/`, the scratch and equipment asset
  trees, `packages/`, the untracked reference trees under `mod/` and the local test profile,
  followed by the paths both deploys exclude: the host-owned paths of
  `tools/commands/deployment/src/host_owned_paths.rs` (the API's `.env` and `.tools/`, and
  `crates/frontend/shell/frontend_application/dist/`), then the patterns from
  `tools/commands/deployment/src/development_machine_only_paths.rs`: what only a
  development machine holds, such as the retired and hand-set cargo target folders beside
  `target/`, worktrees and
  the local files of its agents and tools. With no `--delete-excluded`, every exclusion is also a
  path rsync never deletes on the server; `assets/glyphs/` is tracked and travels with the
  rsync. `dry_run_lines` renders the transfer and one `[dry-run]   --exclude=` line per exclusion,
  in argv order, which is what `--dry-run` prints.
- `remote_steps`: every compose step `cd`s into `TBD_REMOTE_DIR`, exports
  `TBD_POSTGRES_HOST_PORT` and defines `staging_compose`, a shell function that runs
  `deploy/compose.staging.yml` under docker compose when the host has docker, else
  under podman compose. The steps are: `staging_compose up -d postgres`; `cargo build --release -p
  api_server --bin api-server` (`API_SERVER`), proven by `test -x target/release/api-server`; the
  staging host tools of `STAGING_HOST_TOOLS`, `cargo build --release
  -p staging_fixtures --bin staging-fixtures` and `cargo build --release -p developer_tools --bin
  acknowledgement-dropping-relay`, each proven by `test -x target/release/<executable>`, so the
  staging harness and `cargo xtask deploy staging` find them built from the same checkout;
  `trunk build --release` in `crates/frontend/shell/frontend_application`;
  `staging_compose up -d caddy`, then `staging_compose exec -T caddy caddy reload --config
  /etc/tbd-caddy/Caddyfile --adapter caddyfile`, tried up to `CADDY_RELOAD_ATTEMPTS` (5)
  times a second apart, because `up -d` returns before Caddy's admin endpoint listens;
  `TBD_DB_CONTAINER=tbd_staging_db cargo xtask db repair-migration-checksum --force`; and the
  `systemctl --user restart` of the unit followed by `is-active`.
  `website.rs` orders them: Postgres, the API build, the host tools build, the app build, Caddy and
  the checksum repair. `TBD_SKIP_COMPOSE=1` drops both compose steps;
  `TBD_SKIP_API_BUILD=1` drops both cargo steps; `TBD_SKIP_SPA_BUILD=1` drops only the app
  build, so Caddy still starts, reloads and serves the build already on the host. The Caddyfile's
  global options trust a forwarded client address from the tunnel's loopback peer
  `127.0.0.1/32` alone. Every command,
  the map-asset probe and the restart included, reaches ssh through `login_shell`, as the one word
  `bash -lc '<command>'`: ssh joins its remote arguments into a single line for the host's shell,
  so an unquoted command would lose all but its first word to that shell, which runs in the home
  folder without the login profile.
- `systemd_unit`: the default unit is the file name of `WEBSITE_API_UNIT`,
  `tbd-website-api.service`; `install_command` renders its template with
  `TBD_REPO_DIR_PLACEHOLDER` set to `TBD_REMOTE_DIR` without the leading slash, writes it to
  `~/.config/systemd/user/` and enables it. The deploy prints that command only when the restart
  fails.

## Boundaries

- Depends on: `repository_layout` (`DEPLOY_ENV`, `WEBSITE_API_UNIT`,
  `SYSTEMD_UNITS_DIR`); `crate::host_owned_paths` and `crate::development_machine_only_paths`
  for the exclusions both deploys share; `crate::api_environment_file_preflight` for the `.env`
  probe; the `[[bin]]` names of `crates/api/api_server/Cargo.toml` and
  `tools/developer_tools/Cargo.toml` for the API server and the host tools; `tools/commands/deployment/src/website.rs` reads the
  settings through `deploy_settings`; on the host, the `postgres` and `caddy`
  services of `deploy/compose.staging.yml` and the Caddyfile the `caddy` service
  mounts.
- Used by: `tools/commands/deployment/src/website.rs`.
- Rules: the exclude list keeps the secrets, the asset and scratch trees
  (`rsync_excludes_the_secrets_asset_and_scratch_trees` in
  `tools/commands/deployment/src/tests/website/tests.rs`) and every development-machine-only
  path (`rsync_excludes_every_development_machine_only_path`), the dry run prints every exclusion, and the argv
  ends with source and destination
  (`rsync_argv_keeps_source_and_destination_last`); the plan ends with the checksum repair, and the Caddy
  step follows compose, not the app build;
  the host tools build after the API and skip with it, each is built and proven and is a `[[bin]]` of the
  package it names, the API server and the relay under the names their units run; the API build names `api_server` and
  `api-server` and the app build runs in the app's folder; the `.env` probe reaches
  the host as one login-shell word and the unit loads
  the file it proves; the Caddyfile trusts only
  the tunnel's peer;
  every compose step names the staging compose file
  (`every_compose_step_runs_the_staging_compose_file_from_the_checkout`); a step reaches
  the host's login shell whole (`a_remote_step_reaches_the_login_shell_as_one_word`); the Caddy
  reload names the Caddyfile where the compose service mounts it, and the Caddyfile's site root
  is where it mounts the app;
  the service mounts the Caddyfile's own folder and no folder that holds `deploy.env`
  (`the_caddy_service_mounts_no_folder_holding_the_deploy_secrets`);
  the API unit keeps its runtime files in its own state folder
  (`the_unit_template_keeps_the_runtime_files_in_its_state_directory`); the
  `crates/api/api_server/.env.example` the host's `.env` starts from sets none of the variables the
  API unit pins, since a value in the `.env` overrides the unit's; the install command
  renders the shipped template.
