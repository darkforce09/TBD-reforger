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
- `GET /api/missions` (server token — **mission list** for the in-game admin browser; `{missions:[{id,name,terrain,slotCount,...}],count}`; merges DB + disk like `/compiled`) — added 2026-06-14
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
- **Verified on the staging dedicated server (2026-06-14):** 18× built slot spawn, Stage → LOBBY, mission + roster loaded
- **Mission browser (in progress — see §16):** `TBD_MissionListLoader.c` (list fetch), `TBD_BackendConfig` write-back (`SetMissionId`/`SetBackend`), `TBD_FrameworkManager` select+reload, `TBD_ScenarioRouter.c`, `TBD_AdminCommands.c` (chat — **inactive**, no chat entity), `TBD_MissionBrowser.c` (modded `SCR_PlayerController` RPC backbone + keybind trigger). All compile in Workbench.

### Ops scripts

- `setup-workbench-linux.sh`, `setup-server-profile.sh`, `run-dev-server.sh`
- `deploy-staging.sh`, `remote-log-grep.sh`, `bootstrap-staging-server.sh`
- `setup-client-addons.sh`, `debug-direct-join.sh`
- `mcp-call.sh`, `mcp-wb-logs.sh`, `tbd-spawn-verify.sh`, `tbd-dev-bootstrap.sh`, `setup-mcp-game-root.sh`
- `test-phase1-api.sh`, `manual-test.sh`, `seed-milestone-announcement.sh`

### Not done yet

- Framework: full ORBAT enforcement (identity linking), capture objective, results POST wiring
- **Mission browser — last 5%:** the only missing piece is a client **input action** to fire the RPC (see §16). Backend + game logic + RPC + trigger code all done/compile.
- **Cross-terrain mission switch:** needs a generic TBD scenario built on Arland (Workbench) + `RequestScenarioChangeTransition` arg verification (§16).
- Web: mission upload UI, slot assignment UI, results on event page
- ~~Staging LAN join on 192.168.0.140~~ **DONE 2026-06-14** — `tbd-framework` published to Workshop (modId = gproj GUID `B2C3D4E5F6A78901`), staging runs `-config` mode, client Direct-Joined and spawned at a US slot. See §10.
- **SECURITY (open):** a `license.txt` containing PrairieLearn secrets (Google OAuth client secret, DB pw) was accidentally bundled into the published Workshop mod (v1.0.0/1.0.1). **Rotate those credentials**, remove/replace the file, and re-check the Workshop item.

---

## 7. Current focus — start here

**→ Claude Code:** read [`CLAUDE-CODE-START.md`](CLAUDE-CODE-START.md) first. Use a **new chat** for Phase 1 gameplay work.

### Phase 1 priority (toward Milestone #1 — Sat 2026-08-22)

Build in this order:

1. ~~**Staging LAN join**~~ ✓ DONE 2026-06-14 — client Direct-Joined the `-config` + Workshop-mod server and spawned at a slot (§10)
2. **Finish the mission browser (§16)** — define the 2 input actions so the admin keybind fires the RPC; then cross-terrain (Arland scenario). Everything else is done + compiles.
3. **Capture objective** — win condition for internal test
4. **Full ORBAT enforcement** — identity linking + roster slot assignment (round-robin spawn works today)
5. **Stage machine** — `#stage next`, safe start, boundary
6. **Web admin UI** — mission upload, slot assignment, link-code UX
7. **Results persistence** — wire `POST /api/results` to DB + event page

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
| No Workbench to play | ✓ — staging dedicated server (`-config` + Workshop mod) Direct-Joinable; client spawned at slot 2026-06-14 |

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
| `GET /api/missions` | server token | ✓ | Mission **list** for in-game admin browser (id/name/terrain/slotCount; DB+disk) |
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
| Game port | `2001` UDP/TCP (game) + **`17777` UDP for A2S** — `a2sPort` must **differ** from `bindPort` or replication fails |
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

