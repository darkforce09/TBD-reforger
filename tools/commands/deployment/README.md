# Deployment

The `deployment` crate: the `cargo xtask deploy` group. It deploys the website to the home server
and the staging fleet of dedicated game servers, holds the paths no deploy ships, and runs the
`cargo xtask verify staging-compose-paths` gate. The `deploy db` verbs it carries are the
`database_operations` crate's. The operator runs the deploys from a development machine.

## Contents

```text
tools/commands/deployment/
├── Cargo.toml  the `deployment` library package: `database_operations`, `deploy_settings`, `process_runner`, layout tier 5
└── src/        the website and staging drivers, the shared excludes, the compose-path check and the errors
```

## How it works

`tools/xtask/src/cli/dispatch.rs` passes the parsed `DeployCmd` to `deployment::run`. The
`website` and `staging` commands take their arguments raw and parse them themselves; `db` parses
with clap and goes to `database_operations`.

Both deploys read `deploy/deploy.env` (or the file `DEPLOY_ENV` names), which is gitignored and
excluded from both rsyncs, through `deploy_settings`: parsed as `KEY=VALUE` lines and never
executed, the file deciding every key it assigns and the process environment filling only the
others; a missing file exits 1 and names `deploy.env.example`. The host is whatever
`TBD_SSH_HOST` names, and the remote folders default under its user's `/home/<user>`; ssh runs
through sshpass when `TBD_SSH_PASS` is set, else with `-i` when `TBD_SSH_IDENTITY_FILE` is,
always with `StrictHostKeyChecking=no`.

Both rsyncs append the patterns of `development_machine_only_paths.rs` to their own exclusions:
anchored at the checkout root, they name what only a development machine holds (the retired and
hand-set cargo target folders, retired app builds, the vanilla compile baseline, slice and ticket
worktrees, the wave gate's receipts, and the local files of Claude Code, Codex and the MCP
configuration). None of them is tracked, and the host needs none of them.

```text
deploy website: asset probe ─▶ rsync --delete ─▶ compose postgres ─▶ API build ─▶ host tools build
                ─▶ app build ─▶ compose caddy, then its reload ─▶ checksum repair
                ─▶ restart the unit ─▶ hints
deploy staging: settings check ─▶ website API check ─▶ secret files ─▶ single-instance check
                ─▶ rsync --delete ─▶ per instance: files and runtime smoke ─▶ units, restart
                ─▶ boot verdict per instance ─▶ relay ─▶ host agents ─▶ log check per instance
deploy db:      database_operations::container_database::run
```

## Commands

Each runs as `cargo xtask deploy <command>`; a clap usage error exits 2.

### website

- Synopsis: `cargo xtask deploy website [--dry-run] [--help]`
- Does: reads `TBD_SSH_HOST` (required, with the deploy user) and `TBD_REMOTE_DIR` (default
  `/home/<user>/tbd/repo`), refuses a host, remote folder or `TBD_PROFILE_DIR` that contains
  `prairielearn` in any case, a host without a user, and a remote folder outside
  `/home/<user>/tbd/` or holding `..`, then probes the server's map assets, rsyncs the checkout,
  and over ssh brings up the staging Postgres (`TBD_POSTGRES_HOST_PORT`, default 5432), builds the
  release API, the staging host tools `staging-fixtures` and `acknowledgement-dropping-relay` and
  the app, starts the staging Caddy and reloads its Caddyfile, repoints comments-only migration
  checksums and restarts `TBD_WEBSITE_SYSTEMD_UNIT` (default `tbd-website-api.service`). Set to 1,
  `TBD_SKIP_COMPOSE` skips both compose steps, `TBD_SKIP_API_BUILD` both cargo builds and
  `TBD_SKIP_SPA_BUILD` the app build. A failing step stops the deploy before the restart; a failed
  restart only warns and prints the unit's install command. `--dry-run` prints the plan, with
  every rsync exclusion, and connects to nothing; it still needs a filled `deploy.env`.
