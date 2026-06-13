# TBD Reforger Platform — Claude CLI Continuation Guide

> **Purpose:** Live handoff document for Claude CLI (or any fresh session) to continue TBD Reforger platform work without re-deriving context from Cursor chat history. **Keep this file updated** as decisions change.
>
> **Last updated:** 2026-06-13 · **Workspace:** `/home/Samuel/Projects/Arma reforger`

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
| A — TBD-Framework | Main team (Samuel) | `tbd-framework` (to create) |
| B — Web platform | Main team | `Tbdevent_Website/` (exists) |
| C — TBD Voice | **Partner** | `tbd-voip` (partner) |
| D — Content/registry | Main team first | `tbd-content` (to create) |
| Schema contract | Main team | `tbd-schema` (to create) |

**Integration boundary:** Shared **bridge contract** in `tbd-schema` — how Mission JSON `radioPlan` maps to voice nets, and game↔client messages (`OnSpawn`, `OnDeath`, `OnNetChange`, `OnPTT`). Main team exposes hooks in framework; partner implements bridge + voice stack.

**Milestone #1 does NOT require VOIP.** In-game VON is acceptable as temporary fallback.

---

## 4. Enfusion MCP — mandatory for game code

Install (Linux/macOS):

```bash
claude mcp add --scope user enfusion-mcp -- npx -y enfusion-mcp
```

Cursor project config — create `.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "enfusion-mcp": {
      "command": "npx",
      "args": ["-y", "enfusion-mcp"]
    }
  }
}
```

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
├── tbd-reforger-platform-build-plan.md   # Original master plan (reference)
├── CLAUDE-CONTINUATION.md                # This file (live — keep updated)
├── MILESTONES.md                         # Milestone #1/#2 dates + criteria
├── .cursor/mcp.json                      # enfusion-mcp enabled
├── .github/workflows/schema.yml          # tbd-schema compatibility CI
├── tbd-schema/                           # DONE: schema, golden missions, bridge contract
│   ├── schema/mission.schema.json
│   ├── schema/registry.schema.json
│   ├── golden-missions/                  # 2 golden missions (validated)
│   ├── registry/registry.example.json
│   ├── bridge/                           # VOIP bridge contract + schema + samples
│   ├── spikes/                           # REST 0.1 report + VOIP 0.2 brief/matrix
│   └── scripts/validate.mjs              # npm run validate
├── Tbdevent_Website/                     # Go + React — LIVE foundation
│   ├── cmd/server/main.go
│   ├── cmd/restspike/                    # Phase 0.1 REST spike harness (no DB)
│   ├── internal/handlers/                # pages, events, auth, admin, gameserver
│   ├── internal/middleware/              # auth + servertoken
│   ├── internal/migrate/migrations/      # 00001–00003
│   ├── missions/                         # compiled missions served by game API
│   ├── scripts/rest-spike.sh             # spike client
│   └── web/src/                          # React SPA
├── Tbd_framework/                        # CRF fork — REFERENCE ONLY
└── (to create)
    ├── tbd-framework/                    # Greenfield Enfusion mod
    └── tbd-content/                      # Compositions, registry assets
