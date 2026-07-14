# Home-server website setup — LAN (`dooley`)

Deploy the **TBD Reforger website** (Rust API + React SPA) on the same home server used for PrairieLearn and the Reforger game staging stack.

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
| `/home/sam/tbd/repo/apps/website/.env` | **Server-only** secrets (never rsync from dev) |
| `/home/sam/tbd/repo/apps/website/frontend/dist` | Built SPA |
| `/home/sam/tbd/website-data/postgres` | Optional named volume / bind for DB (if not compose default) |
| `/home/sam/prairielearn/` | **Forbidden** for TBD |

Create layout:

```bash
ssh sam@192.168.0.140 'mkdir -p /home/sam/tbd/{repo,profile,addons-staging,website-data}'
```

---

## Honest gaps (as of 2026-07-10)

The **game** staging path assumes `apps/website/docker-compose.staging.yml`, but that file is **not in the repo today**. Local compose only starts Postgres (`apps/website/docker-compose.yml`). The API is Rust (`apps/website`, `cargo run --bin api`); it serves `/api/v1` + `/uploads` but **does not** serve the SPA `frontend/dist` yet.

Until Claude Code / a follow-up slice adds compose + static hosting, use this doc’s **manual phase** (Postgres Docker + release `api` binary + Caddy/nginx for SPA + Cloudflare). Mark each checkbox when you ship the automation.

| Gap | Needed for “one command” website host |
|-----|----------------------------------------|
| `docker-compose.staging.yml` (or `website.yml`) | Postgres + optional API container |
| Dockerfile for API (Rust) | Reproducible server image |
| Serve SPA from API **or** Caddy/nginx | `/` → `index.html`, `/api` → Axum |
| COOP/COEP headers on SPA | Same as Vite (`same-origin` + `credentialless`) — required for map wasm / SAB |
| `deploy-website.sh` | Rsync + build + restart (game `deploy-staging.sh` is game-focused) |

---

## One-time prerequisites

### Dev PC

```bash
which sshpass rsync ssh curl git node cargo docker
node -v          # 26+ (repo .nvmrc)
rustc --version  # toolchain per apps/website/rust-toolchain.toml
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
# On server, inside /home/sam/tbd/repo/apps/website after first sync
# Edit docker-compose.yml ports to "127.0.0.1:5432:5432" OR use a dedicated staging compose when added
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
cd /home/sam/tbd/repo/apps/website
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
- For a first LAN-only smoke you may temporarily use `APP_ENV=development` + `FRONTEND_URL=http://192.168.0.140:5173` — do **not** leave that on a public tunnel.
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
  --exclude 'apps/website/.env' \
  --exclude 'target' \
  --exclude 'packages/map-assets' \
  ./ "${TBD_SSH_HOST}:${TBD_REMOTE_DIR}/"
```

Then on the **server**:

```bash
cd /home/sam/tbd/repo
# Toolchain: install Rust + Node 26 if missing (nvm or nodesource), then:
cd apps/website/frontend && npm ci && npm run build
cd /home/sam/tbd/repo/apps/website
cargo build --release --bin api
```

Map assets: Mission Creator satellite/DEM bundles are large LFS. For a **library-only** site you can skip `packages/map-assets` initially; for full Mission Creator, sync or build assets separately and run `make map-assets-link` on the server.

---

## Phase D — Run the API

```bash
cd /home/sam/tbd/repo/apps/website
# Ensure .env is present; migrations run on boot
./target/release/api
# Smoke:
curl -sf http://127.0.0.1:8080/healthz
curl -sf http://127.0.0.1:8080/api/v1/health   # if exposed; else /healthz only
```

User systemd unit sketch (`~/.config/systemd/user/tbd-website-api.service`):

```ini
[Unit]
Description=TBD Reforger website API
After=network-online.target docker.service
Wants=network-online.target

[Service]
Type=simple
WorkingDirectory=/home/sam/tbd/repo/apps/website
EnvironmentFile=/home/sam/tbd/repo/apps/website/.env
ExecStart=/home/sam/tbd/repo/apps/website/target/release/api
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
```

```bash
systemctl --user daemon-reload
systemctl --user enable --now tbd-website-api.service
journalctl --user -u tbd-website-api -f
```

---

## Phase E — SPA reverse proxy + Cloudflare Tunnel

PrairieLearn pattern: **cloudflared** on the host publishes a hostname to a **local** port. TBD should get its own hostname (suggestion: `tbd.icanteam.com`) so PL cookies/OAuth stay isolated.

### E1 — Local reverse proxy (Caddy example)

Until the API serves `frontend/dist`, put Caddy (or nginx) on `127.0.0.1:3080` (any free local port):

```caddy
:3080 {
  # Mission Creator wasm / SharedArrayBuffer parity with Vite
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
cd /home/sam/tbd/repo/apps/website/frontend && npm ci && npm run build
cd /home/sam/tbd/repo/apps/website && cargo build --release --bin api
systemctl --user restart tbd-website-api.service
# Caddy picks up new dist automatically (static files)
```

When `docker-compose.staging.yml` + `deploy-website.sh` exist, replace this section with those commands and keep the verification table.

---

## Relationship to existing docs

| Doc | Role |
|-----|------|
| This file | **Website** on home server + Cloudflare |
| [`docs/mod/STAGING-SERVER.md`](../mod/STAGING-SERVER.md) | Game server + LAN API smoke |
| [`docs/website/DEV_RUNBOOK.md`](DEV_RUNBOOK.md) | Local laptop stack (`make db-up/api/web`) |
| [`scripts/deploy/deploy.env.example`](../../scripts/deploy/deploy.env.example) | Shared SSH + paths (`TBD_REMOTE_DIR=/home/sam/tbd/repo`) |
| PL `PrairieLearn.md` / `HandoverContext.md` | SSH + Cloudflare precedent (do not copy PL ports/paths) |

---

## Suggested next code slice (Claude Code — not this doc)

1. Add `apps/website/docker-compose.staging.yml` (postgres + optional api).
2. Add `Dockerfile` for release `api`.
3. Nest `ServeDir` for `frontend/dist` + SPA fallback + COOP/COEP in Axum **or** ship Caddyfile in `scripts/deploy/`.
4. Add `scripts/website/deploy-website.sh` (rsync + remote build/restart), refuse any `prairielearn` path.
5. Ticket/registry entry when you want this tracked as `T-xxx`.

Until those land, **Phases A–F above are enough to get the website reachable** on the home server using the same SSH + Cloudflare pattern as PrairieLearn.