- Exit codes: 0 deployed, or the plan printed; 1 no `deploy.env`, a malformed one, a missing
  required value, a refused path or host, or a refused asset layout; 2 an unknown option; a
  failing rsync or ssh step's own code; 127 ssh, sshpass or rsync not installed.
- Example: `cargo xtask deploy website --dry-run`

### staging

- Synopsis: `cargo xtask deploy staging [--dry-run] [--migrate-single-instance] [--render-only
  <directory>] [--verify-boot <console.log>] [--verify-boot-selftest]`
- Does: checks that the website API answers `/healthz` at `TBD_BACKEND_URL` on the staging host,
  then deploys the checkout there and runs the fleet of `TBD_FLEET_INSTANCES` (at most 5)
  dedicated servers: instance N runs as `tbd-reforger@N.service` with its own `-config` and
  `-profile` under `~/tbd/fleet/instance-N/`, beside its
  [fleet host agent](/documentation/glossary/a_to_f.md#fleet-host-agent)
  `fleet_host_agent@N.service`; the relay instance's agent polls the API through
  `acknowledgement-dropping-relay@N.service`. Each instance's boot is judged from its own log.
  The deploy refuses missing secret files, the retired single-server settings, and the installed
  single-server units unless `--migrate-single-instance` retires them first. `--render-only`
  writes every instance's server config into a local directory; `--verify-boot` judges a log you
  already have; `--verify-boot-selftest` proves the verdict can fail. The last two need no
  `deploy.env`.
- Exit codes: 0 deployed, rendered or judged healthy; 1 a missing, retired or refused setting, a
  missing or malformed secret file, the single-server units still installed, a website API that
  does not answer, or a failed boot verdict or log check; 2 an unknown option or a flag without
  its value; a failing step's own code; 127 a tool that is not installed.
- Example: `cargo xtask deploy staging --verify-boot-selftest`

### db

- Synopsis: `cargo xtask deploy db <command>`
- Does: the verified backup, the dump verifier, the guarded restore, the restore drill and their
  helpers; see the [database operations crate](/tools/commands/database_operations/README.md).
- Example: `cargo xtask deploy db is-safe-scratch --db rust_it`

### verify staging-compose-paths

- Synopsis: `cargo xtask verify staging-compose-paths`
- Does: holds that `deploy/compose.staging.yml` has one owner: every compose command of
  `deploy website` names it, and `deploy staging` runs none.
- Exit codes: 0 held; 1 findings; 2 a source could not be read.
- Example: `cargo xtask verify staging-compose-paths`

## Boundaries

- Depends on: `database_operations` (`deploy db`), `deploy_settings`, `process_runner`,
  `repository_layout`, `verification_core`, `newtype_ids`, `clap`, `regex`, `serde_json`,
  `thiserror`; ssh, sshpass and rsync on the development machine; on the host, cargo, trunk,
  docker or podman compose, systemd user units and the dedicated server.
- Used by: the `deploy`, `verify` and `ci` groups of `xtask`, and the `staging_procedures` and
  `remote_debugging` crates for the fleet layout (`staging::fleet_instances`, `staging::payloads`) and the remote toolchain
  line; the operator, for both deploys.
- Rules: tier 5 of `tools/commands` (`cargo xtask verify crate-tiers`); `deploy.env` is never
  executed and never rsynced; both rsyncs exclude every development-machine-only path, no such
  path matches a tracked file, and each pattern is anchored at the checkout root
  (`no_tracked_file_matches_a_development_machine_only_path`); the website deploy refuses a remote
  folder outside the deploy user's `tbd` folder (`remote_prefix_rejects_escape_and_outside`); an
  unknown staging option stops before `--help` (`unknown_option_short_circuits_before_help`).

## Related documentation

- [Website deployment](/documentation/runbooks/website_deployment.md) — the home server, its
  API unit, Caddy and the website deploy.
- [Game server staging](/documentation/runbooks/game_server_staging/README.md) — the staging
  game server and its host agent.
- [Deployment templates](/deploy/README.md) — `deploy.env`, the Caddy site and the systemd units
  these commands read or print.
