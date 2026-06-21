# Mission Creator — Master Roadmap

**Document:** `02_roadmap.md`  
**Purpose:** Single planning view — what is **done**, what **must work** for a functional editor, and what is a **separate hard track** (kits/loadouts).  
**Not Eden parity:** See `08_eden_gap_analysis.md` for UI/feature gap vs Arma 3 Eden. This doc is about **shipping a working mission editor**.

---

## Three tracks (do not mix them)

| Track | Question it answers | Blocks gameplay? |
|-------|---------------------|------------------|
| **A — Core Map Editor** | Can I open a map, place things, and get **exact X/Y/Z** into the mission file? | **Yes** |
| **B — Entity & Asset Pipeline** | Can I place **the right entity types** (unit, vehicle, marker, prop) from real game data? | **Yes** (for full missions) |
| **C — Kits & Loadouts** | Can I assign **per-slot gear** (uniform, vest, weapon, attachments) validated against the game? | **Yes** (for ORBAT loadout strings) — but **depends on C-prereqs** |

Tracks A and B can progress in parallel once map assets exist. **Track C is its own program** — do not block Track A on it.

---

## Terminology (fixes the “kits vs armory” confusion)

| Term | What it is today | Where |
|------|------------------|-------|
| **Mission Armory** | Aggregate briefing list (“M16A2 Rifle ×45”) per faction | Backend `MissionArmory` + `GET/PUT /missions/:id/armory` — **already exists**, separate from the editor |
| **ORBAT slot `loadout`** | Short string on each slot in export (`"L85A3 + GL"`) | `json_payload.orbat[].slots[].loadout` — compiler writes `''` today |
| **Loadout (editor model)** | Full per-slot gear graph: uniform, vest, weapons, mags, attachments | Y.Doc `loadouts` + `items` maps in schema — **UI not built** |
| **Master Item Registry** | Every valid classname + slot rules + icons | **Not built** — planned `GET /api/v1/registry`; source = game/modpack ingest (incl. Armory Forger data) |
| **Loadout Forge** | Web UI to edit a slot’s loadout | Stub in `AttributesModal` — Phase 6 in Ultra Plan |

**Armory Forger** (external / mod) = **data source** for Track C, not something the map editor implements. The website needs an **ingest pipeline + Postgres registry**, then the editor **reads** it.

---

## DONE (code-evidenced, 2026-06-20)

### Editor shell & routing
- Lazy route `/missions/:id/edit` (`mission_maker+`, full-bleed)
- Aegis glass UI: top strip, left sidebar (ORBAT + Editor Layers), right asset palette, bottom toolbelt
- Mission Settings dialog (time, weather, view distance, thermals)
- Attributes modal (Identity / Transform read-only / States+Arsenal stubs)

### Map engine (Track A — partial)
- Deck.gl orthographic viewport, Arma meter coords (`flipY: false`, identity projection)
- Terrain **definitions** (Everon 12800×12800 m, Arland, custom bounds)
- Vector grid base map (no satellite/topo imagery yet)
- Pan/zoom with bounds clamp; cursor X/Y in toolbelt
- Icon layer for placed **slots**; selection highlight; marquee select + live overlay
- Drag-move slots with live preview + Y.Doc commit; undo/redo

### Placement (Track B — partial, slots only)
- Mock asset palette (Factions tab); HTML5 drag-drop → `addSlot`
- Auto squad/faction on first drop; active Editor Layer targets drops
- Double-click slot in **Editor Layers** tree → Attributes

### State & persistence (partial)
- Y.Doc normalized store + Zustand mirror + y-indexeddb per mission
- `compileMission` → `json_payload` superset (`orbat[]` + `editor` block with positions)
- Semver Save Version to API; IndexedDB vs server conflict dialog
- Hydrate from server `json_payload` (or lossy ORBAT-only fallback)

### Documentation
- FEDS inventory (`06`), Eden reference (`07`/`07a`/`07b`), gap analysis (`08`), wiki scrape (`09`, 28 pages)

---

## NOT DONE — Track A: Core Map Editor (must work)

These are **required** for “it functions” with **positioning you can trust**.

