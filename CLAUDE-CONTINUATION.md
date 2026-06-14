# TBD Reforger Platform — Claude CLI Continuation Guide

> **Purpose:** Live handoff document for Claude CLI (or any fresh session) to continue TBD Reforger platform work without re-deriving context from Cursor chat history. **Keep this file updated** as decisions change.
>
> **Last updated:** 2026-06-14 · **Workspace:** `/home/Samuel/Projects/Arma reforger`  
> **Phase:** 1 · **GitHub:** `darkforce09/tbd-reforger-platform` · **Branch:** `main` only

---

## How to use this file

1. **Claude Code:** start with [`CLAUDE-CODE-START.md`](CLAUDE-CODE-START.md) (Workbench + MCP).
2. Read this entire file for full context.
2. Read [`tbd-reforger-platform-build-plan.md`](tbd-reforger-platform-build-plan.md) for the full original vision (Mission JSON schema outline, workstream details, authoring tiers).
3. Apply the **overrides in section 2** — they supersede the master plan where they conflict.
4. Enable **Enfusion MCP** before writing any Enfusion script (section 4).
5. Continue from **section 7 — Current focus** unless the user directs otherwise.

**Claude CLI with MCP:**

```bash
claude mcp add --scope user enfusion-mcp -- npx -y enfusion-mcp
cd "/home/Samuel/Projects/Arma reforger"
claude "Read CLAUDE-CONTINUATION.md and continue from section 7 — Current focus"
```

**Cursor:** add `.cursor/mcp.json` with the same server (see section 4).

---

## 1. Project summary

TBD Event is building a **data-driven Arma Reforger event platform**:

- **Missions are JSON documents** fetched by the game server at load — not Workbench-built mods per mission.
- **One greenfield Enfusion mod (TBD-Framework)** runs all missions on generic per-terrain scenarios.
- **Web platform** (`Tbdevent_Website/`) handles Discord auth, events, slotting, mission validation, and game-server API.
- **Custom TFAR-like VOIP** — external Teamspeak-like client + voice server + game bridge mod. **Partner-owned**, parallel track.
- **No payments** — Stripe, entitlements, and supporter tiers are out of scope.

---

## 2. Decisions that override the master plan

| Topic | Decision |
|---|---|
| Framework | **Greenfield TBD-Framework** — NOT continuing CRF fork as primary path |
| `Tbd_framework/` | Reference only (Coalition fork). Do not rebrand/prune unless explicitly asked |
| Payments | **Removed** — no Stripe, no `/api/entitlements`, no monetization features |
| VOIP | **External TFAR-like app** — NOT in-engine TBD-VON enhancement |
| VOIP owner | **Partner** — main team does NOT build voice client/server |
| Web stack | **Go + React** (embedded binary), NOT Next.js — already built |
| Enfusion APIs | **Never guess** — verify via Enfusion MCP `api_search` / `wiki_read` |

---

## 3. Team split

| Workstream | Owner | Repo |
|---|---|---|
| A — TBD-Framework | Main team (Samuel) | `tbd-framework/` (exists) |
| B — Web platform | Main team | `Tbdevent_Website/` (exists) |
| C — TBD Voice | **Partner** | `tbd-voip` (partner) |
| D — Content/registry | Main team first | `tbd-content` (future) |
| Schema contract | Main team | `tbd-schema/` (exists) |

**Integration boundary:** Shared **bridge contract** in `tbd-schema` — how Mission JSON `radioPlan` maps to voice nets, and game↔client messages (`OnSpawn`, `OnDeath`, `OnNetChange`, `OnPTT`). Main team exposes hooks in framework; partner implements bridge + voice stack.

**Milestone #1 does NOT require VOIP.** In-game VON is acceptable as temporary fallback.

---

## 4. Enfusion MCP — mandatory for game code

Install (Linux/macOS):

```bash
claude mcp add --scope user enfusion-mcp -- npx -y enfusion-mcp
```

Cursor project config — create `.cursor/mcp.json` (include env vars for parity with `.mcp.json`):

