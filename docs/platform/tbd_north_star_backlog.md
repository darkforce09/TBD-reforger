# TBD North Star backlog — unplanned gaps

**Purpose:** Capture product ideas that are **real requirements** but were only prose, brain dumps, or build-plan bullets — not yet sliced into executable specs.  
**Registry:** `idea` rows **T-131…T-142** · **Brainstorm:** [`docs/TICKET_BRAINSTORM.md`](../TICKET_BRAINSTORM.md)  
**Authority for near-term work:** [`docs/TICKET_LEAD.md`](../TICKET_LEAD.md) — this doc does **not** change execution order.

---

## How to read this

| Layer | Where it lives | Examples |
|-------|----------------|----------|
| **Active queue** | Registry `ready` / `queued` with specs | T-090, T-068, T-092, T-130 |
| **Deferred Eden / scale** | Registry `deferred` + MC ROADMAP | T-078–T-084, T-110, T-094 |
| **North Star gaps (this doc)** | Registry `idea` T-131+ | Route planner, 3D AAR, mod sets |
| **Platform endgame** | [`tbd-reforger-platform-build-plan.md`](../mod/tbd-reforger-platform-build-plan.md) | M1 event, telemetry, license matrix |

**Do not** start `idea` tickets without promoting to `queued` + a slice spec (`./scripts/ticket mark-ready`).

---

## Three programs (context)

```text
A  Map intelligence     T-090 → T-091 (shipped) → T-090.1.1 Map view → T-090.2–.9 world objects
B  Editor + ORBAT         T-068 → T-071 → T-069+ markers/vehicles
C  Play the mission       T-092 → T-068.13 lobby → T-114–T-120 staging event
```

Most “map understanding” (forests, roads, object glyphs, cartographic Map view) is **program A**, not a gap — see [`t090_091_map_terrain_program.md`](../specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md).

---

## Gap index

| ID | Title | Depends on | Severity |
|----|-------|------------|----------|
| **T-131** | Route planner tool | T-090.5 (roads layer) | Medium |
| **T-132** | Multiplayer MC + visual git | T-062+ scale stable | Large |
| **T-133** | OFCR timed objectives | T-115 (objectives v1) | Medium |
| **T-134** | In-game marker system (mod) | T-069 (editor markers) | Medium |
| **T-135** | Mission modset manager | T-092, license matrix | Medium |
| **T-136** | 3D AAR / OCAP-style replay | T-116, telemetry ingest | Large |
| **T-137** | Discord platform rework | T-118 slotting polish | Medium |
| **T-138** | One-command self-host install | T-124 deps baseline | Medium |
| **T-139** | Lobby loadout visual preview | T-068.13, T-114 | Medium |
| **T-140** | Mission client payload budget | T-092 compile path | Medium (spike) |
| **T-141** | Procedural slot naming | T-071 ORBAT | Low |
| **T-142** | MC shell layout polish | T-082 optional | Low |

**Testing at scale:** no separate ticket — expand acceptance in **T-120** (staging soak + golden mission). Per-slice manual verify logs remain the norm until then.

---

## T-131 — Route planner

**Problem:** After roads exist on the map, mission makers need to plan convoy/supply routes on the road graph — not just draw lines.

**Not in scope:** Real-time pathfinding in Reforger runtime (separate mod concern).

**Depends on:** **T-090.5** vector road layer + exported `roads.json` topology.

**Promotion sketch:**
- Pick start/end (or waypoints) on road network
- Show distance, elevation profile (post-T-091), optional convoy time estimate
- Export route polyline into mission JSON (marker/trigger attachment TBD)

**Related:** LoS/viewshed is **deferred** until T-091 + T-090.6 geometry audit (`engineering_plan.md` §tools).

---

## T-132 — Multiplayer mission editor + visual git

**Problem:** Co-editing missions in browser; diff/review changes like git (visual timeline, blame, merge).

**Reality check:** Y.Doc + undo stack already exist (`ADR-3` in `engineering_plan.md`) — v1 ships **solo**; y-websocket + presence is Phase 4+.

