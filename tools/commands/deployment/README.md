# Deployment

The `deployment` crate: the `cargo xtask deploy` group. It deploys the website to the home server
and the staging fleet of dedicated game servers, and holds the paths no deploy ships. The
`deploy db` verbs it carries are the `database_operations` crate's. The operator runs the deploys
from a development machine.

## Contents

```text
tools/commands/deployment/
├── Cargo.toml  the `deployment` library package: `database_operations`, `deploy_settings`, `process_runner`, layout tier 5
└── src/        the website and staging drivers, the shared excludes and the errors
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

Both rsyncs exclude the paths of `host_owned_paths.rs`, which the host keeps for itself in its
checkout: the API's settings file `deploy/api.env`, the `.tools/` folder in
`crates/api/api_server/`, and the app the website deploy builds in
`crates/frontend/shell/frontend_application/dist/`. Before either rsync runs,
`api_environment_file_preflight.rs` moves a settings file the host still keeps at
`crates/api/api_server/.env` to `<TBD_REMOTE_DIR>/deploy/api.env` (never over an existing one),
asks the host over ssh whether that file exists and is readable, and refuses the deploy, naming
the operator's step, when it is missing, unreadable or the probe gets no answer: the rsync runs
with `--delete`, so a settings file left at a path no exclusion names would be deleted.

Both rsyncs append the patterns of `development_machine_only_paths.rs` to their own exclusions:
anchored at the checkout root, they name what only a development machine holds (the retired and
hand-set cargo target folders, retired app builds, the vanilla compile baseline, the linked
worktrees under `.worktrees/`, the machine-local `.workstation/`, and the local files of Claude Code
and the MCP configuration). None of them is tracked, and the host needs none of them.

```text
deploy website: asset probe ─▶ settings file probe ─▶ rsync --delete ─▶ compose postgres ─▶ API build ─▶ host tools build
                ─▶ app build ─▶ compose caddy, then its reload ─▶ checksum repair
                ─▶ restart the unit ─▶ hints
deploy staging: settings check ─▶ website API check ─▶ secret files ─▶ single-instance check
                ─▶ retired host agent name check ─▶ settings file probe ─▶ rsync --delete
                ─▶ per instance: files and runtime smoke ─▶ units, restart
                ─▶ boot verdict per instance ─▶ relay ─▶ host agents ─▶ log check per instance
