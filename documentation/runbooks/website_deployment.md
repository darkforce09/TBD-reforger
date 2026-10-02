**Status:** live

# Website deployment

Deploys the website [API](/documentation/glossary/a_to_f.md#api) and the single-page app to the home
server: `cargo xtask deploy website` copies the checkout to the host, starts the staging Postgres,
builds the release API, the staging host tools and the app there, starts the Caddy web server and
reloads its configuration, and restarts the API's systemd user unit. Caddy serves the app on port
3080 and a Cloudflare Tunnel can publish it. Postgres and Caddy run from the staging compose file, so the
container runtime brings both back after a reboot. The first
[deployment](/documentation/glossary/a_to_f.md#deployment) runs Phases A to E once, about an hour with
the server-side builds; every later one is the single command under
[Redeploy](#redeploy). The fleet of dedicated game servers on the same host has its own runbook,
[Game server staging](/documentation/runbooks/game_server_staging/README.md).

```text
browser ──▶ Cloudflare Tunnel (optional, Phase E) ──▶ Caddy :3080  (container tbd_staging_caddy, host network)
            cloudflared on 127.0.0.1, the one peer    │  site: deploy/caddy/Caddyfile
            whose X-Forwarded-For Caddy keeps         │
                                                      ├── /api/*, /uploads/*, /map-assets/*, /healthz
                                                      │     ──▶ API 0.0.0.0:8080  (tbd-website-api.service, Phase D)
                                                      └── every other path ──▶ apps/frontend/dist
API ──▶ Postgres tbd_staging_db on 127.0.0.1:${TBD_POSTGRES_HOST_PORT:-5432}
API uploads ──▶ ~/.local/state/tbd-website-api/uploads  (outside the checkout)
API equipment data ──▶ ~/.local/state/tbd-website-api/equipment  (outside the checkout)
host tools ──▶ target/release/staging-fixtures  (the staging verification harness runs it)
           ──▶ target/release/acknowledgement-dropping-relay  (cargo xtask deploy staging installs it)
deploy/compose.staging.yml runs tbd_staging_caddy and tbd_staging_db (restart: unless-stopped)
```

The host is whatever `TBD_SSH_HOST` names in `deploy/deploy.env`; this runbook
writes it, and the checkout folder `TBD_REMOTE_DIR`, as placeholders.

## Prerequisites

On the development machine:

- ssh and rsync, plus sshpass when the host takes a password (`TBD_SSH_PASS`). Check:
  `command -v ssh rsync`.
- The Rust toolchain, for `cargo xtask`.
- ssh access to the host as the deploy user; a key (`TBD_SSH_IDENTITY_FILE`) is preferred over a
  password. `TBD_SSH_HOST` is `user@<host>.local`: the host answers as `<host>.local` via avahi and
  keeps its address through a router DHCP reservation, because the game server's public address is
  fixed at deploy time.

On the host, which [Prepare the staging host](/documentation/runbooks/game_server_staging/host_preparation.md)
sets up on Ubuntu:

- docker or podman with a compose provider; the deploy tries `docker compose` first, then
  `podman compose`. Check: `docker compose version` or `podman compose version`. The runtime
  must start at boot, because it is what restarts Postgres and Caddy: `systemctl is-enabled docker`
  prints `enabled`; on a Podman host, `systemctl --user enable podman-restart.service` does it for
  the deploy user.
- No Caddy install: Caddy runs in the `caddy` service of `deploy/compose.staging.yml`,
  and the deploy pulls its image.
- A C toolchain (`build-essential`) and rustup under `~/.cargo/bin`, with Trunk installed there
  too: the remote build steps put only `$HOME/.cargo/bin` on `PATH`. The root
  `rust-toolchain.toml` pins 1.95.0 with the `wasm32-unknown-unknown` target, and rustup installs
  it on the first build. Check: `~/.cargo/bin/trunk --version`.
- cloudflared, only for the tunnel in Phase E.
- Ports 3080, 8080, 2019 (Caddy's admin endpoint, on localhost) and the Postgres port free:
  `ss -tlnp`. The host runs other services; the deploy refuses a `TBD_REMOTE_DIR`,
  `TBD_SSH_HOST` or `TBD_PROFILE_DIR` that contains `prairielearn` in any case, and a
  `TBD_REMOTE_DIR` outside the deploy user's `/home/<user>/tbd/` that
  `require_tbd_remote_prefix` checks (`tools/xtask/src/commands/deploy/website.rs`), because
  the rsync runs with `--delete`.
- Disk space: the website needs little; the game server install beside it needs at least 30 GB
  (`df -h ~`).

## Steps

Run the development-machine steps from the repository root; a step that runs on the host says so.

### Phase A — Configure the deploy

1. Create the deploy settings from the template, then set `TBD_SSH_HOST` (`user@host`), either
   `TBD_SSH_IDENTITY_FILE` or `TBD_SSH_PASS` when plain ssh does not log in, `TBD_REMOTE_DIR` when
   the checkout is not `/home/<user>/tbd/repo` and, when 5432 is taken on the host,
   `TBD_POSTGRES_HOST_PORT`. The file is gitignored and never rsynced; the commands parse it as
   `KEY=VALUE` lines and never run it. A key the file sets beats the process environment, and an
   empty value there counts as unset. Every key the website deploy reads is in the
   [deployment templates README](/deploy/README.md#configuration).

   ```bash
   cp deploy/deploy.env.example deploy/deploy.env
   ```

   Expected: no output. A `DEPLOY_ENV` environment variable points `deploy website`, and every
   other command that reads the file, at another one.

2. Print the plan without contacting the host.

   ```bash
   cargo xtask deploy website --dry-run
   ```

   Expected, exit 0: `==> deploy-website → <TBD_SSH_HOST>:<TBD_REMOTE_DIR>`, then the map-asset
   probe, the rsync with one `[dry-run]   --exclude=` line per protected path (`.git/`,
   `target/`, the gate build folders, `node_modules/`, `apps/frontend/dist/`, the server's
   `apps/api/.env`, `deploy/deploy.env`, `assets/terrains/`,
   `assets/scratch/`, `packages/` and the reference mod folders, then what only the
   development machine holds, anchored at the checkout root: `/target-*/` and the other build
   folders, the worktrees under `.ai/artifacts/worktrees/`, the wave gate's receipts, and the
   local files of Claude Code, Codex and `.mcp.json`), the six remote steps below,
   `==> remote: restart tbd-website-api.service`,
   `==> unit: deploy/systemd/tbd-website-api.service is installed by hand (see documentation/runbooks/website_deployment.md Phase D)`,
   the smoke hints and `==> done`. The printed list is the authority; the code is
   `tools/xtask/src/commands/deploy/website/rsync_argv.rs`, and the development-machine-only
   paths, which `cargo xtask deploy staging` excludes too, are in
   `tools/xtask/src/commands/deploy/development_machine_only_paths.rs`.

   | Remote step, as printed | What runs on the host |
   |---|---|
   | `staging Postgres (docker compose)` | `compose -f deploy/compose.staging.yml up -d postgres` with `TBD_POSTGRES_HOST_PORT`; skipped by `TBD_SKIP_COMPOSE=1` |
   | `cargo build --release -p api --bin api` | the release API into `target/release/api`; skipped by `TBD_SKIP_API_BUILD=1` |
   | `cargo build --release: the staging host tools (staging-fixtures, acknowledgement-dropping-relay)` | `cargo build --release -p api --bin staging-fixtures`, then `-p developer_tools --bin acknowledgement-dropping-relay`, each proven by `test -x target/release/<executable>`: the staging verification harness runs `staging-fixtures` on the host, and `cargo xtask deploy staging` installs the relay; skipped by `TBD_SKIP_API_BUILD=1` |
   | `trunk build --release (Leptos SPA → frontend/dist)` | the app into `apps/frontend/dist`; skipped by `TBD_SKIP_SPA_BUILD=1` |
   | `staging Caddy on :3080 (docker compose), then reload its Caddyfile` | `compose … up -d caddy`, then `compose … exec -T caddy caddy reload --config /etc/tbd-caddy/Caddyfile --adapter caddyfile`, up to 5 attempts a second apart; skipped by `TBD_SKIP_COMPOSE=1`, and still run with `TBD_SKIP_SPA_BUILD=1`, serving the `dist` already on the host |
   | `repoint the checksums of comments-only migration edits` | `TBD_DB_CONTAINER=tbd_staging_db cargo xtask db repair-migration-checksum --force` |

   Every compose step runs from `<TBD_REMOTE_DIR>` with `TBD_POSTGRES_HOST_PORT` exported, under
   `docker compose` when the host has docker and `podman compose` otherwise.

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

### Phase C — Sync, build and start the compose services

5. Run the deploy.

   ```bash
   cargo xtask deploy website
   ```

   Expected on the first run: the probe prints
   `WARN: no map asset tree on the server (neither assets/terrains nor packages/map-assets).`
   and continues, rsync lists the files it copies, compose starts `tbd_staging_db`, the three
   builds finish, compose starts `tbd_staging_caddy` and Caddy takes the Caddyfile, and then the checksum
   repair stops the deploy with
   `could not read _sqlx_migrations (psql exit 1)` and exit 1, because a fresh database has no
   migration table until the API first boots. Phase D boots it; the [redeploy](#redeploy) then
   runs through. Once the API has applied its migrations, the deploy runs to the end: it prints
   `WARN: systemctl restart failed — is tbd-website-api.service installed?` and the install line
   while the unit is missing, and `==> done` either way.

The rsync mirrors the checkout with `--delete`: a file on the host outside the excluded paths
disappears at the next deploy, so the server keeps its own files only at `apps/api/.env`,
under `assets/terrains/` and outside the checkout. The glyph atlas `assets/glyphs/` is not
excluded; it is tracked and arrives with every deploy. An excluded path that is already on the
host is neither updated nor deleted: a development machine's build folder or tool state found
there stays until it is removed by hand.

A host that serves only the [mission](/documentation/glossary/g_to_m.md#mission) library needs no map assets: every `/map-assets` request
answers 404 and the deploy warns and continues. For the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator), the host needs its own copy of
the terrain tree (about 590 MB of LFS content, never rsynced) at `assets/terrains/` in the
checkout, with `terrain-registry.json` at its top; see
[Terrain assets](/assets/terrains/README.md) for what the tree holds.

### Phase D — Install and run the API

6. On the host, create the server's API settings from the template that arrived with the rsync.

   ```bash
   install -m 600 <TBD_REMOTE_DIR>/apps/api/.env.example <TBD_REMOTE_DIR>/apps/api/.env
   ```

   Expected: no output; the file is readable by the deploy user only. Set these values; the full
   list, with defaults and failure modes, is
   [API environment variables](/documentation/apps/api/environment_variables.md).

   | Variable | Value on the host |
   |---|---|
   | `PORT` | `8080`, which Caddy and the smoke hints expect |
   | `APP_ENV` | `production` (any value but `development`). `development` registers the [dev login](/documentation/glossary/a_to_f.md#dev-login) and relaxes the checks below: use it only for a LAN-only first smoke, never behind the tunnel |
   | `FRONTEND_URL`, `ALLOWED_ORIGINS` | the public origin, `https://<site host>` |
   | `DATABASE_URL` | `postgres://tbd:<POSTGRES_PASSWORD>@127.0.0.1:<TBD_POSTGRES_HOST_PORT>/tbd_reforger?sslmode=disable` |
   | `JWT_SECRET` | the output of `openssl rand -hex 32` |
   | `JWT_ACCESS_TTL_MIN` | `15`, the template's value |
   | `TRUSTED_PROXIES` | `127.0.0.1/32`: Caddy on the loopback is the only proxy believed; Caddy in turn keeps a forwarded address from the tunnel alone ([forwarded client addresses](#forwarded-client-addresses)) |
   | `DISCORD_CLIENT_ID`, `DISCORD_CLIENT_SECRET`, `DISCORD_REDIRECT_URL` | required outside development; the redirect is `https://<site host>/api/v1/auth/discord/callback` |
   | `DISCORD_GUILD_ID`, `DISCORD_BOT_TOKEN`, `DISCORD_WEBHOOK_URL` | optional; empty turns the path that needs them off |
   | `OBSERVABILITY_TOKEN` | the output of `openssl rand -hex 32`; the bearer a scraper sends to `/metrics` and the detailed `/healthz`, and nothing else accepts it |

   The unit sets `MAP_ASSETS_DIR`, `GLYPH_ASSETS_DIR`, `UPLOAD_DIR` and `EQUIPMENT_DATA_DIR`
   itself; leave them out of the file, as the template does. systemd lets a value in the file
   override the unit's own, and outside development a relative `UPLOAD_DIR` or
   `EQUIPMENT_DATA_DIR` stops the boot.

7. On the host, create the user unit folder.

   ```bash
   mkdir -p ~/.config/systemd/user
   ```

   Expected: no output.

8. On the host, from `<TBD_REMOTE_DIR>`, render the unit template into it. The placeholder takes
   `TBD_REMOTE_DIR` without its leading slash, because the template already writes
   `/TBD_REPO_DIR_PLACEHOLDER/…`.

   ```bash
   sed 's|TBD_REPO_DIR_PLACEHOLDER|<TBD_REMOTE_DIR without its leading slash>|g' deploy/systemd/tbd-website-api.service > ~/.config/systemd/user/tbd-website-api.service
   ```

   Expected: no output. The unit runs `target/release/api` from `apps/api`, loads that
   folder's `.env`, pins `MAP_ASSETS_DIR` and `GLYPH_ASSETS_DIR` to the checkout's
   `assets/terrains` and `assets/glyphs`, and keeps uploads and the imported equipment data
   under its state directory (`StateDirectory=tbd-website-api`); the
   [systemd README](/deploy/systemd/README.md#configuration) explains each line.
   When a deploy's restart fails, the deploy prints steps 7 to 10 as one line
   (`install_command` in `tools/xtask/src/commands/deploy/website/systemd_unit.rs`).

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
(`deploy/Dockerfile`), with its settings in the compose `environment` block and its uploads in
a named volume. The deploy does not use it, and it binds the same `127.0.0.1:8080` as the unit, so
run one or the other.

### Phase E — Caddy and the Cloudflare Tunnel

12. On the host, check that Caddy serves the app. Nothing is installed or started by hand: the
    deploy starts the compose file's `caddy` service, container `tbd_staging_caddy`, and reloads
    `deploy/caddy/Caddyfile` in it, so an edited Caddyfile applies with the next
    deploy. The service mounts `deploy/caddy/` at `/etc/tbd-caddy` and
    `apps/frontend/` at `/srv/tbd-frontend`, both read-only, and serves
    `/srv/tbd-frontend/dist`, wherever the checkout sits.

    ```bash
    curl -sfI http://127.0.0.1:3080/
    ```

    Expected: `HTTP/1.1 200 OK` with `Cross-Origin-Opener-Policy: same-origin` and
    `Cross-Origin-Embedder-Policy: credentialless` (the Mission Creator's WebAssembly needs both).
    Caddy proxies `/api/*`, `/uploads/*`, `/map-assets/*` and `/healthz` to `127.0.0.1:8080`, and
    serves every other path from the built app with an `index.html` fallback.

13. On the host, check that Postgres and Caddy come back after a reboot: the container runtime
    restarts both, because the compose file gives them `restart: unless-stopped`.

    ```bash
    docker inspect --format '{{.Name}} {{.HostConfig.RestartPolicy.Name}}' tbd_staging_caddy tbd_staging_db && systemctl is-enabled docker
    ```

    Expected: `/tbd_staging_caddy unless-stopped`, `/tbd_staging_db unless-stopped`, then
    `enabled`. On a Podman host, `podman inspect` with the same arguments prints the two names
    without the leading slash, and `systemctl --user is-enabled podman-restart.service` prints
    `enabled`.

14. Publish the site through the tunnel. No command: in Cloudflare Zero Trust, under Networks and
    Tunnels, add a public hostname for the site to the host's tunnel with the service
    `http://127.0.0.1:3080`, and point the DNS name at the tunnel. The site gets its own hostname,
    so other services on the host keep their cookies and OAuth apps apart.

    Expected: the hostname loads the app in a browser. cloudflared reaches Caddy from
    `127.0.0.1`, the one peer whose `X-Forwarded-For` the Caddyfile keeps, so the API sees each
    visitor's own address ([forwarded client addresses](#forwarded-client-addresses)).

15. Register the Discord redirect. No command: in the Discord developer portal, add
    `https://<site host>/api/v1/auth/discord/callback` to the OAuth2 redirects of the production
    application; it matches `DISCORD_REDIRECT_URL`, and `FRONTEND_URL` and `ALLOWED_ORIGINS` name
    the same host.

    Expected: signing in with Discord returns to the app's `/auth/callback` page and signs in.

### Redeploy

A host whose checkout is not yet in the layout of restructure stage S2 takes the one-time host
steps W1 to W3 of the [S2 operator steps](/documentation/archive/restructure_agent_briefs/s2_a2_to_a6.md#oc-deploy-operator-steps) around its next deploy.

16. Deploy the current checkout. This is the whole procedure after the first deployment.

    ```bash
    cargo xtask deploy website
    ```

    Expected: every step of the dry run in step 2 runs, the staging host tools among them, the
    restart prints `active`, and the deploy ends with `==> done`. The checksum repair prints
    `every applied migration examined matches its file. Nothing to repair.` unless a migration's
    comments changed. Caddy serves the new build at once, since it reads the built files per
    request, and the Caddy step's reload applies a changed Caddyfile.

### Game server credentials

Each game server signs in to the API with its own
[machine credentials](/documentation/glossary/g_to_m.md#machine-credential). The host runs a
fleet of five, "TBD Staging 1" to "TBD Staging 5", and no credential passes through `deploy.env`,
which refuses the retired `TBD_MOD_RUNTIME_CREDENTIAL` and `TBD_HOST_AGENT_CREDENTIAL`. In order:

- `cargo xtask deploy website` builds the host tool `staging-fixtures`;
- `cargo xtask staging provision-fleet` registers the five servers through it and writes each
  one's two credentials, mode 600, under `~/tbd/fleet/instance-N/secrets/` on the host;
- the operator writes the servers' join password into `~/tbd/fleet/join-password` (mode 600);
- `cargo xtask deploy staging --migrate-single-instance` retires the single server's units and
  runs the fleet; it refuses an instance whose credential files are missing.

`cargo xtask staging rotate-credential` replaces one credential (`--stage`, then `--promote`).
The [Server Control](/documentation/glossary/n_to_z.md#server-control) page (`/admin/server`;
`POST /api/v1/servers/{id}/credentials`) issues and revokes credentials too; a secret, `tbdm_…`,
is shown once, and revoking it stops its executor at the next request.

| Executor | Credential kind | File on the host | Commands it runs |
|---|---|---|---|
| the game runtime (the mod) | `mod_runtime` | `~/tbd/fleet/instance-N/secrets/mod-runtime-credential`, written into the instance profile's `TBD_BackendConfig.json` as `machineCredential` | `broadcast`, `kick`, `load_mission`; runtime sessions, heartbeats, roster reads |
| the [fleet host agent](/documentation/glossary/a_to_f.md#fleet-host-agent) | `host_agent` | `~/tbd/fleet/instance-N/secrets/host-agent-credential`, named by `~/.config/fleet_host_agent/instance-N/agent.toml` | `start`, `stop`, `restart`, `list_players`, `restart_with_mission`, `console_command` (one line to the server's RCON console, sent once) |

Which mission a server runs is a
[mission deployment](/documentation/glossary/g_to_m.md#mission-deployment). The fleet, its
ports and the first deployment are in
[Give the staging servers their credentials and a mission](/documentation/runbooks/game_server_staging/machine_credentials_and_mission_deployment.md)
and the [staging setup checklist](/documentation/runbooks/staging_verification/setup_checklist.md).

## Verify

Run the checks on the host unless the row says otherwise.

| Check | Command | Pass |
|---|---|---|
| ssh, from the development machine | `ssh <TBD_SSH_HOST> true` | exit 0 |
| Postgres | `docker exec tbd_staging_db pg_isready -U tbd -d tbd_reforger` (or `podman exec`) | `accepting connections` |
| Caddy container | `docker ps --filter name=tbd_staging_caddy --format '{{.Status}}'` (or `podman ps`) | `Up …` |
| API | `curl -sf http://127.0.0.1:8080/healthz` | `{"status":"ok"}` |
| app through Caddy | `curl -sfI http://127.0.0.1:3080/` | `HTTP/1.1 200 OK` |
| API through Caddy | `curl -sf http://127.0.0.1:3080/healthz` | `{"status":"ok"}` |
| after a reboot | the Caddy and Postgres rows again, with nothing started by hand | the same answers |
| tunnel | the public hostname in a browser | the app loads |
| sign-in | Discord sign-in in the browser | back on `/auth/callback`, signed in |
| isolation | `ss -tlnp` | the website holds only 3080, 8080, the Postgres port and Caddy's admin endpoint on `localhost:2019`; the other services keep theirs |
| host tools | `test -x <TBD_REMOTE_DIR>/target/release/staging-fixtures && test -x <TBD_REMOTE_DIR>/target/release/acknowledgement-dropping-relay` | exit 0 |
| forwarded addresses | the `rate_limit_buckets` read under [forwarded client addresses](#forwarded-client-addresses) | a row per visitor address, none new for `127.0.0.1` |

`/healthz` answers 503 with `{"status":"unavailable"}` while the database is down or the
migrations are unreadable. The API has no other health route.

### Forwarded client addresses

cloudflared reaches Caddy from `127.0.0.1` and reports the visitor in `X-Forwarded-For`. The
Caddyfile's global options trust that peer alone (`trusted_proxies static 127.0.0.1/32`): from it
Caddy keeps the header and appends `127.0.0.1`; from any other peer, a LAN client included, it
replaces the header with the peer's own address. The API trusts only Caddy
(`TRUSTED_PROXIES=127.0.0.1/32`) and keys its rate limits on the rightmost untrusted hop, so each
visitor and each LAN address keeps a bucket of its own rather than one `127.0.0.1` bucket for the
whole tunnel.

Prove Caddy's half on the development machine with podman before a deploy; nothing reaches the
host. From the repository root, in one shell:

```bash
proof="$(mktemp -d)" && printf '{\n\tadmin off\n}\n\n:8080 {\n\tbind 127.0.0.1\n\trespond "x-forwarded-for={header.X-Forwarded-For}"\n}\n' > "$proof/Caddyfile.echo"
podman network create tbd-forwarding-proof && podman pod create --name tbd-forwarding-proof --network tbd-forwarding-proof
podman run -d --pod tbd-forwarding-proof --name tbd-forwarding-proof-echo --security-opt label=disable -v "$proof:/etc/proof:ro" docker.io/library/caddy:2 caddy run --config /etc/proof/Caddyfile.echo --adapter caddyfile
podman run -d --pod tbd-forwarding-proof --name tbd-forwarding-proof-caddy --security-opt label=disable -v "$PWD/deploy/caddy:/etc/tbd-caddy:ro" docker.io/library/caddy:2 caddy run --config /etc/tbd-caddy/Caddyfile --adapter caddyfile
podman exec tbd-forwarding-proof-echo wget -qO- --header 'X-Forwarded-For: 203.0.113.7' http://127.0.0.1:3080/api/forwarding-proof
podman run --rm --network tbd-forwarding-proof docker.io/library/caddy:2 sh -c 'hostname -i; wget -qO- --header "X-Forwarded-For: 203.0.113.7" http://tbd-forwarding-proof:3080/api/forwarding-proof'
podman pod rm -f tbd-forwarding-proof && podman network rm tbd-forwarding-proof && rm -r "$proof"
```

Expected: the echo, on `127.0.0.1:8080` in the pod, answers what Caddy forwards to the API. The
probe from the pod's loopback, where cloudflared stands, prints
`x-forwarded-for=203.0.113.7, 127.0.0.1`; the probe from another container, where a LAN client
stands, prints that container's own address and then `x-forwarded-for=<that address>`, with the
claimed `203.0.113.7` gone. A Caddyfile without the `servers` block prints
`x-forwarded-for=127.0.0.1` for the first probe: every tunnel visitor would share one bucket.
`--security-opt label=disable` lets SELinux hosts read the mounts without relabelling the checkout.

After the deploy, on the host, sign in through the public hostname (its `/api/v1/auth/` requests
take the strict tier), then read the buckets, read-only:

```bash
docker exec -e PGOPTIONS='-c default_transaction_read_only=on' tbd_staging_db psql -X -A -U tbd -d tbd_reforger -c "SELECT bucket_key, updated_at FROM rate_limit_buckets ORDER BY updated_at DESC LIMIT 10"
```

Expected: a fresh `strict|<your public address>` row and no fresh `strict|127.0.0.1` row; a
request from a LAN machine to `http://<host>:3080/api/v1/auth/…` adds `strict|<its LAN address>`.
On a Podman host, `podman exec` takes the same arguments.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `Missing <path> — copy from deploy/deploy.env.example`, exit 1 | no `deploy.env`, or `DEPLOY_ENV` names a missing file | step 1 |
| `TBD_SSH_HOST is not set: add it to <path>`, exit 1 | the value is missing, or empty in the file | set the value |
| `<path>:<line>: TBD_SSH_HOST: …`, or `<path>:<line>: <message>`, exit 1 | the value on that line of the file is not `user@host`, or the line is not `KEY=VALUE` | fix that line |
| `Refusing to deploy: TBD_SSH_HOST must name the deploy user (user@host) …`, exit 1 | `TBD_SSH_HOST` names no user, so there is no `/home/<user>/tbd/` to hold `TBD_REMOTE_DIR` | write `user@host` |
| `Refusing to deploy: TBD_REMOTE_DIR must be under …`, or `… must not contain 'prairielearn'`, exit 1 | the folder is outside `/home/<user>/tbd/`, contains `..`, or names the other service | point `TBD_REMOTE_DIR` inside `/home/<user>/tbd/`, or leave it unset |
| `ERROR: could not determine the server's map asset layout (probe exit 12)` | `TBD_REMOTE_DIR` does not exist on the host; exit 255 means ssh itself failed | step 3; check ssh with the Verify row |
| `ERROR: <dir>/assets/terrains is missing, but the pre-relocation packages/map-assets is present.` | the host keeps its terrain tree at the old place, which this build does not serve | on the host, move `packages/map-assets/everon`, `arland` and `terrain-registry.json` into `assets/terrains/`, as the message prints, then deploy again |
| `sshpass: command not found`, exit 127 | `TBD_SSH_PASS` is set without sshpass installed | install sshpass, or use `TBD_SSH_IDENTITY_FILE` |
| `could not read _sqlx_migrations (psql exit 1)` during the checksum repair | the database has never seen the API boot | Phase D, then [redeploy](#redeploy) |
| `WARN: systemctl restart failed — is tbd-website-api.service installed?` | the unit is not installed, or its boot failed | Phase D; the journal shows why a boot failed |
| the API refuses to boot with `migration N was previously applied but has been modified` | a comments-only edit to an applied migration; sqlx hashes the whole file | the deploy repairs it before each restart; by hand, [repair a migration checksum](/documentation/runbooks/database_operations.md#repair-a-migration-checksum) (step 2 there, on the host). Never reset the database for it |
| the journal shows `DISCORD_CLIENT_ID is required` (or the secret or redirect) | outside development the three Discord settings are required | step 6 |
| `UPLOAD_DIR is required` | the API was started without the unit, which sets it | start it through the unit (steps 8 to 10) |
| `UPLOAD_DIR is malformed: must be an absolute path outside development` | the `.env` sets `UPLOAD_DIR` to a relative path, and a value in the `.env` overrides the unit's | delete that line from the `.env` (step 6), then `systemctl --user restart tbd-website-api.service` |
| `EQUIPMENT_DATA_DIR is malformed: must be an absolute path outside development` | the `.env` sets `EQUIPMENT_DATA_DIR` to a relative path, and a value in the `.env` overrides the unit's | delete that line from the `.env` (step 6), then `systemctl --user restart tbd-website-api.service` |
| every `/map-assets` request answers 404 and the journal says nothing | the terrain tree is missing, or the API runs with another working directory; the asset server never checks its root | put the tree at `assets/terrains/`, and run the API through the unit, which pins both folders |
| the Mission Creator reports that `SharedArrayBuffer` is missing | the page is not served through the Caddyfile, so it lacks the cross-origin isolation headers | open the site through Caddy on port 3080 or the tunnel (step 12) |
| the deploy stops at `staging Caddy on :3080 …`, and `docker logs tbd_staging_caddy` shows `address already in use` | another process holds port 3080 or Caddy's admin port 2019 on the host, such as a Caddy started outside the compose file | stop it (`caddy stop` for such a Caddy; `ss -tlnp` names the holder), then deploy again |
| the deploy stops at the Caddy step with `adapting config using caddyfile: …` | the Caddyfile does not parse; a running Caddy keeps its previous configuration | fix `deploy/caddy/Caddyfile`, then deploy again |
| the deploy stops at `cargo build --release: the staging host tools …` | the checkout does not build `staging-fixtures` or `acknowledgement-dropping-relay`, or a package no longer declares that `[[bin]]` | fix the build on the development machine (`cargo build --release -p <package> --bin <executable>`), then deploy again; `TBD_SKIP_API_BUILD=1` skips both cargo steps |
| `rate_limit_buckets` gains only `strict\|127.0.0.1` rows for tunnel traffic | Caddy runs a Caddyfile without the `servers { trusted_proxies … }` block, or cloudflared reaches Caddy from an address other than `127.0.0.1` | deploy again so the Caddy step reloads the committed Caddyfile; point the tunnel's service at `http://127.0.0.1:3080` (step 14) |
| every page but the API paths answers 404 | there is no `apps/frontend/dist` on the host: the app build was skipped (`TBD_SKIP_SPA_BUILD=1`) or never ran | deploy without `TBD_SKIP_SPA_BUILD` |
| Caddy and Postgres are gone after a reboot | the container runtime does not start at boot | `sudo systemctl enable --now docker`; on a Podman host, `systemctl --user enable --now podman-restart.service` with lingering on (step 4) |
| the API stops when the deploy user logs out | lingering is off | step 4 |
| the backup timers fail to find their container | the backup units name `tbd_reforger_db`; the deploy's compose starts `tbd_staging_db` | [Database operations](/documentation/runbooks/database_operations.md#schedule-the-home-servers-backups) |

## Related

- [Local development](/documentation/runbooks/local_development.md) — the same stack on a
  developer machine.
- [Database operations](/documentation/runbooks/database_operations.md) — backups, restore
  drills, the backup timers and the checksum repair on the home server.
- [Game server staging](/documentation/runbooks/game_server_staging/README.md) — the fleet of
  dedicated game servers and their host agents on the same host.
- [Staging verification](/documentation/runbooks/staging_verification/README.md) — the runs
  that call the host tools this deploy builds.
- [Testing and CI](/documentation/runbooks/testing_and_ci.md) — the gates a change passes before
  it is deployed.
- [API environment variables](/documentation/apps/api/environment_variables.md) — every
  setting of the server's `.env`.
- [Deployment templates](/deploy/README.md) — `deploy.env`, the Caddyfile and the
  units; [the deploy commands](/tools/xtask/src/commands/deploy/website/README.md) — how
  `deploy website` works.