**Client join:** NOT possible with the local-mod (`-server`+`-addons`) server — see below.
Joining requires Workshop publish + `-config`. Full guide: [`docs/STAGING-SERVER.md`](docs/STAGING-SERVER.md) → "Client join".

### LAN join — SOLVED (2026-06-14)

Root cause was two-fold, both verified with runtime evidence:

1. **Server was crash-looping.** The systemd unit's `-a2sPort 2001` set the A2S query
   socket to the **same UDP port as the game socket** (2001). They can't share a port →
   `Starting RPL server … 2001` then `NETWORK (E): Unable to start replication` → `Game
   destroyed` (exits status 0, so `Restart=on-failure` never fired). **Fixed:** `a2sPort`
   → **17777** in `deploy/tbd-reforger.service` + `scripts/deploy-staging.sh`. Server now
   healthy (18× slot spawn, `Stage → LOBBY`).
2. **`-server`+`-addons` is not Direct-Joinable.** Reforger's Direct Join resolves servers
   through the **BI backend room registry**, not a raw LAN A2S query. Only `-config` mode
   registers a room (`Server registered with address:` + `Direct Join Code:`). Confirmed by
   joining a vanilla `-config` Game Master server by IP from the Proton client. But `-config`
   **cannot** be combined with `-addons` (engine fatal), and config `mods[]` only loads
   **Workshop** mods — so `tbd-framework` must be Workshop-published.

**Not the problem (all verified fine):** versions match (client+server `1.7.0.49`), mod
loads, ping/firewall OK, A2S reachable on 17777. Steam `buildid` differs between client app
(1874880) and server app (1874900) — different apps, so **don't** compare their buildids;
compare the game **version string** instead.