```

Partner repo (external): `tbd-voip` — voice client, server, game bridge.

---

## 6. What is already built (website)

**Done in `Tbdevent_Website/`:**

- Discord OAuth + admin CMS (rules, compliance, server, mods pages)
- Event hub: sidebar layout, events, announcements, dashboard
- Discord sign-up with capacity/waitlist (`event_registrations` table)
- Admin tabs: Content, Events, Announcements, Registrations

**Done (Phase 0 — added in this pass):**

- Server-token auth for game servers (`internal/middleware/servertoken.go`)
- `GET /api/missions/{id}/compiled` (serves from `MISSIONS_DIR`)
- `POST /api/results`, `POST /api/telemetry` (log-only)
- REST spike harness (`cmd/restspike`) + client (`scripts/rest-spike.sh`) — loop verified
- `tbd-schema`: Mission JSON schema v1, registry schema, 2 golden missions, VOIP bridge contract; `npm run validate` passes
- Env: `GAME_SERVER_TOKENS`, `MISSIONS_DIR` in `.env.example`

**Not done (next):**

- `POST /api/link` (identity linking) + 6-digit lobby code
- Mission JSON upload UI + schema validation in the web app
- ORBAT role slotting (upgrade from headcount sign-up)
- `GET /api/events/{id}/roster` returning identityId → slotId for the game server
- Registry POC 0.4 (alias → GUID in Enfusion) and map tiles 0.3
- TBD-Framework Enfusion mod (Phase 1)
- Mission wizard (Phase 2)

**Existing migrations:** `00001_init.sql`, `00002_seed_content.sql`, `00003_events_announcements.sql`

---

## 7. Current focus — start here

**→ Claude Code users:** read [`CLAUDE-CODE-START.md`](CLAUDE-CODE-START.md) first (Workbench bootstrap, API verification, MCP setup). Training data for Enfusion is ~3 years stale — use **enfusion-mcp** for every script change.

### Immediate priority: Workbench green on `tbd-framework/`

1. **Sync Steam builds** — Arma Reforger + Arma Reforger Tools must match (mismatches cause vanilla errors like `Tuple2`).
2. **Base game path** — `bash scripts/setup-workbench-linux.sh` → locate `~/ArmaReforger-Base/data/ArmaReforger.gproj`.
3. **Open only** `tbd-framework/addon.gproj` — never `Tbd_framework/` (Coalition).
4. **Verify APIs** via enfusion-mcp before editing `TBD_*.c` (e.g. `RestCallback` — no `SetOnTimeout` on current API).
5. **POC pass** — attach `TBD_FrameworkManager` + `TBD_RegistryPocComponent`; dedicated server + profile from `scripts/setup-server-profile.sh`.

### After Workbench POC

**Done (Phase 0 → Phase 1 start):**

- Schema **v1.0 frozen** — `tbd-schema/VERSION`, `CHANGELOG.md`, registry `guid` accepts full Enfusion ResourceName
- **Registry POC 0.4** — `registry.vanilla-poc.json`, `tbd-framework/Data/registry.json`, `TBD_Registry.c` + `TBD_RegistryPocComponent.c` (verify in Workbench)
- **`tbd-framework/` scaffold** — mission loader (REST + profile fallback), stage manager stub, radio bridge stubs
- **Web Phase 1 API (backend):**
  - `POST /api/missions` (admin, schema-validated upload)
  - `POST /api/link` (server token — register 6-digit code)
  - `POST /api/me/link` (user consumes code)
  - `GET /api/game/events/{id}/roster` (server token — identityId → slotId)
  - `PUT /api/admin/events/{id}/slots/{slotId}` (manual ORBAT)
  - Migration `00004_missions_orbat_identity.sql`

**Not done (next):**

- Registry POC **Workbench verification** (human + Enfusion MCP connected)
- Map tiles **0.3** spike
- Framework: capture objective, loadouts, ORBAT enforcement in-game, admin chat commands wired
- Web: upload UI, slot assignment UI, results persistence (still log-only)
- Milestone #1 date posted in Discord
- Staging dedicated server soak

**Next, in order:**

1. **Confirm Milestone #1 date publicly** in Discord.
2. **Open `tbd-framework` in Workbench** — run registry POC component; fix `kit:us_sl` GUID from `Character_US_SL.et`.
3. **Run migration** (`00004`) on dev Postgres; test link + roster flow with curl.
4. **Framework Phase 1** — spawner, capture zone, roster enforcement calling `GET /api/game/events/{id}/roster`.
5. **Map tiles 0.3** — Everon ortho pipeline (Phase 2 wizard dependency).

Partner runs **Phase 0.2 VOIP spike** in parallel — see `tbd-schema/spikes/voip-spike-brief.md`; fills `voip-capability-matrix.md`.

### Phase 1 (after Phase 0 green)

**TBD-Framework** (use Enfusion MCP for all code):

- State machine: `LOADING → LOBBY → BRIEFING → SAFE_START → LIVE → END → DEBRIEF`
- Mission loader: REST + `$profile/missions/{id}.json` fallback
- Registry spawner, capture objective, loadouts, safe start, boundary
- ORBAT slot enforcement via backend roster
- Admin commands (`#stage next`, etc.)
- Radio bridge **stub** (hooks only — partner wires later)

