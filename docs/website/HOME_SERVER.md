# Home-server website setup — LAN (`dooley`)

Deploy the **TBD Reforger website** (Rust API + Leptos SPA) on the same home server used for PrairieLearn and the Reforger game staging stack.

**Scope of this doc:** LAN website (Postgres + API + SPA). Cloudflare Tunnel is optional later — not required for LAN.  
**Out of scope:** Arma dedicated server — see [`docs/mod/STAGING-SERVER.md`](../mod/STAGING-SERVER.md).

**Do not touch PrairieLearn.** TBD lives under `/home/sam/tbd/` only. Never write into `/home/sam/prairielearn/`.

**Connection protocol source (already on this PC):**  
[`/home/Samuel/Documents/PrairieLearn/PrairieLearn.md`](/home/Samuel/Documents/PrairieLearn/PrairieLearn.md) +  
[`/home/Samuel/Documents/PrairieLearn/HandoverContext.md`](/home/Samuel/Documents/PrairieLearn/HandoverContext.md)

---

## Live status (2026-07-10)

**Interim deploy.** What’s on the server today was built from the current main/worktree — **not** the Rust rewrite under audit. Do **not** re-upload until that audit is done and you explicitly ask for a full redeploy.

Host DHCP moved (was `.140`, now **`192.168.0.124`**, hostname `dooley`). Pin it (below) so reboots don’t break bookmarks/SSH.

| URL | What |
|-----|------|
| http://192.168.0.124:3080/ | **Interim** TBD Reforger SPA (nginx → static + `/api` proxy) |
| http://192.168.0.124:8081/healthz | **Interim** Rust API direct |
| http://127.0.0.1:8080/ (on server) | **Existing** older `Tbdevent_Website` stack — left running, do not stop |

**Additive only (shipped):** `/home/sam/tbd/website/` + containers `tbd-reforger-pg` (`127.0.0.1:5433`) + `tbd-reforger-web` (host-net `:3080`) + `bin/api` process on `:8081`.  
**Untouched:** `tbdevent_website-*`, PrairieLearn, Huly, n8n, etc.

Dev-login (temporary `APP_ENV=development`):  
http://192.168.0.124:3080/api/v1/auth/dev-login?role=admin  
(via nginx proxy to `:8081`)

### Sticky LAN IP (so it doesn’t change on reboot)

Best: **router DHCP reservation** — bind MAC of `wlo1` (or ethernet) to e.g. `192.168.0.140` or keep `.124`. Survives OS reinstalls.

On the Ubuntu box itself (Netplan), after you pick a permanent address:

```bash
# On server — inspect current Wi‑Fi connection name + MAC
ip -br link show wlo1
nmcli -t -f NAME,DEVICE,TYPE connection show --active
# Or edit Netplan under /etc/netplan/*.yaml: set addresses: [192.168.0.124/24],
# gateway4 / routes + nameservers, then: sudo netplan try
```

Prefer reservation over a hard-coded static if the router already manages the LAN — less chance of a clash.

---

## Target architecture (LAN)

```mermaid
flowchart LR
  browser[Browser_LAN]
  remote[dooley_LAN_IP]

  browser -->|"http://IP:3080"| reverse[nginx_host_3080]
  subgraph remote
    reverse -->|"/api /uploads /healthz"| api[API_0_0_0_0_8081]
    reverse -->|"SPA static"| spa[frontend_dist]
    pg[(Postgres_tbd_reforger_pg_5433)]
    pg --> api
    oldApi[Legacy_Tbdevent_Website_8080]
  end
```

| Piece | Bind | Notes |
|-------|------|--------|
| SPA + proxy | `0.0.0.0:3080` | Container `tbd-reforger-web` (`--network host`) |
| New API | `0.0.0.0:8081` | `/home/sam/tbd/website/bin/api` + `config.env` |
| New Postgres | `127.0.0.1:5433` | Volume `tbd_reforger_pgdata` — **not** the legacy `:5432` |
| Legacy API | `127.0.0.1:8080` | `tbdevent_website-api-1` — leave alone |
| Legacy Postgres | `127.0.0.1:5432` | `tbdevent_website-postgres-1` — leave alone |

---

## Server + SSH (from PrairieLearn protocol)

