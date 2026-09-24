**Status:** live

# README template: deploy or config

**When to use:** a folder of templates, service units and profiles that set up a host or a server:
`tools_v2/xtask/deploy/`, its `systemd/` folder, `tools_v2/xtask/dedicated_server_profiles/`. The
[README standard](/documentation_v2/standards/readme_standard.md) defines every rule this template
follows; the deploy or config kind adds Configuration and Installed by.

## Skeleton

Copy the block and replace every `<…>` placeholder; each one says what goes there. Configuration
and Installed by cover the folder's own files in full. For a child folder they state only what
crosses the folder's boundary or holds for all of the child's files, such as the step that
installs them or a setting they share, one line each; the detail stays in the child's README.

````markdown
# <What the files set up, in plain words: no path, no backticks>

<One to three sentences: which hosts or servers the files set up, and what applies them.>

## Contents

```text
<repository path of the folder>/
├── <child folder>/  <what it holds: a lowercase phrase, no closing period>
└── <file>           <what it sets up>
```

## How it works

<How the files reach a host: the constants that name their paths, the command that reads each,
the placeholders a template carries and how they are substituted, and what must never be
committed.>

## Configuration

<Every setting the files hold or the commands read from them: the key, its default, whether it is
required, and the code that reads it.>

## Installed by

- <file>: <the xtask command or the manual step that puts it to work, and where it ends up on the
  host>

## Boundaries

- Depends on: <the commands, services and host tools the files assume>
- Used by: <the commands, services and tests that read the files, found with git grep>
- Rules: <the invariants a change must keep: secrets stay out of the repository, placeholders stay
  in templates, hosts are named by variable>

## Related documentation

- [<runbook title>](/documentation_v2/runbooks/<runbook>.md) — <what it covers>
````

## Worked sample

Written from `tools_v2/xtask/deploy/`. The sample covers the two files beside the `systemd/` folder
in full and gives `systemd/` one line under Installed by, because that folder's own README says
what installs each unit. The sample sits in a fenced block, so no gate reads it as a README; the
folder's own README.md is written from the same code and may differ.

````markdown
# Deployment templates

The files that set up the home server and the staging game server: the deploy settings template,
the Caddy site, and the systemd user units. Nothing here runs by itself; the `cargo xtask deploy`
commands read these files or print how to install them.

## Contents

```text
tools_v2/xtask/deploy/
├── Caddyfile.website   the Caddy site on :3080: the built app, with API paths proxied to :8080
├── deploy.env.example  the settings template, copied to the gitignored deploy.env beside it
└── systemd/            user units for the API, game server, fleet host agent and database backups
```

## How it works

Every path here is a constant in `tools_v2/xtask/src/core/repository_layout.rs` (`DEPLOY_DIR`,
`DEPLOY_ENV`, `CADDYFILE`, `SYSTEMD_UNITS_DIR`), which the command that reads a file joins onto the
checkout root. The operator's `deploy.env` holds the host, the credentials and the remote paths; git
ignores it, and both rsync lanes exclude it, so a development machine never overwrites the server's
copy. The commands parse it as `KEY=VALUE` lines and never execute it. The deploy host is whatever
`TBD_SSH_HOST` names there.

## Configuration

`deploy.env`, from `deploy.env.example`:

- Host access: `TBD_SSH_HOST`, required by every command that reaches the host; `TBD_SSH_PASS`
  (for sshpass) or `TBD_SSH_IDENTITY_FILE` (for `ssh -i`), both optional.
- Website, read by `tools_v2/xtask/src/commands/deploy/website.rs`: `TBD_REMOTE_DIR`, required and
  held under the deploy prefix that `cargo xtask deploy website --help` prints;
  `TBD_POSTGRES_HOST_PORT` (default 5432), the host port of the staging compose Postgres;
  `TBD_WEBSITE_SYSTEMD_UNIT` (default `tbd-website-api.service`); and `TBD_SKIP_COMPOSE`,
  `TBD_SKIP_SPA_BUILD` and `TBD_SKIP_API_BUILD`, which skip a step when set to 1. A `DEPLOY_ENV`
  environment variable points the command at another settings file.
- Game server, read by `tools_v2/xtask/src/commands/deploy/staging/config.rs`: `TBD_REMOTE_DIR`,
  `TBD_PROFILE_DIR`, `TBD_ADDONS_STAGING`, `TBD_GAME_SERVER_TOKEN` and
  `TBD_MOD_RUNTIME_CREDENTIAL` are required. `TBD_SERVER_MODE` defaults to `config`, which also
  needs a mod source: `TBD_WORKSHOP_MOD_ID`, or a modpack through `TBD_MODPACK_JSON` or
  `TBD_MODPACK_URL`; the `addons` mode needs none. Among the settings that default are
  `TBD_BACKEND_URL` (`http://127.0.0.1:8080`) and `TBD_SCENARIO` (the mission header the server
  boots, `{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf`).
- Fleet host agent, read only when `TBD_INSTALL_HOST_AGENT=1`: `TBD_HOST_AGENT_CREDENTIAL` and
  `TBD_RCON_PASSWORD` are required; `TBD_RCON_PORT` defaults to 19999 and
  `TBD_HOST_AGENT_API_URL` to `TBD_BACKEND_URL`.

`Caddyfile.website` listens on `:3080`, sends the cross-origin isolation headers the Mission
Creator's WebAssembly needs (`Cross-Origin-Opener-Policy: same-origin`,
`Cross-Origin-Embedder-Policy: credentialless`), proxies `/api/*`, `/uploads/*`, `/map-assets/*`
and `/healthz` to `127.0.0.1:8080`, and serves every other path from the built app's `dist` folder
with an `index.html` fallback. Its site root is a fixed path, edited when the checkout sits
elsewhere.

## Installed by

- `deploy.env.example`: the operator copies it to `deploy.env` beside it and fills it in. The file
  is read by `cargo xtask deploy website`, `cargo xtask deploy staging`,
  `cargo xtask mod bootstrap-staging`, `cargo xtask mod remote-logs` and
  `cargo xtask debug direct-join`.
- `Caddyfile.website`: loaded by Caddy on the host by hand; `cargo xtask deploy website` ends by
  printing the `caddy reload --config` line for it. `apps/website/api_v2/tests/forwarded_for_trust.rs`
  pins its `reverse_proxy 127.0.0.1:8080` upstream.
- `systemd/`: each unit's install command is in its header and in that folder's README.

## Boundaries

- Depends on: ssh and rsync on the development machine, and Caddy, systemd user units and a
  container runtime on the host.
- Used by: the `deploy`, `mod` and `debug` commands above, through the layout constants; the API's
  forwarded-for test, which reads the Caddyfile.
- Rules: `deploy.env` is never committed and never rsynced; a path added here gets its constant in
  `tools_v2/xtask/src/core/repository_layout.rs`, the one place the commands take it from.

## Related documentation

- [Website deployment](/documentation_v2/runbooks/website_deployment.md) — deploying the API and
  the app to the home server.
- [Game server staging](/documentation_v2/runbooks/game_server_staging/README.md) — the staging
  game server and its host agent.
````