```json
{
  "mcpServers": {
    "enfusion-mcp": {
      "command": "npx",
      "args": ["-y", "enfusion-mcp"],
      "env": {
        "ENFUSION_GAME_PATH": "/path/to/Arma Reforger",
        "ENFUSION_WORKBENCH_PATH": "/path/to/Arma Reforger Tools",
        "ENFUSION_PROJECT_PATH": "/home/Samuel/Projects/Arma reforger/tbd-framework"
      }
    }
  }
}
```

**Workbench test loop** (no `wb_log` tool in enfusion-mcp):

```bash
bash scripts/mcp-call.sh wb_connect '{}'
bash scripts/mcp-call.sh wb_play '{}'
sleep 25
bash scripts/mcp-wb-logs.sh
bash scripts/mcp-call.sh wb_stop '{}'
```

Or: `bash scripts/tbd-spawn-verify.sh`

**Before writing any `.c` file:**

1. `api_search` — confirm class/method names for current game version
2. `wiki_read` — read relevant BI/Enfusion wiki pages
3. `game_read` — inspect base-game reference implementations
4. `script_create` / `mod_create` — scaffold with correct structure

Key tools: `api_search`, `component_search`, `wiki_search`, `wiki_read`, `game_read`, `mod_create`, `script_create`, `mod_validate`, `mod_build`, `server_config`, `wb_launch`.

---

## 5. Repository map

```
/home/Samuel/Projects/Arma reforger/
├── README.md                             # Project overview + quick start
├── tbd-reforger-platform-build-plan.md   # Original master plan (reference)
├── CLAUDE-CODE-START.md                  # Claude Code entry (read first)
├── CLAUDE-CONTINUATION.md                # This file (live — keep updated)
├── MILESTONES.md                         # Milestone #1/#2 dates + criteria
├── docs/                                 # Ops copy (Discord post, etc.)
├── scripts/                              # Workbench, server profile, dev server, staging, MCP, tests
│   ├── setup-workbench-linux.sh
│   ├── setup-server-profile.sh
│   ├── run-dev-server.sh
│   ├── deploy-staging.sh                   # rsync → 192.168.0.140
│   ├── remote-log-grep.sh
│   ├── bootstrap-staging-server.sh
│   ├── mcp-call.sh / mcp-wb-logs.sh / tbd-spawn-verify.sh / tbd-dev-bootstrap.sh
│   ├── setup-mcp-game-root.sh
│   ├── tbd-dev-server.config.json        # For Workshop/config hosting (not local -addons)
│   ├── test-phase1-api.sh
│   └── manual-test.sh
├── .local-test-profile/                  # Default dedicated-server profile (gitignored)
├── .github/workflows/schema.yml          # tbd-schema compatibility CI
├── tbd-schema/                           # Schema, golden missions, bridge contract
│   ├── VERSION                           # 1.1.0
│   ├── schema/mission.schema.json
│   ├── schema/registry.schema.json
│   ├── golden-missions/
│   ├── registry/registry.vanilla-poc.json
│   ├── bridge/
│   └── spikes/                           # REST 0.1, registry 0.4, VOIP 0.2 brief
├── tbd-framework/                          # Production Enfusion mod
│   ├── addon.gproj                         # GUID B2C3D4E5F6A78901
│   ├── Missions/TBD_Dev_POC.conf
│   ├── worlds/TBD_Dev_POC.ent            # Eden subscene
│   ├── Prefabs/Systems/TBD_GameMode.et
│   ├── Scripts/Game/TBD/
│   └── Data/registry.json, backend.example.json
├── Tbdevent_Website/                     # Go + React — live foundation
│   ├── cmd/server/main.go
│   ├── internal/handlers/                  # pages, events, auth, gameserver, missions
│   ├── internal/migrate/migrations/      # 00001–00004
│   ├── missions/                         # compiled JSON for game API
│   └── web/src/
└── Tbd_framework/                        # CRF fork — REFERENCE ONLY (see REFERENCE-ONLY.md)
```

Partner repo (external): `tbd-voip` — voice client, server, game bridge.

---

## 6. What is already built

### Website (`Tbdevent_Website/`)

**Foundation (pre–Phase 0):**