deploy staging --migrate-host-agent-name: settings check ─▶ the name migration over ssh, alone
deploy db:      database_operations::container_database::run
```

## Commands

Each runs as `cargo xtask deploy <command>`; a clap usage error exits 2.

### website

- Synopsis: `cargo xtask deploy website [--dry-run] [--help]`
- Does: reads `TBD_SSH_HOST` (required, with the deploy user) and `TBD_REMOTE_DIR` (default
  `/home/<user>/tbd/repo`), refuses a host, remote folder or `TBD_PROFILE_DIR` that contains
  `prairielearn` in any case, a host without a user, and a remote folder outside
  `/home/<user>/tbd/` or holding `..`, then probes the server's map assets and its API settings file,
  rsyncs the checkout, and over ssh brings up the staging Postgres (`TBD_POSTGRES_HOST_PORT`,
  default 5432), builds the release `api-server` of `api_server`, the staging host tools `staging-fixtures` and `acknowledgement-dropping-relay` and
  the app, starts the staging Caddy and reloads its Caddyfile, repoints comments-only migration
  checksums and restarts `TBD_WEBSITE_SYSTEMD_UNIT` (default `tbd-website-api.service`). Set to 1,
  `TBD_SKIP_COMPOSE` skips both compose steps, `TBD_SKIP_API_BUILD` both cargo builds and
  `TBD_SKIP_SPA_BUILD` the app build. A failing step stops the deploy before the restart; a failed
  restart only warns and prints the unit's install command. `--dry-run` prints the plan, with
  every rsync exclusion, and connects to nothing; it still needs a filled `deploy.env`.
- Exit codes: 0 deployed, or the plan printed; 1 no `deploy.env`, a malformed one, a missing
  required value, a refused path or host, a refused asset layout, or a host without its API settings file
  (or one the probe could not read); 2 an unknown option; a
  failing rsync or ssh step's own code; 127 ssh, sshpass or rsync not installed.
- Example: `cargo xtask deploy website --dry-run`

### staging

- Synopsis: `cargo xtask deploy staging [--dry-run] [--migrate-single-instance] [--render-only
  <directory>] [--verify-boot <console.log>] [--verify-boot-selftest]`, or
  `cargo xtask deploy staging --migrate-host-agent-name [--dry-run]`
- Does: checks that the website API answers `/healthz` at `TBD_BACKEND_URL` on the staging host,
  then deploys the checkout there and runs the fleet of `TBD_FLEET_INSTANCES` (at most 5)
  dedicated servers: instance N runs as `tbd-reforger@N.service` with its own `-config` and
  `-profile` under `~/tbd/fleet/instance-N/`, beside its
  [game server host agent](/documentation/glossary/g_to_m.md#game-server-host-agent)
  `game_server_host_agent@N.service`; the relay instance's agent polls the API through
  `acknowledgement-dropping-relay@N.service`. Each instance's boot is judged from its own log.
  The deploy refuses missing secret files, the retired single-server settings, the installed
  single-server units unless `--migrate-single-instance` retires them first, a host that still
  carries a retired `fleet_host_agent` name, and a host without its API settings file.
  `--migrate-host-agent-name` runs alone (with `--dry-run` at most, which prints the exact
  script): on the host it stops and disables every `fleet_host_agent@N.service`, moves
  `~/.local/bin/fleet_host_agent` and `~/.config/fleet_host_agent/` to their
  `game_server_host_agent` names, removes the retired unit template, installs the current one and
  enables and starts `game_server_host_agent@N.service` for the same N; it refuses with exit 3,
  changing nothing, while both names of the folder or of the binary exist, and a second run
  changes nothing. `--render-only`
  writes every instance's server config into a local directory; `--verify-boot` judges a log you
  already have; `--verify-boot-selftest` proves the verdict can fail. The last two need no
  `deploy.env`.
- Exit codes: 0 deployed, migrated, rendered or judged healthy; 1 a missing, retired or refused
  setting, a missing or malformed secret file, the single-server units or a retired host agent
  name still installed, a host without its API settings file, a website API that does not answer, a
  failed migration step, or a failed boot verdict or log check; 2 an unknown option, a flag
  without its value, or `--migrate-host-agent-name` beside another mode flag; 3 a migration
  refused because both names exist; a failing step's own code; 127 a tool that is not installed.
- Example: `cargo xtask deploy staging --verify-boot-selftest`

### db

- Synopsis: `cargo xtask deploy db <command>`
- Does: the verified backup, the dump verifier, the guarded restore, the restore drill and their
  helpers; see the [database operations crate](/tools/commands/database_operations/README.md).
- Example: `cargo xtask deploy db is-safe-scratch --db rust_it`

## Boundaries

- Depends on: `database_operations` (`deploy db`), `deploy_settings`, `process_runner`,
  `repository_layout` (the shared locations; the addon folders and folder names from its
  `enfusion_mod_folders`), `verification_core`, `newtype_ids`, `clap`, `regex`, `serde_json`,
  `thiserror`; ssh, sshpass and rsync on the development machine; on the host, cargo, trunk,
  docker or podman compose, systemd user units and the dedicated server.
- Used by: the `deploy`, `verify` and `ci` groups of `xtask`, and the `staging_procedures` and
  `remote_debugging` crates for the fleet layout (`staging::fleet_instances`, `staging::payloads`) and the remote toolchain
  line; the operator, for both deploys.
- Rules: tier 5 of `tools/commands` (`cargo xtask verify crate-tiers`); `deploy.env` is never
  executed and never rsynced; both rsyncs exclude every development-machine-only path, no such
  path matches a tracked file, and each pattern is anchored at the checkout root
  (`no_tracked_file_matches_a_development_machine_only_path`); both rsyncs exclude every
  host-owned path (`both_rsyncs_exclude_every_host_owned_path`) and run only after the settings file
  probe answers 0 (`a_missing_file_refuses_before_the_rsync`); the website deploy refuses a remote
  folder outside the deploy user's `tbd` folder (`remote_prefix_rejects_escape_and_outside`); an
  unknown staging option stops before `--help`.

## Related documentation

- [Website deployment](/documentation/runbooks/website_deployment.md) — the home server, its
  API unit, Caddy and the website deploy.
- [Game server staging](/documentation/runbooks/game_server_staging/README.md) — the staging
  game server and its host agent.
- [Deployment templates](/deploy/README.md) — `deploy.env`, the Caddy site and the systemd units
  these commands read or print.
