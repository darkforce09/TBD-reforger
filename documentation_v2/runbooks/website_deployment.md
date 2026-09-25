**Status:** live

# Website deployment

Deploys the website [API](/documentation_v2/glossary.md#api) and the single-page app to the home
server: `cargo xtask deploy website` copies the checkout to the host, starts the staging Postgres,
builds the release API and the app there and restarts the API's systemd user unit; Caddy serves
the app on port 3080 and a Cloudflare Tunnel can publish it. The first
[deployment](/documentation_v2/glossary.md#deployment) runs Phases A to E once, about an hour with
the server-side builds; every later one is the single command under
[Redeploy](#redeploy). The dedicated game server on the same host has its own runbook,
[Game server staging](/documentation_v2/runbooks/game_server_staging/README.md).

```text
browser ──▶ Cloudflare Tunnel (optional, Phase E) ──▶ Caddy :3080  (tools_v2/xtask/deploy/Caddyfile.website)
                                                      ├── /api/*, /uploads/*, /map-assets/*, /healthz
                                                      │     ──▶ API 0.0.0.0:8080  (tbd-website-api.service, Phase D)
                                                      └── every other path ──▶ apps/website/frontend/dist
API ──▶ Postgres tbd_staging_db on 127.0.0.1:${TBD_POSTGRES_HOST_PORT:-5432}  (apps/website/docker-compose.staging.yml)
API uploads ──▶ ~/.local/state/tbd-website-api/uploads  (outside the checkout)
```

The host is whatever `TBD_SSH_HOST` names in `tools_v2/xtask/deploy/deploy.env`; this runbook
writes it, and the checkout folder `TBD_REMOTE_DIR`, as placeholders.

## Prerequisites

On the development machine:

- ssh and rsync, plus sshpass when the host takes a password (`TBD_SSH_PASS`). Check:
  `command -v ssh rsync`.
- The Rust toolchain, for `cargo xtask`.
- ssh access to the host as the deploy user; a key (`TBD_SSH_IDENTITY_FILE`) is preferred over a
  password. The host keeps a fixed LAN address (a DHCP reservation on the router), so
  `TBD_SSH_HOST` stays valid across reboots.

On the host:

- docker or podman with a compose provider; the deploy tries `docker compose` first, then
  `podman compose`. Check: `docker compose version` or `podman compose version`.
- rustup under `~/.cargo/bin`, with Trunk installed there too: the remote build steps put only
  `$HOME/.cargo/bin` on `PATH`. The root `rust-toolchain.toml` pins 1.95.0 with the
  `wasm32-unknown-unknown` target, and rustup installs it on the first build. Check:
  `~/.cargo/bin/trunk --version`.
- Caddy, and cloudflared only for the tunnel in Phase E.
- Ports 3080, 8080 and the Postgres port free: `ss -tlnp`. The host runs other services; the
  deploy refuses a `TBD_REMOTE_DIR`, `TBD_SSH_HOST` or `TBD_PROFILE_DIR` that contains
  `prairielearn` in any case, and a `TBD_REMOTE_DIR` outside the fixed prefix
  `require_tbd_remote_prefix` checks (`tools_v2/xtask/src/commands/deploy/website.rs`), because the
  rsync runs with `--delete`.
- Disk space: the website needs little; the game server beside it needs at least 30 GB
  (`df -h ~`).

## Steps

Run the development-machine steps from the repository root; a step that runs on the host says so.

### Phase A — Configure the deploy

1. Create the deploy settings from the template, then set `TBD_SSH_HOST`, either
   `TBD_SSH_IDENTITY_FILE` or `TBD_SSH_PASS`, `TBD_REMOTE_DIR` and, when 5432 is taken on the host,
   `TBD_POSTGRES_HOST_PORT`. The file is gitignored and never rsynced; the commands parse it as
   `KEY=VALUE` lines and never run it. Every key the website deploy reads is in the
   [deployment templates README](/tools_v2/xtask/deploy/README.md#configuration).

   ```bash
   cp tools_v2/xtask/deploy/deploy.env.example tools_v2/xtask/deploy/deploy.env
   ```

   Expected: no output. A `DEPLOY_ENV` environment variable points `deploy website`, and only
   that command, at another file.

2. Print the plan without contacting the host.

   ```bash
   cargo xtask deploy website --dry-run
   ```

   Expected, exit 0: `==> deploy-website → <TBD_SSH_HOST>:<TBD_REMOTE_DIR>`, then the map-asset
   probe, the rsync with one `[dry-run]   --exclude=` line per protected path (`.git/`,
   `target/`, the gate build folders, `node_modules/`, `apps/website/frontend/dist/`, the server's
   `apps/website/api_v2/.env`, `tools_v2/xtask/deploy/deploy.env`, `assets_v2/terrains/`,
   `assets_v2/scratch/`, `packages/` and the reference mod folders), the five remote steps below,
   `==> remote: restart tbd-website-api.service`, the Caddy hint,
   `==> unit: tools_v2/xtask/deploy/systemd/tbd-website-api.service is installed by hand (see documentation_v2/runbooks/website_deployment.md Phase D)`,
   the smoke hints and `==> done`. The printed list is the authority; the code is
   `tools_v2/xtask/src/commands/deploy/website/rsync_argv.rs`.

   | Remote step, as printed | What runs on the host |
   |---|---|
   | `staging Postgres (docker compose)` | `compose -f apps/website/docker-compose.staging.yml up -d postgres` with `TBD_POSTGRES_HOST_PORT`; skipped by `TBD_SKIP_COMPOSE=1` |
   | `cargo build --release -p website-api --bin api` | the release API into `target/release/api`; skipped by `TBD_SKIP_API_BUILD=1` |
   | `trunk build --release (Leptos SPA → frontend/dist)` | the app into `apps/website/frontend/dist`; skipped by `TBD_SKIP_SPA_BUILD=1` |
   | `repoint the checksums of comments-only migration edits` | `TBD_DB_CONTAINER=tbd_staging_db cargo xtask db repair-migration-checksum --force` |
   | `move runtime files into the unit's state directory` | creates `~/.local/state/tbd-website-api/uploads` and moves an `uploads` folder left inside `apps/website/api_v2/` into it |

   A failing step stops the deploy before the restart, so the running API keeps serving the
   previous build. A failed restart only warns: the code is on the host by then.

### Phase B — Prepare the host

3. Create the checkout folder. The deploy's first act is a probe that `cd`s into it and refuses
   when it cannot (probe exit 12).

   ```bash
   ssh <TBD_SSH_HOST> 'mkdir -p <TBD_REMOTE_DIR>'
   ```

   Expected: no output.

4. On the host, let the deploy user's systemd user units run while nobody is logged in; the API
   unit and the backup timers need it.

   ```bash
   sudo loginctl enable-linger "$USER"
   ```

   Expected: no output; `loginctl show-user "$USER" --property=Linger` prints `Linger=yes`.

The staging Postgres takes its password from `POSTGRES_PASSWORD` in the environment of the
deploy's login shell (the remote steps run through `bash -lc`), and falls back to `CHANGE_ME`.
Export a real one in the deploy user's `~/.profile` before the first deploy: the container keeps
the password its volume was first created with. The database listens on the host's loopback only.

### Phase C — Sync, build and start Postgres

5. Run the deploy.

   ```bash
   cargo xtask deploy website
   ```

   Expected on the first run: the probe prints
   `WARN: no map asset tree on the server (neither assets_v2/terrains nor packages/map-assets).`
   and continues, rsync lists the files it copies, compose starts `tbd_staging_db`, both builds
   finish, and then the checksum repair stops the deploy with
   `could not read _sqlx_migrations (psql exit 1)` and exit 1, because a fresh database has no
   migration table until the API first boots. Phase D boots it; the [redeploy](#redeploy) then
   runs through. Once the API has applied its migrations, the deploy runs to the end: it prints
   `WARN: systemctl restart failed — is tbd-website-api.service installed?` and the install line
   while the unit is missing, and `==> done` either way.

The rsync mirrors the checkout with `--delete`: a file on the host outside the excluded paths
disappears at the next deploy, so the server keeps its own files only at `apps/website/api_v2/.env`,
under `assets_v2/terrains/` and outside the checkout. The glyph atlas `assets_v2/glyphs/` is not
excluded; it is tracked and arrives with every deploy.

A host that serves only the [mission](/documentation_v2/glossary.md#mission) library needs no map assets: every `/map-assets` request
answers 404 and the deploy warns and continues. For the
[Mission Creator](/documentation_v2/glossary.md#mission-creator), the host needs its own copy of
the terrain tree (about 590 MB of LFS content, never rsynced) at `assets_v2/terrains/` in the
checkout, with `terrain-registry.json` at its top; see
[Terrain assets](/assets_v2/terrains/README.md) for what the tree holds.

### Phase D — Install and run the API

6. On the host, create the server's API settings from the template that arrived with the rsync.

   ```bash
   install -m 600 <TBD_REMOTE_DIR>/apps/website/api_v2/.env.example <TBD_REMOTE_DIR>/apps/website/api_v2/.env
   ```

   Expected: no output; the file is readable by the deploy user only. Set these values; the full
   list, with defaults and failure modes, is
   [API environment variables](/documentation_v2/website/api_v2/environment_variables.md).

   | Variable | Value on the host |
   |---|---|
   | `PORT` | `8080`, which Caddy and the smoke hints expect |
   | `APP_ENV` | `production` (any value but `development`). `development` registers the [dev login](/documentation_v2/glossary.md#dev-login) and relaxes the checks below: use it only for a LAN-only first smoke, never behind the tunnel |
   | `FRONTEND_URL`, `ALLOWED_ORIGINS` | the public origin, `https://<site host>` |
   | `DATABASE_URL` | `postgres://tbd:<POSTGRES_PASSWORD>@127.0.0.1:<TBD_POSTGRES_HOST_PORT>/tbd_reforger?sslmode=disable` |
   | `JWT_SECRET` | the output of `openssl rand -hex 32` |
   | `JWT_ACCESS_TTL_MIN` | `15`, the template's value |
   | `TRUSTED_PROXIES` | `127.0.0.1/32`: Caddy on the loopback is the only proxy believed |
   | `DISCORD_CLIENT_ID`, `DISCORD_CLIENT_SECRET`, `DISCORD_REDIRECT_URL` | required outside development; the redirect is `https://<site host>/api/v1/auth/discord/callback` |
   | `DISCORD_GUILD_ID`, `DISCORD_BOT_TOKEN`, `DISCORD_WEBHOOK_URL` | optional; empty turns the path that needs them off |
   | `SERVICE_TOKEN` | the output of `openssl rand -hex 24`; the game server's `TBD_GAME_SERVER_TOKEN` in `deploy.env` carries the same value |

   The unit sets `MAP_ASSETS_DIR`, `GLYPH_ASSETS_DIR` and `UPLOAD_DIR` itself; leave them out of
   the file.

7. On the host, create the user unit folder.

   ```bash
   mkdir -p ~/.config/systemd/user
   ```

   Expected: no output.

8. On the host, from `<TBD_REMOTE_DIR>`, render the unit template into it. The placeholder takes
   `TBD_REMOTE_DIR` without its leading slash, because the template already writes
   `/TBD_REPO_DIR_PLACEHOLDER/…`.

   ```bash
   sed 's|TBD_REPO_DIR_PLACEHOLDER|<TBD_REMOTE_DIR without its leading slash>|g' tools_v2/xtask/deploy/systemd/tbd-website-api.service > ~/.config/systemd/user/tbd-website-api.service
   ```

   Expected: no output. The unit runs `target/release/api` from `apps/website/api_v2`, loads that
   folder's `.env`, pins `MAP_ASSETS_DIR` and `GLYPH_ASSETS_DIR` to the checkout's
   `assets_v2/terrains` and `assets_v2/glyphs`, and keeps uploads under its state directory
   (`StateDirectory=tbd-website-api`); the
   [systemd README](/tools_v2/xtask/deploy/systemd/README.md#configuration) explains each line.
   When a deploy's restart fails, the deploy prints steps 7 to 10 as one line
   (`install_command` in `tools_v2/xtask/src/commands/deploy/website/systemd_unit.rs`).

9. On the host, make systemd read the new unit.

   ```bash
   systemctl --user daemon-reload
   ```

   Expected: no output.

10. On the host, enable the unit and start the API. It applies the pending migrations at boot.

    ```bash
    systemctl --user enable --now tbd-website-api.service
    ```

    Expected: `Created symlink … → …/tbd-website-api.service`.

11. On the host, follow the boot.

    ```bash
    journalctl --user -u tbd-website-api -f
    ```

    Expected: the log lines `migrations applied` and `listening on 0.0.0.0:8080`. A missing
    setting stops the boot with `<VARIABLE> is required` (see Troubleshooting).

The staging compose file also carries an `api` profile that runs the API in a container
(`apps/website/Dockerfile`), with its settings in the compose `environment` block and its uploads in
a named volume. The deploy does not use it, and it binds the same `127.0.0.1:8080` as the unit, so
run one or the other.

### Phase E — Caddy and the Cloudflare Tunnel

12. On the host, check the site root in the Caddyfile. `tools_v2/xtask/deploy/Caddyfile.website`
    names the built app's folder as a fixed absolute path, which matches the template's
    `TBD_REMOTE_DIR`; edit that `root` line on the host when the checkout sits elsewhere, then
    confirm it.

    ```bash
    grep -n 'root \*' <TBD_REMOTE_DIR>/tools_v2/xtask/deploy/Caddyfile.website
    ```

    Expected: one line naming `<TBD_REMOTE_DIR>/apps/website/frontend/dist`.

13. On the host, start Caddy on the site. Later changes to the file take
    `caddy reload --config` with the same path, the line every deploy prints.

    ```bash
    caddy start --config <TBD_REMOTE_DIR>/tools_v2/xtask/deploy/Caddyfile.website
    ```

    Expected: Caddy listens on `:3080`, sends `Cross-Origin-Opener-Policy: same-origin` and
    `Cross-Origin-Embedder-Policy: credentialless` (the Mission Creator's WebAssembly needs both),
    proxies `/api/*`, `/uploads/*`, `/map-assets/*` and `/healthz` to `127.0.0.1:8080`, and serves
    every other path from the built app with an `index.html` fallback.

14. Publish the site through the tunnel. No command: in Cloudflare Zero Trust, under Networks and
    Tunnels, add a public hostname for the site to the host's tunnel with the service
    `http://127.0.0.1:3080`, and point the DNS name at the tunnel. The site gets its own hostname,
    so other services on the host keep their cookies and OAuth apps apart.

    Expected: the hostname loads the app in a browser.

15. Register the Discord redirect. No command: in the Discord developer portal, add
    `https://<site host>/api/v1/auth/discord/callback` to the OAuth2 redirects of the production
    application; it matches `DISCORD_REDIRECT_URL`, and `FRONTEND_URL` and `ALLOWED_ORIGINS` name
    the same host.

    Expected: signing in with Discord returns to the app's `/auth/callback` page and signs in.

### Redeploy

16. Deploy the current checkout. This is the whole procedure after the first deployment.

    ```bash
    cargo xtask deploy website
    ```

    Expected: every step of the dry run in step 2 runs, the restart prints `active`, and the deploy
    ends with `==> done`. The checksum repair prints
    `every applied migration examined matches its file. Nothing to repair.` unless a migration's
    comments changed. Caddy serves the new build at once; it reads the built files per request.

### Game server credentials

Each game server signs in to the API with its own
[machine credentials](/documentation_v2/glossary.md#machine-credential), which an administrator
issues on the [Server Control](/documentation_v2/glossary.md#server-control) page (`/admin/server`; `POST /api/v1/servers/{id}/credentials`). The
secret, `tbdm_…`, is shown once; revoking it stops its executor at the next request.

| Executor | Credential kind | Where the secret goes | Commands it runs |
|---|---|---|---|
| the game runtime (the mod) | `mod_runtime` | `TBD_MOD_RUNTIME_CREDENTIAL` in `deploy.env`, written into the server profile's `TBD_BackendConfig.json` as `machineCredential` | `broadcast`, `kick`, `load_mission`; runtime sessions, heartbeats, roster reads |
| the [fleet host agent](/documentation_v2/glossary.md#fleet-host-agent) | `host_agent` | `TBD_HOST_AGENT_CREDENTIAL` in `deploy.env`, written to `~/.config/fleet-host-agent/machine-credential` | `start`, `stop`, `restart`, `list_players`, `restart_with_mission` |

`cargo xtask deploy staging` installs both; which mission a server runs is a
[mission deployment](/documentation_v2/glossary.md#mission-deployment). Issuing them step by step,
and the first deployment, are in
[Give the staging server its credentials and a mission](/documentation_v2/runbooks/game_server_staging/machine_credentials_and_mission_deployment.md).

## Verify

Run the checks on the host unless the row says otherwise.

| Check | Command | Pass |
|---|---|---|
| ssh, from the development machine | `ssh <TBD_SSH_HOST> true` | exit 0 |
| Postgres | `docker exec tbd_staging_db pg_isready -U tbd -d tbd_reforger` (or `podman exec`) | `accepting connections` |
| API | `curl -sf http://127.0.0.1:8080/healthz` | `{"status":"ok"}` |
| app through Caddy | `curl -sfI http://127.0.0.1:3080/` | `HTTP/1.1 200 OK` |
| API through Caddy | `curl -sf http://127.0.0.1:3080/healthz` | `{"status":"ok"}` |
| tunnel | the public hostname in a browser | the app loads |
| sign-in | Discord sign-in in the browser | back on `/auth/callback`, signed in |
| isolation | `ss -tlnp` | the website holds only 3080, 8080 and the Postgres port; the other services keep theirs |

`/healthz` answers 503 with `{"status":"unavailable"}` while the database is down or the
migrations are unreadable. The API has no other health route.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `Missing <path> — copy from tools_v2/xtask/deploy/deploy.env.example`, exit 1 | no `deploy.env`, or `DEPLOY_ENV` names a missing file | step 1 |
| `deploy.env: line 79: TBD_SSH_HOST: TBD_SSH_HOST required in deploy.env`, exit 1 (`line 80` for `TBD_REMOTE_DIR`) | the value is missing or empty; the line number is fixed in the message, not the file's line | set the value |
| `Refusing to deploy: TBD_REMOTE_DIR must be under …`, or `… must not contain 'prairielearn'`, exit 1 | the folder is outside the deploy prefix, contains `..`, or names the other service | point `TBD_REMOTE_DIR` inside the prefix |
| `ERROR: could not determine the server's map asset layout (probe exit 12)` | `TBD_REMOTE_DIR` does not exist on the host; exit 255 means ssh itself failed | step 3; check ssh with the Verify row |
| `ERROR: <dir>/assets_v2/terrains is missing, but the pre-relocation packages/map-assets is present.` | the host keeps its terrain tree at the old place, which this build does not serve | on the host, move `packages/map-assets/everon`, `arland` and `terrain-registry.json` into `assets_v2/terrains/`, as the message prints, then deploy again |
| `sshpass: command not found`, exit 127 | `TBD_SSH_PASS` is set without sshpass installed | install sshpass, or use `TBD_SSH_IDENTITY_FILE` |
| `could not read _sqlx_migrations (psql exit 1)` during the checksum repair | the database has never seen the API boot | Phase D, then [redeploy](#redeploy) |
| `WARN: systemctl restart failed — is tbd-website-api.service installed?` | the unit is not installed, or its boot failed | Phase D; the journal shows why a boot failed |
| the API refuses to boot with `migration N was previously applied but has been modified` | a comments-only edit to an applied migration; sqlx hashes the whole file | the deploy repairs it before each restart; by hand, [repair a migration checksum](/documentation_v2/runbooks/database_operations.md#repair-a-migration-checksum) (step 2 there, on the host). Never reset the database for it |
| the journal shows `DISCORD_CLIENT_ID is required` (or the secret or redirect) | outside development the three Discord settings are required | step 6 |
| `UPLOAD_DIR is malformed: must be an absolute path outside development` | the API was started without the unit, which sets it | start it through the unit (steps 8 to 10) |
| every `/map-assets` request answers 404 and the journal says nothing | the terrain tree is missing, or the API runs with another working directory; the asset server never checks its root | put the tree at `assets_v2/terrains/`, and run the API through the unit, which pins both folders |
| the Mission Creator reports that `SharedArrayBuffer` is missing | the page is not served through the Caddyfile, so it lacks the cross-origin isolation headers | serve the app through Caddy (step 13) |
| the API stops when the deploy user logs out | lingering is off | step 4 |
| the backup timers fail to find their container | the backup units name `tbd_reforger_db`; the deploy's compose starts `tbd_staging_db` | [Database operations](/documentation_v2/runbooks/database_operations.md#schedule-the-home-servers-backups) |

## Related

- [Local development](/documentation_v2/runbooks/local_development.md) — the same stack on a
  developer machine.
- [Database operations](/documentation_v2/runbooks/database_operations.md) — backups, restore
  drills, the backup timers and the checksum repair on the home server.
- [Game server staging](/documentation_v2/runbooks/game_server_staging/README.md) — the dedicated
  game server and its host agent on the same host.
- [Testing and CI](/documentation_v2/runbooks/testing_and_ci.md) — the gates a change passes before
  it is deployed.
- [API environment variables](/documentation_v2/website/api_v2/environment_variables.md) — every
  setting of the server's `.env`.
- [Deployment templates](/tools_v2/xtask/deploy/README.md) — `deploy.env`, the Caddyfile and the
  units; [the deploy commands](/tools_v2/xtask/src/commands/deploy/website/README.md) — how
  `deploy website` works.
