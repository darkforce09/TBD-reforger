# Deployment templates

The files that set up the home server and the staging game server: the deploy settings template,
the Caddy site and the systemd user units. Nothing here runs by itself; the `cargo xtask deploy`
commands read these files, carry them to the host, or print how to install them.

## Contents

```text
tools/xtask/deploy/
├── Caddyfile.website   the Caddy site on :3080: the built app, with API paths proxied to :8080
├── deploy.env.example  the settings template, copied to the gitignored deploy.env beside it
└── systemd/            user units for the API, the game server fleet, its host agents and relay, and database backups
```

## How it works

Every path here is a constant in `tools/xtask/src/core/repository_layout.rs` (`DEPLOY_DIR`,
`DEPLOY_ENV`, `DEPLOY_ENV_EXAMPLE`, `CADDYFILE`, `SYSTEMD_UNITS_DIR`, `WEBSITE_API_UNIT`), which
the command that reads a file joins onto the checkout root. The operator's `deploy.env` holds the
host, the credentials and the remote paths; `TBD_SSH_HOST` there is the one place the deploy host
is named. The repository's root `.gitignore` names it, and both rsync lanes exclude it, so a
development machine never overwrites the server's copy.

Every command reads it through `tools/xtask/src/core/deploy_environment.rs`, the same way:

- `KEY=VALUE` lines, parsed and never executed: an optional `export `, values in `"…"` or `'…'`
  taken verbatim, a comment on its own line or after whitespace; a line that breaks the grammar
  stops the command with `<path>:<line>`.
- The file decides every key it assigns, and an empty assignment counts as unset; the process
  environment fills only the keys the file never assigns. A stale exported `TBD_SSH_HOST` therefore
  never redirects a deploy, and the example leaves its optional keys commented out. A command-line
  flag (`debug a2s-probe --host`) beats both.
- A `DEPLOY_ENV` variable points every command at another file; `deploy staging` hands the
  resolved path to the `mod remote-logs` it runs last.
- A value the command refuses is reported as `<path>:<line>: <KEY>: <problem>`, and a missing one
  as `<KEY> is not set: add it to <path>`. A missing file exits 1 and names the example to copy for
  the deploys; the other readers then take everything from the environment.

```text
deploy.env.example ──copy, fill in──▶ deploy.env ──read by──▶ deploy website | deploy staging |
                                                               mod bootstrap-staging | mod remote-logs |
                                                               debug direct-join | debug a2s-probe |
                                                               setup client-addons
Caddyfile.website  ──served by the caddy service of apps/website/docker-compose.staging.yml;
                     deploy website starts that service and reloads the file
systemd/           ──see that folder's README for what installs each unit
```

## Configuration

`deploy.env`, from `deploy.env.example`:

- Host access, read by every command that reaches the host: `TBD_SSH_HOST` (required), `user@host`
  or `host`, where the user is the deploy account whose home the remote folders default under; and
  `TBD_SSH_PASS` (for sshpass) or `TBD_SSH_IDENTITY_FILE` (for `ssh -i`), both optional.
- Remote folders, each defaulting under `/home/<user>` of `TBD_SSH_HOST` and required when it names
  no user: `TBD_REMOTE_DIR` (`tbd/repo`), `TBD_PROFILE_DIR` (`tbd/profile`), `TBD_ADDONS_STAGING`
  (`tbd/addons-staging`) and `TBD_SERVER_DIR` (`steam/arma-reforger-server`).
