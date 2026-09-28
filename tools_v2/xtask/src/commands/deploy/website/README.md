# Website deploy steps

The pure parts of `cargo xtask deploy website`: the text of every command it sends to the server,
the rsync argument list and the help block. `tools_v2/xtask/src/commands/deploy/website.rs`
declares these modules, reads `deploy.env` and runs the steps in order; keeping the text here lets
the dry run print exactly what a live run executes and lets the tests pin it.

## Contents

```text
tools_v2/xtask/src/commands/deploy/website/
├── asset_preflight.rs  the remote map-asset probe, its three verdicts and the move a refusal prints
├── help_text.rs        the `--help` block: the settings file, its precedence rule and every key
├── remote_steps.rs     each remote step's shell, and the login-shell word ssh sends it as: compose, builds, restart
├── rsync_argv.rs       the rsync argv, whose exclude list is also the `--delete` guard, and its dry-run lines
└── systemd_unit.rs     the default API unit name, its template path and its one-time install command
```

## How it works

- `asset_preflight`: before the rsync, `probe_script` asks the server whether
  `assets_v2/terrains/terrain-registry.json` exists in `TBD_REMOTE_DIR` (exit 0), whether only a
  `packages/map-assets` tree does (exit 10), or neither (exit 11). The deploy continues on the
  first, continues with a warning on the third (a host that serves only the mission library), and
  stops on the second or on any other answer, printing the `mv` commands for the second.
- `rsync_argv`: `rsync -e <ssh> -avz --delete` of the checkout root with exclusions for `.git/`,
  build output (`target/`, `target-gate-*/`, `dist-gate-*/`, `node_modules`,
  `apps/website/frontend/dist/`), the server's secrets (the API's `.env` and `.tools/`,
  `deploy.env`), the served terrain tree `assets_v2/terrains/`, the scratch and equipment asset
  trees, `packages/`, the untracked reference trees under `apps/mod/` and the local test profile,
  followed by the patterns `deploy staging` excludes too, from
  `tools_v2/xtask/src/commands/deploy/development_machine_only_paths.rs`: what only a
  development machine holds, such as the cargo target folders beside `target/`, worktrees and
  the local files of its agents and tools. With no `--delete-excluded`, every exclusion is also a
  path rsync never deletes on the server; `assets_v2/glyphs/` is tracked and travels with the
  rsync. `dry_run_lines` renders the transfer and one `[dry-run]   --exclude=` line per exclusion,
  in argv order, which is what `--dry-run` prints.
- `remote_steps`: every compose step `cd`s into `TBD_REMOTE_DIR`, exports
  `TBD_POSTGRES_HOST_PORT` and defines `staging_compose`, a shell function that runs
  `apps/website/docker-compose.staging.yml` under docker compose when the host has docker, else
  under podman compose. The steps are: `staging_compose up -d postgres`; `cargo build --release -p
  website-api --bin api`; `trunk build --release` in `apps/website/frontend`;
  `staging_compose up -d caddy`, then `staging_compose exec -T caddy caddy reload --config
  /etc/tbd-caddy/Caddyfile.website --adapter caddyfile`, tried up to `CADDY_RELOAD_ATTEMPTS` (5)
  times a second apart, because `up -d` returns before Caddy's admin endpoint listens;
  `TBD_DB_CONTAINER=tbd_staging_db cargo xtask db repair-migration-checksum --force`; the move of
  any `uploads` folder left in `apps/website/api_v2/` into the unit's state folder
  `tbd-website-api`; and the `systemctl --user restart` of the unit followed by `is-active`.
  `website.rs` orders them: Postgres, the two builds, Caddy, the checksum repair and the state
  move. `TBD_SKIP_COMPOSE=1` drops both compose steps; `TBD_SKIP_SPA_BUILD=1` drops only the app
  build, so Caddy still starts, reloads and serves the build already on the host. Every command,
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

- Depends on: `crate::core::repository_layout` (`DEPLOY_ENV`, `WEBSITE_API_UNIT`,
  `SYSTEMD_UNITS_DIR`); `crate::commands::deploy::development_machine_only_paths` for the
  exclusions both deploys share; `tools_v2/xtask/src/commands/deploy/website.rs` reads the
  settings through `crate::core::deploy_environment`; on the host, the `postgres` and `caddy`
  services of `apps/website/docker-compose.staging.yml` and the Caddyfile the `caddy` service
  mounts.
- Used by: `tools_v2/xtask/src/commands/deploy/website.rs`; `cargo xtask verify
  staging-compose-paths`, which reads `remote_steps.rs` as text.
- Rules: the exclude list keeps the secrets, the asset and scratch trees
  (`rsync_excludes_the_secrets_asset_and_scratch_trees` in
  `tools_v2/xtask/src/commands/deploy/tests/website/tests.rs`) and every development-machine-only
  path (`rsync_excludes_every_development_machine_only_path`), the dry run prints every exclusion
  (`the_dry_run_prints_every_exclusion_the_development_machine_only_paths_included`), and the argv
  ends with source and destination
  (`rsync_argv_keeps_source_and_destination_last`); the plan ends with the checksum repair and the
  state move (`the_remote_plan_ends_with_the_checksum_repair_and_the_state_move`), and the Caddy
  step follows compose, not the app build (`the_web_server_step_follows_compose_and_not_the_app_build`);
  every compose step names the staging compose file
  (`every_compose_step_runs_the_staging_compose_file_from_the_checkout`, and
  `cargo xtask verify staging-compose-paths` over this folder's `remote_steps.rs`); a step reaches
  the host's login shell whole (`a_remote_step_reaches_the_login_shell_as_one_word`); the Caddy
  reload names the Caddyfile where the compose service mounts it, and the Caddyfile's site root
  is where it mounts the app (`the_caddy_service_serves_what_the_caddyfile_and_the_reload_name`);
  the state folder is the one the API unit declares
  (`the_unit_template_declares_the_state_directory_the_deploy_moves_into`); the
  `apps/website/api_v2/.env.example` the host's `.env` starts from sets none of the variables the
  API unit pins, since a value in the `.env` overrides the unit's
  (`the_env_template_sets_none_of_the_variables_the_unit_pins`); the install command
  renders the shipped template (`the_unit_install_command_renders_the_shipped_template_for_the_remote_dir`).