**DONE 2026-06-14:** `tbd-framework` published to the (stable) Workshop — in Reforger the
Workshop **modId is the gproj GUID**, so modId = `B2C3D4E5F6A78901`. Staging now runs
`-config` mode (`deploy.env`: `TBD_SERVER_MODE=config`, `TBD_WORKSHOP_MOD_ID=B2C3D4E5F6A78901`):
the server downloaded the Workshop mod, loaded the TBD scenario (18× slot spawn), registered
a backend room, and a Proton client **Direct-Joined by IP and spawned at a US slot** (server
logged `Authenticated player … name=Darkforce` + faction join). Clients must use the Workshop
mod — **remove** any old `-addonsDir … -addons B2C3D4E5F6A78901` launch options (local unsigned
copy won't hash-match → mod-mismatch). First join may stall on "Downloading required mods";
Cancel + Direct Join again connects. See `docs/STAGING-SERVER.md` → "Client join".

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
| [`tbd-framework/Scripts/Game/TBD/Gamemode/TBD_MissionBrowser.c`](tbd-framework/Scripts/Game/TBD/Gamemode/TBD_MissionBrowser.c) | Admin mission browser RPC + keybind (the working transport; see §16) |
| [`Tbdevent_Website/internal/handlers/gameserver.go`](Tbdevent_Website/internal/handlers/gameserver.go) | `GET /api/missions` list (+ `/compiled`, roster, results) |

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

---

## 16. Mission browser feature — handoff (in progress, 2026-06-14)

**Goal:** the dedicated server pulls a mission **list** from a configurable webserver; an in-game **admin** browses + switches the active mission (Arma-3 style), any terrain, refreshable at runtime.

### Done & verified
- **Backend:** `GET /api/missions` (server-token) → `{missions:[{id,name,terrain,slotCount,schemaVersion,publishedAt}],count}`. `repository.ListMissions` + disk merge in `handlers/gameserver.go` (`MissionList`/`listDiskMissions`), `models.MissionSummary`/`MissionListResponse`, route in `server.go`. Unit tests green; **live-verified** on staging (returns 3 missions: `msn_8f3a2c` everon/18, `msn_2d91be` arland/0, `msn_levieus9` everon/9).
- **Game (compiles in Workbench, live list-fetch verified):**
  - `TBD_MissionListLoader.c` — REST `GET /api/missions` (mirrors `TBD_MissionLoader` pattern); server logged `MissionList: 3 missions available` on LOBBY.
  - `TBD_BackendConfig.c` — `SetMissionId`/`SetBackend` write back to `$profile:TBD_BackendConfig.json` (survives a world reload).
  - `TBD_FrameworkManager.c` — `BuildMissionListText`, `SelectMissionByNumber` (persist id → `GameStateTransitions.RequestScenarioRestart()` same-terrain / `RequestScenarioChangeTransition()` cross-terrain), `SetBackend`; list preloads on LOBBY.
  - `TBD_ScenarioRouter.c` — terrain→scenario map (Everon wired; Arland TODO).
  - `TBD_MissionBrowser.c` — **the transport that works on dedicated**: `modded SCR_PlayerController` with admin-gated client↔server RPCs (`TBD_RequestMissionList`/`TBD_RpcAsk_*`/`TBD_RpcDo_*`, `TBD_RequestSelectMission`) gated by `SCR_PlayerListedAdminManagerComponent.IsPlayerOnAdminList(GetPlayerId())`; plus a client keybind trigger (`AddActionListener("TBD_MissionCycle"/"TBD_MissionLoad", DOWN, …)`).
- **Deploy:** `deploy-staging.sh` config mode renders `game.admins[]` from `TBD_ADMIN_IDENTITY_IDS` (set in `deploy.env`; Darkforce = `873f197a-5ff0-4c45-8f8a-554273c9a875`).

### Dead end (do not retry): chat commands
`TBD_AdminCommands.c` (modded `SCR_ChatComponent.OnNewMessage`, `#tbd` commands) is **inactive** — the `GameMode_Plain`-based TBD scenario has **no `ScriptedChatEntity`**, so chat is never processed (`Chat not available - missing ChatEntity`). Kept for reference only. Use the RPC path.

### THE LAST 5% — define 2 input actions (FILES WRITTEN 2026-06-15, verification pending)
The keybind needs input actions **`TBD_MissionCycle`** + **`TBD_MissionLoad`** to exist.

**Open question RESOLVED (by structure, 2026-06-15):** the base game stores input Actions and Contexts as **modular per-item `.conf` files** under `Configs/System/Actions/*.conf` and `Configs/System/ActionContext/*.conf` (e.g. `ManualCameraTeleport.conf`, `EditorBrowserContext.conf`; also per-input files in `Inputs/`). Strong evidence input configs are **additive** → a mod **ships its own files at those paths**; do **not** override the 235 KB `chimeraInputCommon.conf` (full-copy override crashed precisely because replacing a giant config is fragile). Additive files can't break base keybinds even if malformed.

**Written (additive, `tbd-framework/` only):**
- `Configs/System/Actions/TBD_MissionCycle.conf` — `keyboard:KC_F6`, `InputFilterDown`
- `Configs/System/Actions/TBD_MissionLoad.conf` — `keyboard:KC_F7`, `InputFilterDown`
- `Configs/System/ActionContext/TBD_BrowserContext.conf` — `ActionRefs` → both actions
- `TBD_MissionBrowser.c` → `TBD_TryRegisterListeners()` now calls `im.ActivateContext("TBD_BrowserContext")` before the `AddActionListener` calls (`ActivateContext` verified on `InputManager`/`ActionManager`).

**STILL UNVERIFIED (couldn't test on this Linux/Proton box):** (a) the exact config text syntax — base `.conf` are **binary**, `game_read` returns "incorrect header check", no readable example to copy; (b) whether the engine auto-loads those dirs additively. Why no live test: `wb_reload` reloads **scripts only** (not configs), and this session the WB bridge was in mode **"game"/play** so reload was a **no-op** (`console.log` mtime frozen, `ExecuteAction=false` = nothing happened). Last clean full compile (warnings only, no errors) predates the `.c` edit, so even that one-liner isn't recompiled yet.

**Next session — verify:** restart Workbench (loads new configs **and** recompiles) → check log for parse/compile errors → **Play**, press **F6/F7**, watch for `[TBD][browser]` Prints. OR republish → `deploy-staging.sh` → join → F6/F7. **If actions don't register:** fallback = a *minimal* same-path `Configs/System/chimeraInputCommon.conf` with ONLY the new entries (tests whether input configs merge vs. replace).

### Cross-terrain (after the keybind)
Build a generic TBD scenario on **Arland** in Workbench (`worlds/TBD_Arland.ent` SubScene of `worlds/Arland/Arland.ent`, same `TBD_GameMode`, `Missions/TBD_Arland.conf`), register its `.conf` GUID in `TBD_ScenarioRouter`, verify `RequestScenarioChangeTransition(scenario, worldSystemsConfig, addonList)` args at runtime. An Arland mission with `slots[]` is also needed (sample `msn_2d91be` has 0).

### How to test
Republish the mod → `deploy-staging.sh` (config mode pulls new version + admins) → join → press the keybinds → watch staging `console.log` for `[TBD][browser]` lines + a world reload into the new mission.

---

## 17. Enfusion / Workbench gotchas (learned the hard way — 2026-06-14)

- **`out` is a reserved Enforce keyword** (out-params). Never name a variable `out` (compiles as "Broken expression (missing ';'?)").
- **No ternary `?:`** in Enforce — use `if`/`else`.
- **The local dedicated-server compile check is UNRELIABLE for *new* files** — it reuses a cached `resourceDatabase.rdb` and silently skips newly-added scripts (same file/class count + CRC across runs = it didn't recompile). **Workbench is the authoritative compiler.** Verify via the MCP: `wb_connect` → `wb_reload {target:scripts}` → grep the WB log (`compatdata/1874910/.../ArmaReforgerWorkbench/logs/logs_*/console.log`) for `Can't compile` / `(E):`. (Editing *existing* files does recompile locally; adding files does not.)
- **Publishing packs `data.pak` + `meta` into the source addon dir → Workbench/Launcher treat it as a read-only packed addon** (padlock). Fix: delete `tbd-framework/{data.pak,meta,ServerData.json,*_manifest.json}` (gitignored) after each publish. If the launcher still shows it locked, restart it (it scans at startup). The Workshop link is the gproj **GUID**, not these files — safe to delete.
- **Read-only after a Workbench crash** = no editable project loaded (`wb_projects list` → "No projects loaded"). Reopen the project via the Launcher (Open `addon.gproj`); the MCP `wb_projects open` may return `false` on this Linux/Proton box.
- **Chat doesn't work on a `GameMode_Plain` dedicated scenario** (no `ScriptedChatEntity`). For admin input on dedicated, use **client→server RPC** (`modded SCR_PlayerController` + `[RplRpc(...Server/Owner)]` + `Rpc(...)`), not chat.
- **Direct Join resolves via the BI backend room registry**, not a raw `:2001` probe. Only `-config` mode registers a room; `-server`+`-addons` is **not** joinable. A2S lives on **17777** (must differ from game port `2001` or replication fails). See [[staging-a2sport-collision]] in memory.
- **WB MCP bridge** (this Linux/Proton box): `wb_connect`, `wb_reload`, `project_read`/`project_write` (bypass WB read-only), `wb_projects`. `wb_reload` returns `ExecuteAction=false` but does trigger a reload — confirm via the WB log, not the return value. enfusion-mcp **cannot launch** Workbench; launch via Steam.
- **`config_create`** (MCP) supports faction/mission-header/entity-catalog/editor-placeables only — **not** input configs.