| ID | Requirement | Status | Notes |
|----|-------------|--------|-------|
| A-01 | **Aligned map imagery** (top-down Everon/Arland tiles, same origin as Reforger) | **Missing** | Today: grid only. Ultra Plan §0.3 asset hosting. Without this, X/Y are mathematically consistent but **not visually verifiable** against the real island. |
| A-02 | **Terrain wired to mission** (`meta.terrain` → viewport) | **Partial** | Hardcoded `terrain="everon"` in `MissionCreatorPage.tsx`. |
| A-03 | **DEM / heightmap** (16-bit, per terrain) | **Missing** | No `dem/` module. All placements get `z: 0`. |
| A-04 | **Z on place & move** (sample DEM at x,y) | **Missing** | `addSlot` / `moveEntity` set `z: 0`. Schema expects `z from DEM`. |
| A-05 | **Z in UI** (toolbelt + Attributes, editable) | **Partial** | Toolbelt shows Z from cursor but no DEM → always 0; Transform tab read-only. |
| A-06 | **Numeric X/Y/Z edit** (no “eyeball only”) | **Missing** | Required for sub-meter accuracy and mod hand-off. |
| A-07 | **Rotation** (numeric + map) | **Missing** | Schema field exists; no UI/tool. |
| A-08 | **Export contract verified** | **Unknown** | Compiler emits positions in `editor` block; **mod must confirm** same coord system as in-game. Need golden JSON from Reforger mod team. |
| A-09 | **Title hydrate from API** | **Missing** | Mission row title not applied to `meta.title`. |
| A-10 | **Autosave to mission version** | **Partial** | Save Version works; continuous autosave debounce not fully wired per Ultra Plan Phase 9. |

**Accuracy note:** Deck.gl `unproject` is exact in **world meters** for the defined terrain bounds. “Off by 10%” failures usually mean **(1)** map tiles not aligned to world origin, **(2)** wrong terrain bounds vs game, or **(3)** Z always zero. Fix A-01 + A-03/A-04 + A-06 before tuning icons.

### Track A — suggested build order
1. A-02 Wire terrain + A-09 title (quick)
2. A-01 Host/import aligned map tiles for Everon
3. A-03/A-04 DEM load + `sampleElevation(x,y)` on place/move/drag-end
4. A-05/A-06 Z (and X/Y) in toolbelt + Attributes — editable, mono font
5. A-08 Golden-file test with mod: one slot at known coords → spawn in-game at same point
6. A-10 Autosave polish

---

## NOT DONE — Track B: Entity & Asset Pipeline

Required to place **objects**, not just generic slots.

| ID | Requirement | Status | Notes |
|----|-------------|--------|-------|
| B-01 | **Registry API** `GET /api/v1/registry?modpack=` | **Missing** | No Go handler, no DB tables for classnames. Ultra Plan backend prereq #1. |
| B-02 | **Registry ingest** (from game server / Armory Forger export) | **Missing** | Push or batch import: weapons, vests, uniforms, mags, attachments, vehicles, props. |
| B-03 | **Catalog UI** backed by registry (not `assetCatalogMock.ts`) | **Missing** | F1-style taxonomy: factions, vehicles, props, markers. |
| B-04 | **`addVehicle` / `addMarker` / `addObjective`** in ydoc | **Missing** | Store maps exist; only `addSlot` implemented. |
| B-05 | **Drop creates correct entity kind** | **Missing** | Mock MRAP/sandbags still create slots. |
| B-06 | **Map layers** for vehicles, markers, objectives | **Missing** | Render, select, move, delete each kind. |
| B-07 | **ORBAT authoring UI** (create faction/squad before place) | **Missing** | `addFaction`/`addSquad` exist in ydoc, no UI. |
| B-08 | **Icons** per classname from registry | **Missing** | Generic slot icon today. |

### Track B — suggested build order
1. B-01/B-02 Minimal registry (even JSON blob v1) — classnames + categories + icons
2. B-03/B-05 Palette reads registry; drop → correct kind
3. B-04/B-06 ydoc + map layers for vehicles & markers
4. B-07 ORBAT tree authoring
5. Expand registry completeness (all variants user mentioned)

**Track B can start with a thin registry** (classname, displayName, category, iconUrl) — full attachment compatibility rules are Track C concern.

---

## NOT DONE — Track C: Kits & Loadouts (Armory Forger)

**Hardest track.** Separate from “put a unit on the map.”