**Visual git:** Folder `docs/specs/Mission_Creator_Mock_Up/tbd_mission_creator_visual_git_diffing/` — no executable slice yet.

**Promotion sketch:**
- Phase 1: export/import mission versions as comparable snapshots (semver already immutable)
- Phase 2: side-by-side diff (slots added/removed/moved)
- Phase 3: CRDT sync server + conflict UX

**Depends on:** T-062 scale program stable @ 360k+ (save/load/conflict already painful).

---

## T-133 — OFCR-style timed objectives

**Problem:** Objectives that **fire or evaluate at mission time T+N** (e.g. “Objective 1 completes check at T+40m”) — OFCR-style pacing, not just capture/destroy/hold.

**Partial coverage:** [`tbd-reforger-platform-build-plan.md`](../mod/tbd-reforger-platform-build-plan.md) §objectives lists `objective_capture`, `objective_destroy`, `objective_hold_until` — no timed trigger graph in editor.

**Depends on:** **T-115** capture/win condition + mission runtime clock in mod.

**Promotion sketch:**
- Editor: objective timeline row (offset minutes, condition type, linked entities)
- Export: `objectives[].schedule` block in mission JSON
- Mod: scheduler evaluates at `missionTime >= offset`

---

## T-134 — In-game marker system (mod)

**Problem:** Reforger base markers are hard to read on the map; TBD wants Arma 3–familiar readability and briefing integration.

**Distinction:** **T-069** = Mission Creator **editor** markers on the 2D map. **T-134** = **runtime** marker rendering + player/admin placement in Enfusion.

**Depends on:** T-069 editor authoring → compiled marker payload in T-092 export.

**Promotion sketch:**
- Custom marker prefabs / HUD styles
- Sync marker set from mission JSON on scenario load
- Optional admin place/remove during live op

---

## T-135 — Mission modset manager

**Problem:** Missions may require different Workshop mod sets (vanilla TBD vs Star Wars vs Halo). Today `apps/mod/tbd-framework/Data/registry.json` has a static `modset` array; no per-mission UI or validation.

**Authority:** Build plan §license matrix + “modset = vanilla + TBD + written permission only” on monetized servers.

**Depends on:** **T-092** (compiled mission references registry aliases resolved against active modset).

**Promotion sketch:**
- Mission meta: `modset_id` or embedded mod list
- Library UI: pick preset + override
- Export validation: every alias resolves for selected modset
- Server config: event declares required modset; join gate if client mismatch

---

## T-136 — 3D AAR / OCAP-style replay

**Problem:** Post-event **after action review** with position samples, fires, captures — eventually **3D replay** (DCS/OCAP direction), not just a stats table.

**Exists today:** Backend models `matches` + `match_player_stats`; service-token telemetry ingest; deployment pages have `aar_replay_url` placeholders. **Missing:** ingest volume contract, storage, replay viewer, mod batching.

**Depends on:** **T-116** results POST + telemetry pipeline hardened in mod (**T-119**).

**Promotion sketch:**
- Spike: 128 players × 2 h × 0.5 Hz sample budget (build plan §A10)
- v1: 2D timeline + map scrubber
- v2: 3D WebGL replay (stretch)

---

## T-137 — Discord platform rework

**Problem:** TBD Discord needs structural rework (channels, roles, bot flows) aligned with event slotting, reminders, AAR links — “boring but necessary.”

**Partial coverage:** Webhooks + OAuth exist; **T-118** covers event ORBAT + identity linking on the **website**, not Discord layout/automation.

**Promotion sketch:**
- Bot: slot claim confirm, T-60/T-10 reminders, post-event AAR link (build plan §B5)
- Role sync policy documented alongside `internal/db/seeds/discord_roles.sql`

**Executor note:** Mix of `human` (community ops) + small `claude-code` bot features.

---

## T-138 — One-command self-host install

**Problem:** Another community cloning the repo should reach a working stack with minimal steps — aspirational one-command install with no manual configuration across Podman, `.env`, migrations, map assets, and mod profile.

**Partial coverage:** **T-124** shipped deps/toolchain; `make db-up` / `make api` / `make web` documented in CLAUDE.md; mod scripts under `scripts/mod/`.