- Discord OAuth + admin CMS (rules, compliance, server, mods pages)
- Event hub: events, announcements, dashboard, sign-up with capacity/waitlist
- Admin tabs: Content, Events, Announcements, Registrations

**Phase 0 — game-server API:**

- Server-token auth (`internal/middleware/servertoken.go`)
- `GET /api/missions/{id}/compiled` (disk, then DB when published)
- `POST /api/results`, `POST /api/telemetry` (log-only for now)
- REST spike harness (`cmd/restspike`) — GREEN

**Phase 1 — backend (done; UI pending):**

- `POST /api/missions` (admin, schema-validated upload)
- `POST /api/link` (server token — register 6-digit code)
- `POST /api/me/link` (user consumes code)
- `GET /api/game/events/{id}/roster` (server token — identityId → slotId)
- `GET /api/admin/events/{id}/slots`, `PUT /api/admin/events/{id}/slots/{slotId}` (manual ORBAT)
- Migration `00004_missions_orbat_identity.sql`

**Migrations:** `00001`–`00004`

### Schema (`tbd-schema/`)

- Mission JSON Schema **1.1** (`slots[]` required) + registry JSON Schema v1.0.0 (`VERSION`, `CHANGELOG.md`)
- Two golden missions + `npm run validate` (CI in `.github/workflows/schema.yml`)
- `scripts/validate-file.mjs`, `scripts/flatten-orbat-slots.mjs`
- Registry POC file, VOIP bridge contract + samples
- Spikes: REST 0.1 GREEN, registry 0.4 GREEN, VOIP 0.2 brief (partner)

### Framework (`tbd-framework/`)

- `TBD_BackendConfig.c`, `TBD_MissionLoader.c` (REST + profile fallback)
- `TBD_Registry.c` (alias resolution)
- `TBD_SpawnManager.c`, modded `SCR_MenuSpawnLogic` — per-slot spawn from mission `slots[]`
- `TBD_RosterLoader.c` — roster API poll
- `TBD_FrameworkManager.c`, `TBD_GameStage.c`, radio bridge stubs
- `TBD_GameMode.et` prefab (manager + RplComponent; **no** Registry POC on game mode)
- Dev scenario: Eden subscene + `Missions/TBD_Dev_POC.conf`
- **Workbench verified (2026-06-14):** 18× built slot spawn, assigned slot, spawn requested

### Ops scripts

- `setup-workbench-linux.sh`, `setup-server-profile.sh`, `run-dev-server.sh`
- `deploy-staging.sh`, `remote-log-grep.sh`, `bootstrap-staging-server.sh`
- `setup-client-addons.sh`, `debug-direct-join.sh`
- `mcp-call.sh`, `mcp-wb-logs.sh`, `tbd-spawn-verify.sh`, `tbd-dev-bootstrap.sh`, `setup-mcp-game-root.sh`
- `test-phase1-api.sh`, `manual-test.sh`, `seed-milestone-announcement.sh`

### Not done yet

- Framework: full ORBAT enforcement (identity linking), capture objective, admin commands, results POST wiring
- Web: mission upload UI, slot assignment UI, results on event page
- Staging LAN join on 192.168.0.140 — server deploy ✓, **client Direct Join still under debug** (see section 10)

---

## 7. Current focus — start here

**→ Claude Code:** read [`CLAUDE-CODE-START.md`](CLAUDE-CODE-START.md) first. Use a **new chat** for Phase 1 gameplay work.

### Phase 1 priority (toward Milestone #1 — Sat 2026-08-22)

Build in this order:

1. **Staging LAN join** — unblock Direct Join `192.168.0.140:2001` (see section 10); then confirm slot spawn from client
2. **Capture objective** — win condition for internal test
3. **Full ORBAT enforcement** — identity linking + roster slot assignment (round-robin spawn works today)
4. **Stage machine + admin commands** — `#stage next`, safe start, boundary
5. **Web admin UI** — mission upload, slot assignment, link-code UX
6. **Results persistence** — wire `POST /api/results` to DB + event page

### Dedicated server (local dev)

```bash
bash scripts/setup-server-profile.sh
bash scripts/run-dev-server.sh
```