| Item | Value |
|------|--------|
| Host (LAN) | Current DHCP IP (verify with `hostname -I` on server; was `192.168.0.124` at last ship) |
| SSH user | `sam` |
| Auth | Password via `sshpass` (see PL handover) **or** SSH key — this PC’s key was **not** in `authorized_keys` yet |
| Server OS | Ubuntu Server (`dooley`) |
| TBD root | `/home/sam/tbd/` |
| New website tree | `/home/sam/tbd/website/` |
| Legacy monorepo snapshot | `/home/sam/tbd/repo` (old layout; do not confuse with current dev tree) |

```bash
# Discover IP if DHCP moved again
# On server: hostname -I
sshpass -p "$TBD_SSH_PASS" ssh -o StrictHostKeyChecking=no sam@192.168.0.124 'echo ok'
```

Prefer lasting setup: install this PC’s SSH public key on the server + `scripts/deploy/deploy.env`.

---

## Paths (isolated from PrairieLearn)

| Path | Purpose |
|------|---------|
| `/home/sam/tbd/repo` | Monorepo checkout / rsync target |
| `/home/sam/tbd/repo/apps/website/api_v2/.env` | **Server-only** secrets (never rsync from dev) |
| `/home/sam/tbd/repo/apps/website/frontend/dist` | Built SPA |
| `/home/sam/tbd/website-data/postgres` | Optional named volume / bind for DB (if not compose default) |
| `/home/sam/prairielearn/` | **Forbidden** for TBD |

Create layout:

```bash
ssh sam@192.168.0.140 'mkdir -p /home/sam/tbd/{repo,profile,addons-staging,website-data}'
```

---

## Honest gaps (as of 2026-07-27)

`apps/website/docker-compose.staging.yml` **exists** (T-251). Game deploy (`cargo xtask deploy staging`) and website deploy (`cargo xtask deploy website`) both compose from that path (T-438). Local laptop compose remains `apps/website/api_v2/docker-compose.yml` (Postgres on 5434). Remaining gaps below are still manual until SPA static hosting + COOP/COEP are one-button.

| Gap | Needed for “one command” website host |
|-----|----------------------------------------|
| Serve SPA from API **or** Caddy/nginx | `/` → `index.html`, `/api` → Axum (`scripts/deploy/Caddyfile.website` exists; wire still manual) |
| COOP/COEP headers on SPA | Same as Trunk (`same-origin` + `credentialless`) — required for map wasm / SAB |

---

## One-time prerequisites

### Dev PC

```bash
which sshpass rsync ssh curl git cargo docker
rustc --version  # toolchain pin: root rust-toolchain.toml
cp scripts/deploy/deploy.env.example scripts/deploy/deploy.env
# Set TBD_SSH_HOST=sam@192.168.0.140 and SSH pass or identity file
```

### Server

| Check | Command / note |
|-------|----------------|
| Disk | `df -h ~` (website alone is light; game staging needs ≥30 GB) |
| Docker | `docker compose version` |
| Ports free of TBD use | `ss -tlnp \| grep -E '8080|5432'` — PL uses **3001**, not 8080 |
| Linger (if user systemd) | `sudo loginctl enable-linger sam` |
| cloudflared | Already used for `tenta.icanteam.com`; add a route for TBD |

---

## Phase A — Postgres on the server

Dev compose maps host **5434**; on the home server prefer host **5432** (unless busy — then remap).

**Option A1 — temporary reuse of local compose on server (host port change):**

```bash
# On server, inside /home/sam/tbd/repo/apps/website/api_v2 after first sync
# Edit docker-compose.yml ports to "127.0.0.1:5432:5432" OR use apps/website/docker-compose.staging.yml
docker compose up -d db
```

**Option A2 — inline one-shot (no file yet):**

```bash
docker run -d --name tbd_reforger_db --restart unless-stopped \
  -e POSTGRES_USER=tbd -e POSTGRES_PASSWORD='CHANGE_ME' -e POSTGRES_DB=tbd_reforger \
  -p 127.0.0.1:5432:5432 \
  -v tbd_pgdata:/var/lib/postgresql \
  docker.io/library/postgres:18-alpine
```

Health: `docker exec tbd_reforger_db pg_isready -U tbd -d tbd_reforger`

---

## Phase B — Server `.env` (never commit, never rsync)

On the server:

```bash
cd /home/sam/tbd/repo/apps/website/api_v2
cp .env.example .env
chmod 600 .env
```

Recommended production values (adjust domain when tunnel is live):

```bash
PORT=8080
APP_ENV=production

# Public SPA origin (Cloudflare hostname)
FRONTEND_URL=https://tbd.icanteam.com
ALLOWED_ORIGINS=https://tbd.icanteam.com

# DB — if API runs on host (not in compose network):
DATABASE_URL=postgres://tbd:CHANGE_ME@127.0.0.1:5432/tbd_reforger?sslmode=disable
# If API shares Docker network with service name "postgres":
# DATABASE_URL=postgres://tbd:CHANGE_ME@postgres:5432/tbd_reforger?sslmode=disable

JWT_SECRET=<long-random-64+ hex>
JWT_ACCESS_TTL_MIN=15

# Behind Cloudflare / reverse proxy — trust tunnel/proxy CIDRs when wired in code
TRUSTED_PROXIES=127.0.0.1/32

# Discord (production OAuth — create a dedicated app redirect)
DISCORD_CLIENT_ID=
DISCORD_CLIENT_SECRET=
DISCORD_REDIRECT_URL=https://tbd.icanteam.com/api/v1/auth/discord/callback
DISCORD_GUILD_ID=
DISCORD_BOT_TOKEN=
DISCORD_WEBHOOK_URL=

SERVICE_TOKEN=<shared-with-game-server-if-used>
```

Notes:

- `APP_ENV=production` **disables** `GET /api/v1/auth/dev-login`. Use Discord OAuth for real users.
- For a first LAN-only smoke you may temporarily use `APP_ENV=development` + `FRONTEND_URL=http://192.168.0.140:3000` — do **not** leave that on a public tunnel.
- Game token must match `TBD_GAME_SERVER_TOKEN` in `scripts/deploy/deploy.env` when the Reforger server calls the API (see staging doc).

Generate secrets:

```bash
openssl rand -hex 32   # JWT_SECRET
openssl rand -hex 24   # SERVICE_TOKEN
```

---

## Phase C — Sync code + build website

From **dev PC** (repo root), first sync (exclude secrets + heavy junk):

```bash
source scripts/deploy/deploy.env
RSYNC_RSH="ssh -o StrictHostKeyChecking=no"
# Prefer key auth; sshpass if you still use password like PrairieLearn
rsync -avz --delete \
  --exclude '.git' \
  --exclude 'node_modules' \
  --exclude 'apps/website/frontend/node_modules' \
  --exclude 'apps/website/frontend/dist' \
  --exclude 'apps/website/api_v2/.env' \
  --exclude 'apps/website/api_v2/.tools/' \
  --exclude 'scripts/deploy/deploy.env' \
  --exclude 'target' \
  --exclude 'target-gate-*' \
  --exclude 'dist-gate-*' \
  --exclude 'apps/mod/crf_framework' \
  --exclude 'apps/mod/vanilla_reference' \
  --exclude 'apps/mod/playable_selector' \
  --exclude 'apps/mod/.local-test-profile' \
  --exclude 'assets_v2/terrains' \
  --exclude 'assets_v2/scratch' \
  --exclude 'packages' \
  ./ "${TBD_SSH_HOST}:${TBD_REMOTE_DIR}/"
```

This list mirrors the deploy's own, entry for entry, and exists only for a first sync before
`cargo xtask deploy website` can run on the box. **The authoritative set is the deploy's** — print it
with `cargo xtask deploy website --dry-run` and prefer that command once the server is reachable.
Two of these entries are secrets: `apps/website/api_v2/.env` is the server's own configuration
(rsyncing a dev copy overwrites it, and `--delete` is in this command), and
`scripts/deploy/deploy.env` holds `TBD_SSH_PASS` and `TBD_GAME_SERVER_TOKEN`.

`assets_v2/scratch` is ~1.5 GB of gitignored local export output; before the asset relocation it
sat inside the terrain tree and the one exclusion covered both. `packages` no longer exists in the
repo, and is excluded so that `--delete` cannot remove a server still holding its assets at the old
`packages/map-assets` path. Note that `assets_v2/glyphs` is deliberately **not** excluded — it is
188 KB, and the API has no other source for it.

Then on the **server**:

