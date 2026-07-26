# Dev Runbook — spin up the stack

Quick steps to bring up DB + Axum API + Leptos (Trunk) locally. Canonical context: root [`CLAUDE.md`](../../CLAUDE.md).
Backend planning (partially archive): [`docs/website/backend/ROADMAP.md`](backend/ROADMAP.md).
Conventions: [`WHERE_DOES_X_GO.md`](../platform/WHERE_DOES_X_GO.md).

## Start everything

**Toolchain:** Rust stable (API + SPA + tooling). Postgres **18** (`postgres:18-alpine` in `apps/website/api/docker-compose.yml`). Node exists only for `enfusion-mcp` under `scripts/mod` (T-165).

**CI replay:** Primary gate [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml). Local mirror:

```bash
make db-up          # Postgres on host :5434
make ci-local       # editorconfig + website-api + coding-standards + leptos + schema/citations
```

**Formatting:** `make verify-editorconfig` · `cargo fmt --check` in `apps/website/api` + `-p website-frontend`. Coding-standards: `make verify-coding-standards`.

```bash
# 1. Postgres (port 5434)
make db-up

# 2. Axum API on :8080 (CWD = apps/website/api; migrates on boot)
make api

# 3. Leptos Trunk SPA on :3000 (proxies /api + /map-assets → :8080)
#    T-173: make leptos = trunk serve --release (day-to-day / perf-honest).
#    Fast rebuilds only: make leptos-debug (unoptimized wasm — do not judge FPS).
#    T-174: satellite = preview→full progressive by default (sharp TBDS).
#           ?sat=preview = Range-only (gates / fast local); ?sat=full is a no-op.
make leptos
```

Config: `apps/website/api/.env` (`FRONTEND_URL=http://127.0.0.1:3000`). Prod SPA flip: `SPA_DIST_DIR=../frontend/dist`.

## Confirm it's up

```bash
curl -sf http://localhost:8080/api/v1/health
```

- API: http://localhost:8080
- Web: http://127.0.0.1:3000

## Contract codegen, validation & CI (T-123)

```bash
make schema-codegen    # → apps/website/api/src/contract/generated/ (DO NOT hand-edit)
make schema-validate   # packages/tbd-schema goldens
make verify-citations
```

CI jobs: `website-api` + `website-frontend` (renamed from `rust-backend` / `website-leptos` at T-171). Path-filtered supplements: [`contracts.yml`](../../.github/workflows/contracts.yml), [`schema.yml`](../../.github/workflows/schema.yml).

## Log in (no Discord needed)

```
http://localhost:8080/api/v1/auth/dev-login?role=admin
```

Roles: `admin | mission_maker | leader | enlisted`. Requires `APP_ENV=development`.

## Discord OAuth2 — live round-trip (T-207)

dev-login above is the everyday path and needs none of this. This section is for proving the
**real** Discord flow. The implementation is complete — state cookie, constant-time compare,
token exchange, role sync, bounded 429 retry — but **has never been run against live Discord in
this tree**. Finishing it needs a browser and a human at a Discord consent screen; there is no
unattended substitute.

**Credential state** in `apps/website/api/.env` (verified 2026-07-26 — presence and length only,
values never recorded):