Uses `-server {69A85365FC09E2CA}Missions/TBD_Dev_POC.conf` + `-addons B2C3D4E5F6A78901`.  
**Do not** combine `-config` with `-addons` for unpublished local mods.

Profile files live under `.local-test-profile/profile/` (Enfusion `$profile:` root).

### Workbench (when editing scripts)

1. Sync game + Tools Steam builds
2. `bash scripts/setup-workbench-linux.sh`
3. Open only `tbd-framework/addon.gproj`
4. enfusion-mcp before any `.c` edit

### Partner track (parallel)

Phase 0.2 VOIP spike — [`tbd-schema/spikes/voip-spike-brief.md`](tbd-schema/spikes/voip-spike-brief.md)

### Milestone #1 gate (see [`MILESTONES.md`](MILESTONES.md))

| Criterion | Status |
|---|---|
| Mission loads from backend | ✓ |
| File fallback | ✓ |
| Per-slot spawn (round-robin) | ✓ Workbench 2026-06-14 |
| Slots enforce (roster identity) | ✗ |
| Side wins | ✗ |
| Results on event page | ✗ |
| No Workbench to play | ✗ (blocked on staging LAN + capture) |

---

## 8. Mission JSON — the contract

The schema is the most expensive artifact. Consumed by: web validator, Enfusion loader, ORBAT slotting, radio plan, partner VOIP bridge.

Rules:

- Published missions are **immutable**; edits = new version (content-hash ID)
- Wizard emits **registry aliases** (`kit:us_rifleman`, `comp:checkpoint_small`) — never raw prefab GUIDs
- Include `radioPlan.nets[]` with `id`, `label`, `freqMHz`, `faction`
- Schema lives in `tbd-schema/` as formal JSON Schema + golden mission compatibility tests

See master plan section 2 for full schema outline example.

---

## 9. Game server API

| Endpoint | Auth | Status | Purpose |
|---|---|---|---|
| `GET /api/missions/{id}/compiled` | server token | ✓ | Mission JSON for loader |
| `GET /api/game/events/{id}/roster` | server token | ✓ | identityId → slotId map |
| `POST /api/link` | server token | ✓ | Bind 6-digit code → game identity |
| `POST /api/me/link` | user session | ✓ | User consumes link code |
| `POST /api/missions` | admin | ✓ | Mission publish (schema-validated) |
| `PUT /api/admin/events/{id}/slots/{slotId}` | admin | ✓ | Manual ORBAT assignment |
| `POST /api/telemetry` | server token | ✓ (log-only) | Batched gameplay events |
| `POST /api/results` | server token | ✓ (log-only) | Final mission results |
| `GET /api/servers/{id}/commands` | server token | ✗ | Queued admin actions (poll) |
| `POST /api/events/{id}/slots/{slotId}/claim` | user session | ✗ | ORBAT slot claim UI |

~~`GET /api/entitlements/{identityId}`~~ — removed (no payments).

---

## 10. Staging server (192.168.0.140)

Phase A: rsync repo + local mod symlink — no Workshop yet. **Never** deploy under `/home/sam/prairielearn/`.

| Path | Value |
|------|-------|
| Host | `sam@192.168.0.140` |
| Repo | `/home/sam/tbd/repo` |
| Profile | `/home/sam/tbd/profile` |
| Addons staging | `/home/sam/tbd/addons-staging` |
| Game port | `2001` UDP/TCP — **A2S must be on 2001** for Direct Join (`-a2sPort 2001`) |
| API | `127.0.0.1:8080` (Docker on server) |
| Steam app | **1874900** (stable Arma Reforger Server) — not 1890870 Experimental |

```bash
cp scripts/deploy.env.example scripts/deploy.env
bash scripts/bootstrap-staging-server.sh    # one-time discovery
bash scripts/deploy-staging.sh
bash scripts/remote-log-grep.sh
bash scripts/debug-direct-join.sh           # LAN join diagnostics
```

Full guide: [`docs/STAGING-SERVER.md`](docs/STAGING-SERVER.md).

**Client join:** `bash scripts/setup-client-addons.sh` → Steam launch options with `-addonsDir` + `-addons B2C3D4E5F6A78901` → Direct Join `192.168.0.140:2001`.