```bash
cd /home/sam/tbd/repo
# Toolchain: install Rust if missing, then from repo root:
cargo xtask mk leptos-build
cd /home/sam/tbd/repo/apps/website/api_v2
cargo build --release --bin api
```

Map assets: Mission Creator satellite/DEM bundles are large LFS. For a **library-only** site you can skip `assets_v2/terrains` initially — `/map-assets` then 404s, which `cargo xtask deploy website` warns about and allows. For full Mission Creator, sync or build the terrain tree separately on the server at `assets_v2/terrains`, and set both `MAP_ASSETS_DIR` and `GLYPH_ASSETS_DIR` to absolute paths (the unit below does this).

If the server still holds its assets at the pre-relocation `packages/map-assets`, move them once — `cargo xtask deploy website` refuses to deploy until you do, and prints these commands:

```bash
cd /home/sam/tbd/repo
mkdir -p assets_v2/terrains
mv packages/map-assets/everon \
   packages/map-assets/arland \
   packages/map-assets/terrain-registry.json \
   assets_v2/terrains/
```

---

## Phase D — Run the API

```bash
cd /home/sam/tbd/repo/apps/website/api_v2
# Ensure .env is present; migrations run on boot. The CWD matters: the API resolves its
# map-asset defaults relative to it (the unit below sets both explicitly instead).
../../../target/release/api
# Smoke:
curl -sf http://127.0.0.1:8080/healthz
curl -sf http://127.0.0.1:8080/api/v1/health   # if exposed; else /healthz only
```

User systemd unit: [`scripts/deploy/tbd-website-api.service`](../../scripts/deploy/tbd-website-api.service). Substitute `TBD_REPO_DIR_PLACEHOLDER` for your `TBD_REMOTE_DIR` and install it:

```bash
sed "s|TBD_REPO_DIR_PLACEHOLDER|${TBD_REMOTE_DIR#/}|g" \
  scripts/deploy/tbd-website-api.service \
  > ~/.config/systemd/user/tbd-website-api.service
```

It sets `WorkingDirectory` to `apps/website/api_v2` and pins `MAP_ASSETS_DIR` / `GLYPH_ASSETS_DIR`
to absolute paths. Both matter: the API resolves its asset defaults against the working directory,
and `ServeDir` never checks that the root exists, so a wrong CWD serves 404 for every map asset
without logging anything.

It also declares `StateDirectory=tbd-website-api` and points `UPLOAD_DIR` / `MISSION_STAGE_DIR` at
`~/.local/state/tbd-website-api/{uploads,missions}`: everything the API writes lives there, never in
the checkout the deploy rsyncs with `--delete`. Outside development the API refuses to boot unless
both are set to absolute paths. `cargo xtask deploy website` creates the directory and moves any
files an older layout left under `apps/website/api_v2/{uploads,missions}` into it before restarting
the unit; Caddy's `/uploads/*` proxy is unchanged because the API serves that path from wherever
`UPLOAD_DIR` points.

```bash
systemctl --user daemon-reload
systemctl --user enable --now tbd-website-api.service
journalctl --user -u tbd-website-api -f
```

### If the API refuses to boot after a deploy: `migration N was previously applied but has been modified`

`sqlx` hashes each migration file whole, comments included, and compares it against the hash it
recorded when the migration ran. An edit to an applied migration's comments therefore stops every
database that applied it — production included — although the schema is untouched. Do not reset
the database. On the server:

```bash
cd /home/sam/tbd/repo
cargo xtask db repair-migration-checksum --version N --force
```

`--force` is required on the server: the command proves an edit was comments-only by recovering the
applied bytes from git history, and the deploy rsync excludes `.git/`. Verify the edit on a dev
checkout first (`cargo xtask db repair-migration-checksum --version N` there refuses anything but a
comments-only change) and then repoint the server's row. The container name and credentials come
from `TBD_DB_CONTAINER`, `TBD_DB_USER`, `TBD_DB_NAME` (defaults `tbd_reforger_db`, `tbd`,
`tbd_reforger`).

---

## Phase E — SPA reverse proxy + Cloudflare Tunnel

PrairieLearn pattern: **cloudflared** on the host publishes a hostname to a **local** port. TBD should get its own hostname (suggestion: `tbd.icanteam.com`) so PL cookies/OAuth stay isolated.

### E1 — Local reverse proxy (Caddy example)