### What “complete” means
- Every gear variant in DB: uniforms, vests (all 50 if there are 50), weapon variants, ammo, attachments, grenades, items, vehicle inventories
- Slot compatibility validation (cannot put X on Y)
- Loadout Forge UI (paper doll + search grid)
- Per-slot `loadoutId` → resolved export in `json_payload.loadouts` + human string in `orbat[].loadout`
- Optional: sync with **Mission Armory** totals (aggregate counts for briefing page)

### Prerequisites (all missing)
| ID | Prerequisite |
|----|--------------|
| C-01 | **Ingest format** from Armory Forger / game — define JSON schema for one export run |
| C-02 | **Postgres schema** — items, attachments, slot types, compat matrix, modpack version |
| C-03 | **Ingest job** — idempotent upsert per modpack version |
| C-04 | **Registry worker** (frontend) — IndexedDB cache, `canEquip` / `canAttach` |
| C-05 | **Loadout Forge UI** — `ArsenalInspector`, `SoldierDoll`, `ItemPicker` |
| C-06 | **Compiler** — resolve `loadoutId` → classnames for mod export |
| C-07 | **Golden loadout** — one kit exported → correct in Reforger spawn |

### Track C — suggested build order
1. C-01/C-02 Get one real Armory Forger export; design tables
2. C-03 Ingest v1 (weapons + mags only)
3. C-04 Worker + cache; read-only validation API
4. C-05 Minimal Forge (primary + optic only)
5. Expand categories (vest, uniform, attachments, …)
6. C-06/C-07 Compiler + in-game verification

**Do not start Track C until A-03/A-04 (Z) and B-01 (registry exists) are underway** — otherwise loadouts attach to slots that cannot be placed accurately.

---

## Explicitly deferred (not required for “functions now”)

- Eden toolbar (widget, snap grids, vision modes)
- Compositions F2, triggers, waypoints, connections
- Comments, crew seat UI, copy/paste
- y-websocket multiplayer
- Workshop publish
- Full Eden tree parity

See `08_eden_gap_analysis.md` P1–P3.

---

## Current vs target (one glance)

```
TODAY                          TARGET (functional v1)
─────────────────────────────────────────────────────────
Grid map                       Aligned topo/sat map + DEM
Slots only                     Units + vehicles + markers
z = 0 always                   z = DEM sample, editable
Mock catalog                   Registry-backed catalog
loadout = ''                   Named loadout per slot (Track C)
editor block positions         Positions verified in-game
Local IndexedDB + manual save  Autosave + semver versions
```

---

## Recommended program order

| Phase | Track | Deliverable | Depends on |
|-------|-------|-------------|------------|
| **1** | A | Terrain wired, title hydrate, numeric X/Y | — |
| **2** | A | Map tiles hosted + aligned; visual parity with island | Art/export from Reforger tools |
| **3** | A | DEM + Z on place/move + editable Z | Phase 2 heightmap asset |
| **4** | A | Mod golden test (one coordinate round-trip) | Mod team JSON spec |
| **5** | B | Registry v1 API + ingest + palette | In-game classname list |
| **6** | B | Vehicles + markers on map | Phase 5 |
| **7** | B | ORBAT authoring | Phase 5 |
| **8** | C | Loadout Forge MVP | Phase 5 + Armory Forger export |
| **9** | C | Full item matrix + compiler loadouts | Phase 8 |

Phases 1–4 = **“map + accurate positions.”** Phases 5–7 = **“real objects.”** Phases 8–9 = **“kits.”**

---

## Related docs

| Doc | Use |
|-----|-----|
| `06_tbd_mission_creator_feature_inventory.md` | Per-feature code status |
| `08_eden_gap_analysis.md` | Eden parity backlog (orthogonal) |
| `03_engineering_ultra_plan.md` | Engineering ADRs, file tree, compiler §8 |
| `01_technical_specification.md` | Why DEM, registry, 200-slot GPU matter |
| `05_agent_execution_plan.md` | Agent phase execution (shell/UI phases) |

---

## Open decisions (need human input)

1. **Map assets** — Do we have Everon top-down tiles + heightmap exports, or must we generate them from Reforger/workshop tools?
2. **Mod JSON contract** — Who provides the golden `json_payload` / spawn format for position + loadout verification?
3. **Armory Forger export** — Exact file/API format for ingest (this unlocks Track C scope: “50 vests” = count rows in export).
4. **Mission Armory vs slot loadouts** — Should Forge changes update `MissionArmory` quantities automatically, or stay separate?