**Promotion sketch:**
- Single entry script: check deps → copy `.env.example` → db-up → migrate → seed roles → optional map-assets fetch (LFS)
- Health check URL list
- Document supported distros (Fedora atomic vs generic Linux)

**Not:** Windows Workbench one-click (separate `scripts/mod/setup-workbench-linux.sh`).

---

## T-139 — Lobby loadout visual preview

**Problem:** Lobby should show **what kit you’re taking** — Squad-style weapon silhouette + hover details; ideally uniform preview. Full 3D per slot is expensive; **2D T-pose composite** may be v1.

**Partial coverage:** **T-068.13** production LOBBY slot picker; **T-114** roster-synced picker; loadout forge **T-068.7+** on human players.

**Depends on:** Registry item icons (**T-068**) + compiled loadout blocks in export.

**Promotion sketch:**
- v1: icon grid + text list from `loadout[]`
- v2: layered 2D mannequin (jacket/pants/helmet slots)
- v3: 3D preview widget (stretch)

---

## T-140 — Mission client payload budget (spike)

**Problem:** “Three-word downloads” — how much mission data can each client receive at join? Compiled JSON size at 360k slots is ~140 MB on **save**; runtime sync may need deltas, caps, or server-side only entities.

**Authority:** Build plan pillar 1 — missions are **data**, not mods — but size limits are unspecified.

**Depends on:** **T-092** compiled export shape + mod loader (**T-130.4** F1-16 profile cap is mod-side precedent).

**Promotion sketch:**
- Measure compiled payload vs slot count curve
- Document max supported entities for console parity
- Propose: server holds bulk; clients stream interest management (if needed)

---

## T-141 — Procedural slot naming (adjectives)

**Problem:** Auto-generate readable slot labels (“Alpha 1-1 Rifleman — Stoic”) from templates + word lists — polish for ORBAT readability and briefing export.

**Depends on:** **T-071** squad numbering + slot roles defined.

**Promotion sketch:**
- Word list packs (adjective/nickname) per faction
- Optional randomize-on-create; manual override always wins
- Export includes display name separate from role id

---

## T-142 — MC shell layout polish

**Problem:** Bottom-left toolbelt icons (Select/Ruler/LoS) and Attributes modal organization feel provisional; Eden parity for **layout** without waiting for full **T-082** field parity.

**Depends on:** Optional **T-082** for field completeness; can ship layout-only earlier.

**Promotion sketch:**
- User research pass on toolbelt placement (bottom center vs docked)
- Group mission-level attributes vs entity attributes
- Hide or defer stub tools until T-091 unlocks LoS

---

## Already planned (not gaps)

Use this table when a brain dump item **already has a ticket**:

| Topic | Ticket / program |
|-------|------------------|
| Satellite basemap | T-090.1.2.8 **shipped** |
| Cartographic Map view | T-090.1.1 |
| Forest regions | T-090.8 |
| Roads/buildings/trees on map | T-090.5 |
| Object icons + LOD | T-090.9 |
| DEM / Z axis | T-091 **shipped** |
| LoS tool (2D) | After T-091 — `engineering_plan.md` |
| ORBAT Manager + standardizations | T-071 |
| Weapon registry export | T-068 |
| Editor map markers | T-069 |
| Squad connect on map | T-071.1; full sync lines **T-080** deferred |
| Mission archive/delete | T-130.6 |
| Mod mission compile + spawn | T-092 |
| Lobby slot picker | T-068.13 → T-114 |
| Objectives capture/destroy/hold | T-115 |
| Safe start / boundary | T-119 |
| Staging event QA | T-120 |
| Building floors | T-129 idea |

---

## Promotion workflow

1. Pick a gap when its `depends_on` chain is **shipped**.
2. Write a slice spec under `docs/platform/` or `docs/specs/…` (copy an existing program hub pattern).
3. `./scripts/ticket mark-ready T-13x path/to/spec.md`
4. `./scripts/ticket sync && ./scripts/ticket check`

---

## Changelog

| Date | Change |
|------|--------|
| 2026-07-02 | Initial capture from operator brain dump; T-131…T-142 registered as `idea`. |