Until the API serves `frontend/dist`, put Caddy (or nginx) on `127.0.0.1:3080` (any free local port):

```caddy
:3080 {
  # Mission Creator wasm / SharedArrayBuffer parity with Trunk
  header Cross-Origin-Opener-Policy same-origin
  header Cross-Origin-Embedder-Policy credentialless

  handle /api/* {
    reverse_proxy 127.0.0.1:8080
  }
  handle /uploads/* {
    reverse_proxy 127.0.0.1:8080
  }
  handle /healthz {
    reverse_proxy 127.0.0.1:8080
  }
  handle {
    root * /home/sam/tbd/repo/apps/website/frontend/dist
    try_files {path} /index.html
    file_server
  }
}
```

### E2 — Cloudflare Tunnel route

On the server (same cloudflared used for `tenta.icanteam.com`):

1. Cloudflare Zero Trust → Networks → Tunnels → your existing tunnel (or new).
2. Public hostname: `tbd.icanteam.com` → `http://127.0.0.1:3080` (Caddy) **or** directly to API once SPA is nested.
3. DNS: CNAME `tbd` → tunnel.

PL stays on `tenta.icanteam.com` → its existing service (`localhost:3001`).

### E3 — Discord OAuth app

In the Discord developer portal, add redirect:

`https://tbd.icanteam.com/api/v1/auth/discord/callback`

Match `DISCORD_REDIRECT_URL` and `FRONTEND_URL` / `ALLOWED_ORIGINS`.

---

## Phase F — Verification checklist

| # | Check | Pass |
|---|--------|------|
| W1 | SSH | `ssh sam@192.168.0.140` works |
| W2 | Postgres | `pg_isready` inside container |
| W3 | API local | `curl -sf http://127.0.0.1:8080/healthz` → ok |
| W4 | SPA local | `curl -sfI http://127.0.0.1:3080/` → 200 |
| W5 | API via proxy | `curl -sf http://127.0.0.1:3080/api/v1/...` or proxied health |
| W6 | Tunnel | Browser `https://tbd.icanteam.com` loads SPA |
| W7 | CORS/OAuth | Discord login round-trips to `/auth/callback` |
| W8 | Isolation | PL `https://tenta.icanteam.com` still healthy; no writes under `prairielearn/` |
| W9 | Game (optional) | Staging game token hits same API if intended — see STAGING-SERVER |

---

## Ongoing deploy (manual)

```bash
# Dev PC: sync (same rsync as Phase C)
# Server:
cd /home/sam/tbd/repo && cargo xtask mk leptos-build
cd /home/sam/tbd/repo && cargo build --release -p website-api --bin api
systemctl --user restart tbd-website-api.service
# Caddy picks up new dist automatically (static files)
```

Website one-button path: `cargo xtask deploy website` (compose file exists; see `xtask deploy website --help`). Keep the verification table above for manual checks.

---

## Relationship to existing docs

| Doc | Role |
|-----|------|
| This file | **Website** on home server + Cloudflare |
| [`docs/mod/STAGING-SERVER.md`](../mod/STAGING-SERVER.md) | Game server + LAN API smoke |
| [`docs/website/DEV_RUNBOOK.md`](DEV_RUNBOOK.md) | Local laptop stack (`cargo xtask db up/api/web`) |
| [`scripts/deploy/deploy.env.example`](../../scripts/deploy/deploy.env.example) | Shared SSH + paths (`TBD_REMOTE_DIR=/home/sam/tbd/repo`) |
| PL `PrairieLearn.md` / `HandoverContext.md` | SSH + Cloudflare precedent (do not copy PL ports/paths) |

---

## Suggested next code slice (not this doc)

Shipped already: `apps/website/docker-compose.staging.yml` (T-251), `cargo xtask deploy website`, `scripts/deploy/Caddyfile.website`. Remaining:

1. Wire Caddy/SPA + COOP/COEP as the default one-button host path (or Axum `ServeDir` + SPA fallback).
2. Optional Dockerfile for release `api` if you prefer container API over host systemd.
3. Ticket/registry entry only if you want residual SPA hosting tracked as `T-xxx`.

Until SPA static hosting is one-button, **Phases A–F above are enough to get the website reachable** on the home server using the same SSH + Cloudflare pattern as PrairieLearn.