**Web:** JSON upload, events linked to missions, manual ORBAT assignment, identity linking.

**Milestone #1:** 20–40 players, hand-written mission JSON, slots enforce, side wins.

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

## 9. Game server API (to implement)

| Endpoint | Auth | Purpose |
|---|---|---|
| `GET /api/missions/{id}/compiled` | server token | Mission JSON for loader |
| `GET /api/game/events/{id}/roster` | server token | identityId → slotId map |
| `POST /api/link` | server token | Bind 6-digit code → game identity |
| `POST /api/telemetry` | server token | Batched gameplay events |
| `POST /api/results` | server token | Final mission results |
| `GET /api/servers/{id}/commands` | server token | Queued admin actions (poll) |
| `POST /api/missions` (+validate) | user session | Mission publish |
| `POST /api/events/{id}/slots/{slotId}/claim` | user session | ORBAT slot claim |

~~`GET /api/entitlements/{identityId}`~~ — removed (no payments).

---

## 10. What NOT to do

- Do NOT build Stripe, payments, or entitlements
- Do NOT build in-engine TBD-VON — partner builds external TFAR-like stack
- Do NOT resume CRF fork (mod-list pruning, CVON strip, TBD branding) as primary path
- Do NOT guess Enfusion class names — use Enfusion MCP
- Do NOT start mission wizard before Phase 0 spikes + schema freeze
- Do NOT block Milestone #1 on VOIP

---

## 11. Coding conventions

**Website (Go):** chi router, pgx, goose migrations, scs sessions. Match existing patterns in `Tbdevent_Website/internal/`.

**Enfusion:** Original TBD-owned code. Clean-room — may read CRF/BI samples for patterns, never copy verbatim. Verify all APIs via MCP.

**Commits:** Only when user explicitly asks.

**This file:** Update section 7 (Current focus) and section 6 (built/not done) whenever a milestone completes or priorities shift.

---

## 12. Key reference files

| File | Why |
|---|---|
| `tbd-reforger-platform-build-plan.md` | Full schema outline, workstreams, phases, risks |
| `Tbdevent_Website/internal/server/server.go` | Route registration |
| `Tbdevent_Website/internal/migrate/migrations/00003_events_announcements.sql` | Current events schema |
| `Tbd_framework/Scripts/Game/Systems/Core/Managers/PlayerController/Replication/CRF_PlayerRplToAuthorityManager.c` | Example RestApi usage in CRF (reference) |
| `Tbd_framework/Scripts/Game/Systems/ModdedOverrides/CRF_CVON_GamemodeOverride.c` | How CRF disables CVON — pattern reference for partner VOIP bridge |

---

## 13. Open decisions

- **Console VOIP:** Partner decides in Phase 0.2. Desktop VOIP cannot run on Xbox/PS5. Likely v1 = PC full VOIP + console in-game VON fallback.
- **Bridge contract:** Main team publishes draft in `tbd-schema` week 1 of Phase 0; finalize after partner spike.

---

## 14. Session checklist for Claude CLI

When starting a new session:

- [ ] Read this file
- [ ] Confirm Enfusion MCP is connected (`/mcp` in Claude Code)
- [ ] Ask user which Phase 0/1 task to tackle if unclear
- [ ] For Enfusion work: `api_search` first, then implement
- [ ] For web work: read existing handler/repository patterns before adding routes
- [ ] Do not commit unless user asks
- [ ] Update this file when completing items from section 7
