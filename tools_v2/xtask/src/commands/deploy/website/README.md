# Website deploy steps

The pure parts of `cargo xtask deploy website`: the text of every command it sends to the server,
the rsync argument list and the help block. `tools_v2/xtask/src/commands/deploy/website.rs`
declares these modules, reads `deploy.env` and runs the steps in order; keeping the text here lets
the dry run print exactly what a live run executes and lets the tests pin it.

## Contents

```text
tools_v2/xtask/src/commands/deploy/website/
├── asset_preflight.rs  the remote map-asset probe, its three verdicts and the move a refusal prints
├── help_text.rs        the `--help` block, built around the `DEPLOY_ENV` path the command reads
├── remote_steps.rs     each remote step's shell: compose, API and app builds, checksum repair, restart
├── rsync_argv.rs       the rsync argv, whose exclude list is also the `--delete` guard
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
  trees, `packages/`, the untracked reference trees under `apps/mod/` and the local test profile.
  With no `--delete-excluded`, every exclusion is also a path rsync never deletes on the server;
  `assets_v2/glyphs/` is tracked and travels with the rsync.
- `remote_steps`: the staging Postgres from `apps/website/docker-compose.staging.yml` with
  `TBD_POSTGRES_HOST_PORT`, through docker compose or podman compose; `cargo build --release -p
  website-api --bin api`; `trunk build --release` in `apps/website/frontend`;
  `TBD_DB_CONTAINER=tbd_staging_db cargo xtask db repair-migration-checksum --force`; the move of
  any `uploads` folder left in `apps/website/api_v2/` into the unit's state folder
  `tbd-website-api`; and the `systemctl --user restart` of the unit followed by `is-active`.
- `systemd_unit`: the default unit is the file name of `WEBSITE_API_UNIT`,
  `tbd-website-api.service`; `install_command` renders its template with
  `TBD_REPO_DIR_PLACEHOLDER` set to `TBD_REMOTE_DIR` without the leading slash, writes it to
  `~/.config/systemd/user/` and enables it. The deploy prints that command only when the restart
  fails.

## Boundaries

- Depends on: `crate::core::repository_layout` (`DEPLOY_ENV`, `WEBSITE_API_UNIT`,
  `SYSTEMD_UNITS_DIR`).
- Used by: `tools_v2/xtask/src/commands/deploy/website.rs`.
- Rules: the exclude list keeps the secrets, the asset and scratch trees
  (`rsync_excludes_the_secrets_asset_and_scratch_trees` in
  `tools_v2/xtask/src/commands/deploy/tests/website/tests.rs`) and ends with source and destination
  (`rsync_argv_keeps_source_and_destination_last`); the plan ends with the checksum repair and the
  state move (`the_remote_plan_ends_with_the_checksum_repair_and_the_state_move`); the state
  folder is the one the API unit declares
  (`the_unit_template_declares_the_state_directory_the_deploy_moves_into`); the install command
  renders the shipped template (`the_unit_install_command_renders_the_shipped_template_for_the_remote_dir`).