- Website, read by `tools/xtask/src/commands/deploy/website.rs`: `TBD_REMOTE_DIR`, which must
  sit under `/home/<user>/tbd/` (the `--delete` guard `require_tbd_remote_prefix` checks, so a
  `TBD_SSH_HOST` without a user is refused); `TBD_POSTGRES_HOST_PORT` (default 5432), the host port
  of the staging compose Postgres; `TBD_WEBSITE_SYSTEMD_UNIT` (default `tbd-website-api.service`);
  and, each skipping its step when set to 1, `TBD_SKIP_COMPOSE` (both compose steps, Postgres and
  Caddy), `TBD_SKIP_SPA_BUILD` (the app build only: Caddy still starts and serves the build
  already on the host) and `TBD_SKIP_API_BUILD`. `TBD_REMOTE_DIR`, `TBD_SSH_HOST` and
  `TBD_PROFILE_DIR` are refused when they contain `prairielearn` in any case.
- Game server fleet, read by `tools/xtask/src/commands/deploy/staging/config.rs` and
  `fleet_instances.rs` beside it: `TBD_FLEET_INSTANCES` (5, at most 5) instances, instance N on game
  port `TBD_FLEET_GAME_PORT_BASE` + N (2000), A2S port `TBD_FLEET_A2S_PORT_BASE` + N (17776) and
  loopback RCON port `TBD_FLEET_RCON_PORT_BASE` + N (19998), all distinct; the host agent of
  `TBD_FLEET_RELAY_INSTANCE` polls the relay on `127.0.0.1:TBD_FLEET_RELAY_PORT`, which is then
  required, and the other agents poll `TBD_HOST_AGENT_API_URL` (default `TBD_BACKEND_URL`, https or
  loopback http). Every instance starts with `-config` and needs a mod source:
  `TBD_WORKSHOP_MOD_ID`, or a modpack through `TBD_MODPACK_JSON` or `TBD_MODPACK_URL`. Settings with
  defaults: `TBD_BACKEND_URL` (`http://127.0.0.1:8080`), `TBD_ADDON_GUID`, `TBD_SCENARIO` (the
  [mission header](/documentation/glossary/g_to_m.md#mission-header) a new instance boots,
  `{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf`), `TBD_SERVER_DIR` (the install of Steam app
  1890870, the experimental server), `TBD_PUBLIC_ADDRESS` (an IPv4 address; unset, the first IPv4
  address `TBD_SSH_HOST` resolves to at deploy time, and the deploy stops when there is none),
  `TBD_ADMIN_PASSWORD`, `TBD_MAX_PLAYERS` (64), `TBD_ADMIN_IDENTITY_IDS`,
  `TBD_BOOT_VERIFY_TIMEOUT` (180 s), `TBD_MODPACK_TOKEN` and `TBD_WORKSHOP_MOD_NAME`
  (`TBD_Framework`). `TBD_PROFILE_DIR` names the single-instance profile that
  `--migrate-single-instance` archives.
- No secret is a setting. The deploy refuses, naming the replacement, a file that still assigns
  `TBD_MOD_RUNTIME_CREDENTIAL`, `TBD_HOST_AGENT_CREDENTIAL`, `TBD_RCON_PASSWORD`, `TBD_RCON_PORT`,
  `TBD_GAME_PORT`, `TBD_A2S_PORT`, `TBD_INSTALL_HOST_AGENT`, `TBD_SERVER_MODE`, `TBD_SERVER_NAME` or
  `TBD_SERVER_CONFIG_REMOTE`. Each instance's credentials are files on the host under
  `~/tbd/fleet/instance-N/secrets/`, its RCON password is generated there, and the join password is
  the host's `~/tbd/fleet/join-password`.
- Staging verification harness, read by `cargo xtask staging`: `TBD_STAGING_DB_CONTAINER`
  (`tbd_staging_db`), `TBD_STAGING_OPERATOR_DISCORD_ID`, `TBD_STAGING_PARTNER_GUILD_ID`,
  `TBD_STAGING_PARTNER_ROLE_ID`, `TBD_LOAD_TARGET_ORIGIN` and `TBD_LOAD_SOURCE_ADDRESSES`.

`Caddyfile.website` listens on `:3080` and sends the cross-origin isolation headers the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s WebAssembly needs
(`Cross-Origin-Opener-Policy: same-origin`, `Cross-Origin-Embedder-Policy: credentialless`). It
proxies `/api/*`, `/uploads/*`, `/map-assets/*` and `/healthz` to `127.0.0.1:8080`, and serves every
other path from the built app's `apps/website/frontend/dist` with an `index.html` fallback; the
offline service worker loader `/service_worker.js` carries `Cache-Control: no-cache`, so every
update check revalidates it. Its paths are the `caddy` container's: the compose service mounts this
folder read-only at `/etc/tbd-caddy` and `apps/website/frontend/` read-only at
`/srv/tbd-frontend`, so the site root `/srv/tbd-frontend/dist` is the built app wherever the
checkout sits. The service runs on the host's network, so Caddy listens on the host's `:3080` and
the API sees every proxied request come from `127.0.0.1`, the proxy its `TRUSTED_PROXIES` trusts.
Caddy trusts forwarded addresses only from the tunnel's loopback peer
(`trusted_proxies static 127.0.0.1/32` in the global `servers` block of `Caddyfile.website`): it
keeps the `X-Forwarded-For` that `cloudflared` sends and replaces any other peer's with that
peer's own address, so the API keys each visitor by their real address and each LAN client by its
own.

## Installed by

- `deploy.env.example`: the operator copies it to `deploy.env` beside it and fills it in. The
  file is read by `cargo xtask deploy website`, `cargo xtask deploy staging`,
  `cargo xtask mod bootstrap-staging`, `cargo xtask mod remote-logs`,
  `cargo xtask debug direct-join`, `cargo xtask debug a2s-probe` (without `--host`) and
  `cargo xtask setup client-addons` (for its Direct Join hint).
- `Caddyfile.website`: rsynced with the checkout and served by the `caddy` service of
  `apps/website/docker-compose.staging.yml`, which every `cargo xtask deploy website` starts and
  then reloads with `caddy reload --config /etc/tbd-caddy/Caddyfile.website`, so an edit applies
  with the next deploy. `apps/website/api_v2/tests/forwarded_for_trust.rs` pins its
  `reverse_proxy 127.0.0.1:8080` upstream, and
  `the_caddy_service_serves_what_the_caddyfile_and_the_reload_name` in
  `tools/xtask/src/commands/deploy/tests/website/tests.rs` pins its site root against the
  compose service's mounts.
- `systemd/`: each unit's install step is in its header and in that folder's README.

## Boundaries

- Depends on: ssh, rsync and, with `TBD_SSH_PASS`, sshpass on the development machine; systemd
  user units and a container runtime with compose on the host, which runs Caddy from the staging
  compose file.
- Used by: the `deploy`, `mod`, `setup` and `debug` commands above, through the layout constants
  and the loader in `tools/xtask/src/core/deploy_environment.rs`; the `caddy` service of
  `apps/website/docker-compose.staging.yml`, which mounts this folder; the API's forwarded-for
  test, which reads the Caddyfile.
- Rules: `deploy.env` is never committed and never rsynced (both exclude lists name
  `DEPLOY_ENV`, and `the_deploy_secrets_file_sits_beside_its_example` in
  `tools/xtask/src/tests/repository_layout_tests.rs` pins its place beside the example); a
  path added here gets its constant in `tools/xtask/src/core/repository_layout.rs`, which
  `every_committed_location_exists_in_the_checkout` checks; the example loads under the grammar
  and assigns no optional key empty (`the_committed_example_loads_and_masks_nothing`); documents
  name the host only as `TBD_SSH_HOST`, and no production file under `tools` names a
  private-network address (`no_production_tooling_file_names_a_private_network_address`).

## Related documentation

- [Website deployment](/documentation/runbooks/website_deployment.md) — deploying the API and
  the app to the home server.
- [Game server staging](/documentation/runbooks/game_server_staging/README.md) — the staging
  game server and its host agent.
- [Database operations](/documentation/runbooks/database_operations.md) — installing the backup
  and restore-drill timers of `systemd/`.