| Var | State | Required for |
|-----|-------|--------------|
| `DISCORD_CLIENT_ID` | set, 19-digit numeric snowflake | starting the flow |
| `DISCORD_CLIENT_SECRET` | set, 32 chars (Discord's secret length) | the token exchange |
| `DISCORD_REDIRECT_URL` | set, `http://localhost:8080/api/v1/auth/discord/callback` | both, byte-exact |
| `DISCORD_GUILD_ID` | set, 19-digit numeric snowflake | role sync (blank ⇒ skipped) |
| `DISCORD_BOT_TOKEN` | **empty** | nothing — no consumer reads it (T-279) |
| `DISCORD_WEBHOOK_URL` | **empty** | announcement pushes only, not OAuth |

Root `CLAUDE.md`'s note that *"Real Discord OAuth credentials are blank in `.env`"* is **stale** —
client id, secret and guild id are all populated. What is unproven is whether they are *valid*.

`.env` is gitignored, so a **git worktree has no `.env` at all**. Copy the main checkout's
`apps/website/api/.env` in before `make api`, or the API won't boot (`DATABASE_URL` is required).

### 1. Register the redirect URI

Discord Developer Portal → your application → **OAuth2 → Redirects** → add, byte-for-byte:

```
http://localhost:8080/api/v1/auth/discord/callback
```

It must equal `DISCORD_REDIRECT_URL` exactly. The value is sent **twice** — once as
`redirect_uri` on the authorize URL, again as a form field on the token exchange — and Discord
string-compares it both times. Trailing slash, `127.0.0.1` vs `localhost`, `http` vs `https`, port:
all significant.

Do **not** use the portal's URL generator. The app builds its own authorize URL and requests
`identify guilds.members.read` (`apps/website/api/src/services/discord.rs:15`). `guilds.members.read`
returns only the caller's own membership in one guild, so **no bot and no bot token are involved**.

### 2. Align the host string (do this before you touch a browser)

The `oauth_state` CSRF cookie is set with `Path=/; Max-Age=600; HttpOnly; SameSite=Lax` and **no
`Domain`** — so it is host-only, and **`localhost` and `127.0.0.1` are different cookie hosts**
(ports don't matter, hosts do).

The committed dev config mixes them: `FRONTEND_URL=http://127.0.0.1:3000` but
`DISCORD_REDIRECT_URL=http://localhost:8080/...`. Start the flow from the SPA at `127.0.0.1:3000`
and the cookie is stored for host `127.0.0.1`; Discord then returns you to `localhost:8080`, which
never receives it, and **every login fails `invalid_state`**. Pick one host and use it everywhere:

- **Recommended:** set `FRONTEND_URL=http://localhost:3000` and browse the SPA at
  `http://localhost:3000`. Everything — flow start, callback, final redirect — stays on host
  `localhost`, and the committed `ALLOWED_ORIGINS` already lists that origin.
- Browsing at `localhost:3000` while leaving `FRONTEND_URL` on `127.0.0.1` also *works* (the
  cookie is only needed between login-start and callback), but you finish signed in on
  `127.0.0.1:3000`, not the tab you started in — the session lands at `FRONTEND_URL`.
- Or move everything to `127.0.0.1` — but then `DISCORD_REDIRECT_URL` **and** the portal entry
  must both become `http://127.0.0.1:8080/...`. `http://localhost` is the better-supported dev
  origin on Discord's side; prefer it.

### 3. Pre-flight the secret without a browser

A wrong or rotated secret and a genuine network outage produce the *same* user-visible error, and
the exchange failure is not logged (see the table below). Check the pair directly first — the
client-credentials grant validates id+secret and needs no consent screen:

```bash
cd apps/website/api && set -a && . ./.env && set +a
curl -s -o /dev/null -w '%{http_code}\n' \
  -u "$DISCORD_CLIENT_ID:$DISCORD_CLIENT_SECRET" \
  -d grant_type=client_credentials -d scope=identify \
  https://discord.com/api/oauth2/token
```

`200` = the credential pair is good. `401` (`invalid_client`) = wrong client id/secret; regenerate
the secret in the portal and update `.env`. Never paste the secret on a command line or into a
commit — source it from `.env` as above.

### 4. Run it

```bash
make db-up && make seed     # role mappings must exist — see §5
make api                    # :8080
make leptos                 # :3000
```

Then in a browser: open `/login` on your chosen host (e.g. `http://localhost:3000/login`), click
**Sign in with Discord**, approve at Discord's consent screen, and land back on the SPA. Start
from the SPA, not from `:8080` directly — the state cookie must be set on the host you return to
(§2).

**Success** is a 302 to `{FRONTEND_URL}/auth/callback` with the session in the URL **fragment**
(`#access_token=…&arma_linked=…&expires_at=…&refresh_token=…`, keys sorted), the SPA signed in,
and your Discord display name and avatar in the top bar. Errors arrive the same way —
`#error=<reason>` — which the SPA renders as human copy (`apps/website/frontend/src/auth.rs`).

Confirm it server-side rather than trusting the UI:

```sql
-- the upserted profile + resolved tier
SELECT discord_id, username, discord_handle, role, last_login_at
  FROM users ORDER BY last_login_at DESC LIMIT 5;

-- the authoritative role snapshot Discord returned
SELECT discord_role_id, synced_at FROM user_discord_roles WHERE discord_id = '<your-id>';

-- one auth.login INFO row, and NO auth.role_sync_skipped WARN row next to it
SELECT created_at, severity, action, message
  FROM audit_logs WHERE action LIKE 'auth.%' ORDER BY id DESC LIMIT 10;
```

### 5. Map guild roles to web tiers

A flawless login still resolves to **enlisted** unless the guild's role snowflakes are mapped.
`make seed` applies `apps/website/api/seeds/discord_roles.sql`, whose ids are specific to the TBD
guild — and one (`1517290000000000000`, "Squad Leader") is an explicit placeholder. `resolve_role`
takes the `mapped_role` of the highest-`priority` matching row and falls back to enlisted when
nothing matches.

The practical order is: log in once, read the real snowflakes back out of `user_discord_roles`
(they are stored even when unmapped, precisely for this), map them, then re-resolve **everyone**
without making them log in again:

```bash
# admin JWT — in dev, read access_token out of the dev-login redirect fragment
curl -X POST http://localhost:8080/api/v1/admin/roles/sync \
  -H "Authorization: Bearer $ADMIN_JWT"
```

`resolve_role` returns enlisted for an empty snapshot, so a user who is genuinely in no mapped
role stays enlisted by design — that is not the same failure as an unmapped snowflake.

### 6. Telling the failure modes apart

| Symptom | Cause | Fix |
|---------|-------|-----|
| Discord's own *"Invalid OAuth2 redirect_uri"* page; you never return to the app | `DISCORD_REDIRECT_URL` isn't registered, or doesn't match byte-for-byte | §1 — no app-side error exists for this, because the request never reached the app |
| `#error=oauth_unconfigured` | `DISCORD_CLIENT_ID` blank | fill it; the app refuses to send you to Discord with an empty client id |
| `#error=invalid_state` | cookie host mismatch (§2), >10 min at the consent screen (`Max-Age=600`), or cookies blocked | §2 first — it is the overwhelmingly likely cause on a fresh setup |
| `#error=discord_unreachable` | bad/rotated secret **or** a real network failure **or** a redirect_uri mismatch caught at exchange time | §3 discriminates. **The exchange error is not logged** — `exchange_code`/`fetch_user` failures are mapped to this one reason and the cause is dropped, so the curl is your only signal |
| `#error=missing_code` | consent was denied, or the callback URL was hand-built | retry and approve |
| `#error=banned` | `users.is_banned` is true for that Discord id | clear the ban in admin |
| `#error=server_error` | a DB write failed — user upsert, role sync, user reload, or session issue | also silent: the callback's `let-else` arms drop the sqlx error. Confirm Postgres is up and migrated (`make db-up`, restart `make api`), then retry |
| Login succeeds, but you are `enlisted` | (a) role snowflakes unmapped, (b) you aren't in the guild, (c) `DISCORD_GUILD_ID` blank | (a) is the common one → §5. (b) is a **legitimate answer**: Discord 404s the member lookup, that 404 means "not a member", and demoting is correct. (c) leaves a WARN audit row — see below |

### 7. Two live hazards, fixed in code — what you'd see now

Both used to destroy an admin's tier permanently, because `sync_roles` DELETEs every stored
`user_discord_roles` row before re-inserting, and `resync_all_roles` rebuilds from that same table
— once the snapshot was gone there was nothing left to restore from.

- **A transient Discord failure demoted an admin to enlisted, irrecoverably.** Fixed:
  `RoleSnapshot::Unavailable` never mutates roles. Today a timeout or 5xx logs
  *"discord guild-member lookup failed — keeping the stored role snapshot"* and writes nothing.
- **A 200 whose body omits `roles` did the same** — a proxy or gateway serving a JSON error
  envelope with a 200 status decoded to `roles: []`, which reads as "Discord says this user holds
  no roles". Fixed: `roles` is no longer `#[serde(default)]`, so such a body fails to decode and
  travels the same write-nothing path. An explicit `"roles": []` is still a real answer and still
  demotes, correctly.

A blank `DISCORD_GUILD_ID` lands on that same path (it used to enlist the whole community, one
login at a time, without a single log line).

**How you see it:** an `auth.role_sync_skipped` **WARN** audit row alongside the `auth.login`
row. That means the login was *degraded* — the tier you see is the **stored** one, not a fresh
answer from Discord. One is a blip. Two in a row for the same user is a real Discord, proxy, or
config problem, and roles are frozen until it clears.

### 8. Known unfixed: admin role edits are clobbered at the next successful login

`PATCH /api/v1/admin/users/{discord_id}` writes `users.role` directly. Discord is the source of
truth and **nothing records that an override happened**, so the next *successful* login recomputes
the role from the guild snapshot and silently overwrites the edit. You will hit this.

The perverse part: an override survives exactly as long as Discord stays unreachable (that path
writes nothing), and dies the moment Discord recovers.

The only durable fix today is to make Discord agree — map the guild role in `discord_roles` and
`POST /admin/roles/sync` (§5). Use the PATCH for temporary, same-session changes only.

## Stop

```bash
make db-down      # stops Postgres, keeps volume
# API + trunk: kill the background processes
```

## Postgres 18 upgrade (T-124)

If `make api` fails migrations after pulling T-124, the local volume may still be Postgres **16** data. Re-init:

```bash
make db-down
# podman volume rm tbd-reforger_db_data   # or docker — inspect compose project name
make db-up && make seed
```

Dev data is reseedable; mock missions are optional (see below).

## Registry catalog (T-068 / T-150 / T-068.9)

**Dev seed** (`make seed` → `apps/website/api/seeds/registry_dev.sql`) is the thin 21-row smoke set.

**Full catalog** (Workbench universal export): **1,880 items** + **4,012 compat edges**.

```bash
# From repo root — upserts both committed envelopes into the dev DB (idempotent)
make registry-import

# Or explicit paths / prune:
# cargo run --bin import-registry --manifest-path apps/website/api/Cargo.toml -- \
#   --items packages/tbd-schema/registry/registry-items.workbench.json \
#   --compat packages/tbd-schema/registry/registry-compat.workbench.json \
#   [--modpack <uuid>] [--prune]
```

Restart `make api` after handler changes — `cargo run` does not hot-reload.

| Route | Auth | Notes |
|-------|------|--------|
| `GET /api/v1/registry` | mission_maker+ JWT | Items; weak ETag / 304 |
| `GET /api/v1/registry/compat` | mission_maker+ JWT | Edges; `?edge_type=` filter; ETag |

**Mod compiled mission (T-092.2):**

```bash
# Requires SERVICE_TOKEN in apps/website/api/.env
curl -sS -H "X-Service-Token: $SERVICE_TOKEN" \
  http://localhost:8080/api/v1/missions/{mission_id}/compiled | jq .schemaVersion
```

## Map assets (T-090 / T-091 / T-171)

Corpus: `packages/map-assets/` — Everon ~1.3 GB on disk; **tracked in LFS = exactly 2 objects**:

| Object | Size | Purpose |
|--------|------|---------|
| `everon/dem/everon-dem-16bit.png` | ~72 MB | DEM / hillshade / map-engine tests |
| `everon/satellite/everon-sat.tbd-sat` | ~153 MB | Unified satellite basemap |

`**/staging/` + `**/tiles/` are gitignored (rebuildable via `make map-*`). `.gitattributes` LFS patterns: `packages/map-assets/**/*.{png,r16,tbd-sat}`.

| Consumer | Needs | Mechanism |
|----------|-------|-----------|
| CI `map-engine` job | DEM only | `git lfs pull --include …/everon-dem-16bit.png` |
| CI other jobs | none | sat deliberately never dragged |
| Local dev editor | DEM + sat | Axum `ServeDir` `/map-assets` (`MAP_ASSETS_DIR`, default `../../../packages/map-assets` from `api/` CWD) ← Trunk proxy ← SPA `fetch("/map-assets/…")` |
| Gate harness | dist + optional map-assets | `gate serve --map-assets` |
| Clone without LFS | degraded | manifest/JSON/chunks plain-git; DEM/sat 404 → no sat/hillshade |

**Convenience targets:**

```bash
make lfs-dem   # ~72 MB — enough for map-engine tests + hillshade
make lfs-sat   # ~153 MB — full satellite bundle
# or: git lfs install && git lfs pull
```

Each terrain has a `manifest.json` validated against [`terrain-manifest.schema.json`](../../packages/tbd-schema/schema/terrain-manifest.schema.json).

**Tile pyramid (optional):** not in git. Rebuild:

```bash
make map-water-everon
make map-cartographic-everon
make map-cartographic-verify
```

**Mission Settings → Map basemap (T-173):** the Satellite/Map radio is live. **Map** view needs the cartographic tile pyramid from `make map-cartographic-everon`; when those tiles are absent the host **falls back to satellite** (not a broken toggle).

**Satellite load (T-174):** day-to-day `make leptos` upgrades preview→full TBDS automatically (no `?sat=full`). Use `?sat=preview` only for Range-only / fast iteration (same as CI gates). Density-heatmap green glow is removed.

**Forest canopy (T-176):** island forest highlight is **8 m TBDD canopy mass** (not the old 32 m Path B landcover forest wash). Clearings stay open. Retune tightness: `CANOPY_KERNEL_RADIUS_CELLS` / `CANOPY_MASS_ISO`, then `cargo run -p tbd-tools --bin world -- redensify --terrain everon` (committed-chunk path; no Workbench).

See [`packages/map-assets/README.md`](../../packages/map-assets/README.md). **Ops:** ImageMagick spill → `/var/tmp`.

**Verify:**

```bash
make verify-terrain
make verify-terrain-strict
```

**Frontend/engine tests:** `cargo test -p website-frontend` + `cargo test -p map-engine-core --all-features` (DEM peaks need `make lfs-dem` or `git lfs pull`).

## Notes

- A fresh DB only has Discord role mappings + registry smoke rows (`make seed` → `apps/website/api/seeds/`).
- Frontend: `make ci-local-leptos`; full editor gates: `make leptos-gates` (see [`EDITOR_GATE_RUNBOOK.md`](EDITOR_GATE_RUNBOOK.md) — `gate doctor` preflight, full Chrome `--headless=new`, toolchain **1.95.0**).
- Integration tests: `make test-it` (needs `make db-up`).

## Mock data (optional, not run by `make seed`)

`apps/website/api/seeds/mock_data.sql` (Operation Red Dawn etc.) is **manual psql only** — the Go `cmd/seed` applier was deleted at T-145. Example:

```bash
podman exec -i tbd_reforger_db psql -U tbd -d tbd_reforger < \
  apps/website/api/seeds/mock_data.sql
```

To purge those four fixed-UUID missions (children first; no ON DELETE CASCADE):

```bash
docker compose -f apps/website/api/docker-compose.yml exec -T db psql -U tbd -d tbd_reforger <<'SQL'
DELETE FROM mission_versions  WHERE mission_id IN ('00000000-0000-4000-c000-000000000001','00000000-0000-4000-c000-000000000002','00000000-0000-4000-c000-000000000003','00000000-0000-4000-c000-000000000004');
DELETE FROM mission_armories  WHERE mission_id IN ('00000000-0000-4000-c000-000000000001','00000000-0000-4000-c000-000000000002','00000000-0000-4000-c000-000000000003','00000000-0000-4000-c000-000000000004');
DELETE FROM mission_bookmarks WHERE mission_id IN ('00000000-0000-4000-c000-000000000001','00000000-0000-4000-c000-000000000002','00000000-0000-4000-c000-000000000003','00000000-0000-4000-c000-000000000004');
UPDATE missions SET current_version_id = NULL WHERE id IN ('00000000-0000-4000-c000-000000000001','00000000-0000-4000-c000-000000000002','00000000-0000-4000-c000-000000000003','00000000-0000-4000-c000-000000000004');
DELETE FROM missions WHERE id IN ('00000000-0000-4000-c000-000000000001','00000000-0000-4000-c000-000000000002','00000000-0000-4000-c000-000000000003','00000000-0000-4000-c000-000000000004');
SQL
```
