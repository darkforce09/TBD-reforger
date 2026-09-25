# Deployment templates

The files that set up the home server and the staging game server: the deploy settings template,
the Caddy site and the systemd user units. Nothing here runs by itself; the `cargo xtask deploy`
commands read these files, carry them to the host, or print how to install them.

## Contents

```text
tools_v2/xtask/deploy/
├── Caddyfile.website   the Caddy site on :3080: the built app, with API paths proxied to :8080
├── deploy.env.example  the settings template, copied to the gitignored deploy.env beside it
└── systemd/            user units for the API, game server, fleet host agent and database backups
```

## How it works

Every path here is a constant in `tools_v2/xtask/src/core/repository_layout.rs` (`DEPLOY_DIR`,
`DEPLOY_ENV`, `DEPLOY_ENV_EXAMPLE`, `CADDYFILE`, `SYSTEMD_UNITS_DIR`, `WEBSITE_API_UNIT`), which
the command that reads a file joins onto the checkout root. The operator's `deploy.env` holds the
host, the credentials and the remote paths. The repository's root `.gitignore` names it, and both rsync lanes
exclude it, so a development machine never overwrites the server's copy. The commands parse it
as `KEY=VALUE` lines, with an optional `export ` prefix, and never execute it; a missing file
exits 1 and names the example to copy. The deploy host is whatever `TBD_SSH_HOST` names there.

```text
deploy.env.example ──copy, fill in──▶ deploy.env ──read by──▶ deploy website | deploy staging |
                                                               mod bootstrap-staging | mod remote-logs |
                                                               debug direct-join
Caddyfile.website  ──loaded by Caddy on the host; deploy website prints the reload line
systemd/           ──see that folder's README for what installs each unit
```

## Configuration

`deploy.env`, from `deploy.env.example`:

- Host access, read by every command that reaches the host: `TBD_SSH_HOST` (required;
  `deploy website` exits 79 without it), and `TBD_SSH_PASS` (for sshpass) or
  `TBD_SSH_IDENTITY_FILE` (for `ssh -i`), both optional.
- Website, read by `tools_v2/xtask/src/commands/deploy/website.rs`: `TBD_REMOTE_DIR` (required,
  exit 80 without it), which must sit under the fixed deploy prefix `require_tbd_remote_prefix`
  checks; `TBD_POSTGRES_HOST_PORT` (default 5432), the host port of the staging compose Postgres;
  `TBD_WEBSITE_SYSTEMD_UNIT` (default `tbd-website-api.service`); and `TBD_SKIP_COMPOSE`,
  `TBD_SKIP_SPA_BUILD` and `TBD_SKIP_API_BUILD`, which skip a step when set to 1. `TBD_REMOTE_DIR`,
  `TBD_SSH_HOST` and `TBD_PROFILE_DIR` are refused when they contain `prairielearn` in any case. A
  `DEPLOY_ENV` environment variable points this command, and only this one, at another file.
- Game server, read by `tools_v2/xtask/src/commands/deploy/staging/config.rs`, where a value in
  the file wins over the process environment: `TBD_REMOTE_DIR`, `TBD_PROFILE_DIR`,
  `TBD_ADDONS_STAGING`, `TBD_GAME_SERVER_TOKEN` (the API's `SERVICE_TOKEN`) and
  `TBD_MOD_RUNTIME_CREDENTIAL` (a `mod_runtime` machine credential) are required.
  `TBD_SERVER_MODE` defaults to `config`, which also needs a mod source: `TBD_WORKSHOP_MOD_ID`, or
  a modpack through `TBD_MODPACK_JSON` or `TBD_MODPACK_URL`; the `addons` mode needs none and
  registers no joinable room. Settings with defaults: `TBD_BACKEND_URL` (`http://127.0.0.1:8080`),
  `TBD_ADDON_GUID`, `TBD_SCENARIO` (the [mission header](/documentation_v2/glossary.md#mission-header)
  the server boots, `{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf`), `TBD_SERVER_DIR`,
  `TBD_GAME_PORT` (2001), `TBD_A2S_PORT` (17777, which must differ from the game port),
  `TBD_BIND_IP` and `TBD_PUBLIC_ADDRESS`, `TBD_SERVER_NAME`, `TBD_ADMIN_PASSWORD`,
  `TBD_MAX_PLAYERS` (64), `TBD_ADMIN_IDENTITY_IDS`, `TBD_SERVER_CONFIG_REMOTE`,
  `TBD_BOOT_VERIFY_TIMEOUT` (180 s), `TBD_MODPACK_TOKEN` and `TBD_WORKSHOP_MOD_NAME`
  (`TBD_Framework`).
- Fleet host agent, read only when `TBD_INSTALL_HOST_AGENT=1`: `TBD_HOST_AGENT_CREDENTIAL` and
  `TBD_RCON_PASSWORD` are required; `TBD_RCON_PORT` defaults to 19999 and
  `TBD_HOST_AGENT_API_URL` to `TBD_BACKEND_URL`.

`Caddyfile.website` listens on `:3080` and sends the cross-origin isolation headers the
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s WebAssembly needs
(`Cross-Origin-Opener-Policy: same-origin`, `Cross-Origin-Embedder-Policy: credentialless`). It
proxies `/api/*`, `/uploads/*`, `/map-assets/*` and `/healthz` to `127.0.0.1:8080`, and serves every
other path from the built app's `apps/website/frontend/dist` with an `index.html` fallback. Its
site root is a fixed absolute path, edited when the checkout sits elsewhere on the host.

## Installed by

- `deploy.env.example`: the operator copies it to `deploy.env` beside it and fills it in. The
  file is read by `cargo xtask deploy website`, `cargo xtask deploy staging`,
  `cargo xtask mod bootstrap-staging`, `cargo xtask mod remote-logs` and
  `cargo xtask debug direct-join`.
- `Caddyfile.website`: loaded by Caddy on the host by hand; `cargo xtask deploy website` ends by
  printing the `caddy reload --config` line for it. `apps/website/api_v2/tests/forwarded_for_trust.rs`
  pins its `reverse_proxy 127.0.0.1:8080` upstream.
- `systemd/`: each unit's install step is in its header and in that folder's README.

## Boundaries

- Depends on: ssh, rsync and, with `TBD_SSH_PASS`, sshpass on the development machine; Caddy,
  systemd user units and a container runtime on the host.
- Used by: the `deploy`, `mod` and `debug` commands above, through the layout constants
  (`tools_v2/xtask/src/commands/deploy/`, `tools_v2/xtask/src/commands/setup/staging_server.rs`,
  `tools_v2/xtask/src/commands/debug/`); the API's forwarded-for test, which reads the Caddyfile.
- Rules: `deploy.env` is never committed and never rsynced (both exclude lists name
  `DEPLOY_ENV`, and `the_deploy_secrets_file_sits_beside_its_example` in
  `tools_v2/xtask/src/tests/repository_layout_tests.rs` pins its place beside the example); a
  path added here gets its constant in `tools_v2/xtask/src/core/repository_layout.rs`, which
  `every_committed_location_exists_in_the_checkout` checks; documents name the host only as
  `TBD_SSH_HOST`.

## Related documentation

- [Website deployment](/documentation_v2/runbooks/website_deployment.md) — deploying the API and
  the app to the home server.
- [Game server staging](/documentation_v2/runbooks/game_server_staging/README.md) — the staging
  game server and its host agent.