### LAN join debug status (2026-06-14)

| Check | Status |
|-------|--------|
| Server `tbd-reforger.service` active, mission loaded, 18× slot spawn | ✓ |
| API + roster smoke on server | ✓ |
| Client mod loads (Proton log shows `B2C3D4E5F6A78901`) | ✓ |
| Ping PC → server (WiFi vs LAN same `/24`) | ✓ — not an isolation issue |
| A2S on `:2001` from dev PC | ✓ after `-a2sPort 2001` fix |
| Client Direct Join end-to-end | **pending user retest** |
| Steam build match (client vs server `buildid`) | **may differ** — sync via `steamcmd +app_update 1874900 validate` |

**Known fix applied:** `-a2sPort 17777` caused **"No server found"** — Direct Join probes game port 2001, not 17777. Unit now uses `-bindIP 0.0.0.0 -a2sPort 2001`.

**If join still fails after retest:** compare Steam `buildid`, watch server `console.log` during join for connect lines, grep client Proton log for `SERVER_NOT_FOUND` vs version mismatch.

---

## 11. What NOT to do

- Do NOT build Stripe, payments, or entitlements
- Do NOT build in-engine TBD-VON — partner builds external TFAR-like stack
- Do NOT resume CRF fork (mod-list pruning, CVON strip, TBD branding) as primary path
- Do NOT guess Enfusion class names — use Enfusion MCP
- Do NOT start mission wizard before schema freeze (schema is frozen at v1.0.0)
- Do NOT block Milestone #1 on VOIP

---

## 12. Coding conventions

**Website (Go):** chi router, pgx, goose migrations, scs sessions. Match existing patterns in `Tbdevent_Website/internal/`.

**Enfusion:** Original TBD-owned code. Clean-room — may read CRF/BI samples for patterns, never copy verbatim. Verify all APIs via MCP.

**Commits:** Only when user explicitly asks.

**This file:** Update section 7 (Current focus) and section 6 (built/not done) whenever a milestone completes or priorities shift.

---

## 13. Key reference files

| File | Why |
|---|---|
| [`README.md`](README.md) | Project overview + quick start |
| [`CLAUDE-CODE-START.md`](CLAUDE-CODE-START.md) | Claude Code entry |
| [`MILESTONES.md`](MILESTONES.md) | Milestone gates |
| [`docs/STAGING-SERVER.md`](docs/STAGING-SERVER.md) | Staging deploy + verification |
| [`scripts/run-dev-server.sh`](scripts/run-dev-server.sh) | Local dedicated server launcher |
| [`tbd-framework/Prefabs/Systems/TBD_GameMode.et`](tbd-framework/Prefabs/Systems/TBD_GameMode.et) | Game mode prefab |
| [`Tbdevent_Website/internal/server/server.go`](Tbdevent_Website/internal/server/server.go) | Route registration |
| [`Tbdevent_Website/internal/migrate/migrations/00004_missions_orbat_identity.sql`](Tbdevent_Website/internal/migrate/migrations/00004_missions_orbat_identity.sql) | Missions + ORBAT schema |
| [`Tbd_framework/REFERENCE-ONLY.md`](Tbd_framework/REFERENCE-ONLY.md) | Why not to open CRF in Workbench |

---

## 14. Open decisions

- **Console VOIP:** Partner decides in Phase 0.2. Desktop VOIP cannot run on Xbox/PS5. Likely v1 = PC full VOIP + console in-game VON fallback.
- **Bridge contract:** Main team publishes draft in `tbd-schema` week 1 of Phase 0; finalize after partner spike.

---

## 15. Session checklist for Claude CLI

When starting a new session:

- [ ] Read [`CLAUDE-CODE-START.md`](CLAUDE-CODE-START.md) (or this file for full context)
- [ ] Confirm Enfusion MCP is connected (`/mcp` in Claude Code)
- [ ] Phase 1 task is clear — default: staging deploy or capture objective
- [ ] For Enfusion: `api_search` first, then implement in `tbd-framework/` only
- [ ] For web: read existing handler patterns before adding routes
- [ ] Do not commit unless user asks
- [ ] Update section 6–7 of this file when completing milestones
