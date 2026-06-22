# Mission Creator — ROADMAP

**Start here.** Single planning view for the 2D mission editor — what is **done**, what **must work**, and links to all supporting documentation.

**Route:** `/missions/:id/edit` · **Code:** [`frontend/src/features/mission-creator/`](../../frontend/src/features/mission-creator/) + [`tactical-map/`](../../frontend/src/features/tactical-map/)

**Tracks A / B / C** answer different questions — do not mix them. **Eden parity** (UI polish vs Arma 3) is separate: see [`eden/gap_analysis.md`](eden/gap_analysis.md).

---

## Current strategy (locked — 2026-06)

**Eden-first:** Finish the **Eden parity backlog** ( [`eden/gap_analysis.md`](eden/gap_analysis.md) **P0 remaining + P1 + P2** ) **before** Track A Phase 2+ (aligned map tiles **A-01**, DEM/heightmap **A-03/A-04**, and tools that depend on DEM such as Phase 8 LoS/viewshed).

| Do now | Defer until after Eden (+ assets where noted) |
|--------|-----------------------------------------------|
| Eden **P0** remaining — registry, markers, vehicles, ORBAT authoring (P0-01..03, P0-05) | Track A **A-01** map imagery |
| Eden **P1** — selection, copy/paste, search, rotation, ORBAT dbl-click, etc. | Track A **A-03/A-04** DEM + Z sampling |
| Eden **P2** — compositions, triggers, connections, widgets, menus, class search | **A-08** mod golden coord test (needs mod team + accurate map) |
| Thin **Track B** registry as needed to unblock Eden P0 (not “full registry completeness”) | gap_analysis **P3-02/03** (DEM snap, full loadout forge — Track C) |
| Continue Eden quick slices as **T-053+** (spec → code → docs, same as T-048..T-052) | **T-051** title PATCH sync (optional; not Eden-blocking) |

**Rationale:** Eden interaction + entity UX should feel complete on the **flat grid** before investing in hosted heightmaps and satellite tiles. X/Y/Z remain manual/zero until DEM lands; that is acceptable during the Eden push.

### Eden execution order (recommended)

Work [`eden/gap_analysis.md`](eden/gap_analysis.md) **numbered backlog** in priority tier, interleaving small **P1** slices between heavier **P0** blocks:

1. **P1 quick (code-only)** — ~~P1-01 Ctrl+LMB additive select (T-053)~~ → P1-04 asset search → P1-09 ORBAT dbl-click attributes → P1-02 copy/paste → …
2. **P0 ship-blocking** — P0-01 registry (+ thin B-01) → P0-02 markers → P0-03 vehicles → P0-05 ORBAT authoring UI
3. **P1 remainder** — P1-05..P1-11 (multi-place, rotate, Space conflict, vehicle crew, …)
4. **P2 power-user** — P2-01..P2-07
5. **Then** Track A Phase 2 — A-01 tiles → A-03/A-04 DEM → A-08 golden test

Authority for individual Eden items: [`feature_inventory.md`](feature_inventory.md) + [`eden/interactions.md`](eden/interactions.md).

---

## Documentation (read from here)

| Doc | When to open it |
|-----|-----------------|
| **[`agent_execution.md`](agent_execution.md)** | Locked UX decisions, agent phase history, copy-paste agent prompt |
| **[`feature_inventory.md`](feature_inventory.md)** | Per-feature code-evidenced status (FEDS) |
| **[`engineering_plan.md`](engineering_plan.md)** | Y.Doc schema, compiler, workers, engineering phases 0–9 |
| **[`ux_spec.md`](ux_spec.md)** | Eden docked-shell UX contract |
| **[`problem_statement.md`](problem_statement.md)** | Why 200-slot GPU, DEM, nesting, registry matter |
| **[`reference/feds_schema.md`](reference/feds_schema.md)** | FEDS v2 feature-entry schema |
| **[`eden/interactions.md`](eden/interactions.md)** | Eden interaction reference |
| **[`eden/ui_anatomy.md`](eden/ui_anatomy.md)** | Panel-by-panel Eden UI |
| **[`eden/attributes.md`](eden/attributes.md)** | Attribute catalog |
| **[`eden/gap_analysis.md`](eden/gap_analysis.md)** | Eden parity backlog (P0–P3) |
| **[`eden/wiki_manifest.yaml`](eden/wiki_manifest.yaml)** | Scrape manifest — 28 Bohemia Eden Editor wiki pages |
| **[`artifacts/eden-wiki/`](../../artifacts/eden-wiki/)** | **Cached wiki markdown** (generated; do not hand-edit) |
| **[`scripts/tools/scrape-eden-wiki.mjs`](../../scripts/tools/scrape-eden-wiki.mjs)** | Regenerate wiki cache from manifest |
| **[`artifacts/eden-feds-draft.jsonl`](../../artifacts/eden-feds-draft.jsonl)** | Draft FEDS entries derived from wiki research |
| **[`artifacts/README.md`](../../artifacts/README.md)** | Generated artifacts policy |
| **[`t053_eden_p1_additive_select.md`](t053_eden_p1_additive_select.md)** | **T-053** — Eden P1-01: Ctrl/Cmd+LMB additive (toggle) select (shipped) |
| **[`t052_eden_p1_undo_shortcuts.md`](t052_eden_p1_undo_shortcuts.md)** | **T-052** — Eden P1-03: Ctrl/Cmd+Z/Y undo-redo keyboard (shipped) |
| **[`t050_cursor_z_readout.md`](t050_cursor_z_readout.md)** | **T-050** — Cursor Z readout (shipped) |
| **[`t049_track_a_quick_p0.md`](t049_track_a_quick_p0.md)** | **T-049** — Track A quick P0: terrain + title + numeric position (shipped) |
| **[`t048_library_create_dialog.md`](t048_library_create_dialog.md)** | T-048 — Library create dialog (shipped) |
| [`frontend/docs/pages/mission-library.md`](../../frontend/docs/pages/mission-library.md) | Surface spec for `/missions` (+ create dialog T-048) |
| [`frontend/docs/pages/mission-editor.md`](../../frontend/docs/pages/mission-editor.md) | Surface spec for `/missions/:id/edit` |
| [`frontend/docs/pages/mission-creator.md`](../../frontend/docs/pages/mission-creator.md) | Archived — wizard moved into library (T-048) |
| [`CLAUDE.md`](../../CLAUDE.md) §Status | Git milestones T-029–T-052 shipped work |

---

## Three tracks (do not mix them)

| Track | Question it answers | Blocks gameplay? |
|-------|---------------------|------------------|
| **A — Core Map Editor** | Can I open a map, place things, and get **exact X/Y/Z** into the mission file? | **Yes** |
| **B — Entity & Asset Pipeline** | Can I place **the right entity types** (unit, vehicle, marker, prop) from real game data? | **Yes** (for full missions) |
| **C — Kits & Loadouts** | Can I assign **per-slot gear** (uniform, vest, weapon, attachments) validated against the game? | **Yes** (for ORBAT loadout strings) — but **depends on C-prereqs** |

Tracks A and B can progress in parallel **during the Eden push** (registry serves Eden P0). **Track A Phase 2+ (tiles/DEM) is deferred until Eden P0–P2 are done.** Track C remains its own program — do not block Eden on full loadout matrix.

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
- Pan/zoom with bounds clamp; cursor X/Y/Z in toolbelt (Z=0 flat until DEM, T-050)
- Icon layer for placed **slots**; selection highlight; marquee select + live overlay
- Drag-move slots with live preview + Y.Doc commit; undo/redo (buttons + keyboard Cmd/Ctrl+Z/Y, T-052)

### Placement (Track B — partial, slots only)
- Mock asset palette (Factions tab); HTML5 drag-drop → `addSlot`
- Auto squad/faction on first drop; active Editor Layer targets drops
- Double-click slot in **Editor Layers** tree → Attributes

### State & persistence (partial)
- Y.Doc normalized store + Zustand mirror + y-indexeddb per mission
- `compileMission` → `json_payload` superset (`orbat[]` + `editor` block with positions)
- Semver Save Version to API; IndexedDB vs server conflict dialog
- Hydrate from server `json_payload` (or lossy ORBAT-only fallback)

### Documentation & Eden wiki research (T-042)
- FEDS inventory ([`feature_inventory.md`](feature_inventory.md)), Eden reference ([`eden/`](eden/))
- **Arma 3 Eden Editor wiki scrape:** 28 pages in [`artifacts/eden-wiki/`](../../artifacts/eden-wiki/) via [`eden/wiki_manifest.yaml`](eden/wiki_manifest.yaml) + [`scrape-eden-wiki.mjs`](../../scripts/tools/scrape-eden-wiki.mjs); feeds [`eden/interactions.md`](eden/interactions.md), [`eden/ui_anatomy.md`](eden/ui_anatomy.md), [`eden/attributes.md`](eden/attributes.md), [`eden/gap_analysis.md`](eden/gap_analysis.md)

---

## DONE — T-053 (Eden P1 additive select)

| Item | Spec | Deliverable |
|------|------|-------------|
| **Ctrl/Cmd+LMB additive select** | [`t053_eden_p1_additive_select.md`](t053_eden_p1_additive_select.md) | ✅ `TacticalMap onClick` reads `event.srcEvent.ctrlKey/metaKey`; Ctrl/Cmd-click toggles a slot in/out of `selection.ids` (empties → `none`); Ctrl/Cmd + empty-click preserves selection. **Shift unbound** (reserved for range-select); marquee still replaces. One file, no store/`useSelectTool` change. Closes gap_analysis **P1-01** / SEL-MOD-001. |

## DONE — T-052 (Eden P1 undo keyboard)

| Item | Spec | Deliverable |
|------|------|-------------|
| **Ctrl/Cmd+Z/Y undo-redo** | [`t052_eden_p1_undo_shortcuts.md`](t052_eden_p1_undo_shortcuts.md) | ✅ Host keydown in `MissionCreatorPage` + **`useMissionDoc` StrictMode `instanceKey` lifecycle** (dev undo was dead without it). Cmd/Ctrl+Z undo; Cmd/Ctrl+Shift+Z or Ctrl+Y redo; focus guard (INPUT/SELECT/TEXTAREA/contentEditable). Closes gap_analysis **P1-03** / KEY-UNDO-001. |

**Next (Eden-first — see §Current strategy):** T-054+ Eden P1/P0 slices per [`eden/gap_analysis.md`](eden/gap_analysis.md). Immediate: **P1-04** asset browser search, then **P1-09** ORBAT dbl-click attributes, **P1-02** copy/paste. **Deferred:** T-051 title PATCH; Track A A-01/A-03 until Eden P0–P2 complete.

---

## DONE — T-050 (cursor Z readout)

| Item | Spec | Deliverable |
|------|------|-------------|
| **Cursor X/Y/Z** | [`t050_cursor_z_readout.md`](t050_cursor_z_readout.md) | ✅ Toolbelt **CUR** mode shows cursor **X/Y/Z** (was X/Y + dimmed `—`). `onCursorMove` payload + `TacticalMap` `onHover` carry `z: info.coordinate[2] ?? 0`; **Z = 0** on the flat map (real value, not placeholder), off-map → `—`. SEL mode unchanged. |

---

## DONE — T-049 (Track A quick P0)

| Item | Spec | Deliverable |
|------|------|-------------|
| **Terrain + title + numeric position** | [`t049_track_a_quick_p0.md`](t049_track_a_quick_p0.md) | ✅ P0-07 `meta.terrain` → `<TacticalMap>` viewport (key-remount on change); P0-06 `applyMissionRowMeta` hydrates row title/terrain/env on load (fixes empty-`json_payload` early-return); P0-04 `updateSlotPosition` → editable X/Y/Z/rotation in Attributes Transform (x/y clamped to terrain), selection-aware toolbelt readout |

Still blocked on assets for Phase 2+ (map tiles A-01, DEM A-03). Does not include registry/markers/vehicles (P0-01..05).

---

## DONE — T-048 (platform UX)

| Item | Spec | Deliverable |
|------|------|-------------|
| **Create from Library** | [`t048_library_create_dialog.md`](t048_library_create_dialog.md) | ✅ `CreateMissionDialog` on `/missions` (header button + My-Missions empty-state CTA + Cmd/Ctrl+N, `mission_maker+`); `/missions/create` route + sidebar nav removed |

Did not block Track A/B/C editor work; "Mission Creator" labels remain on the dossier CTA + `/missions/:id/edit` breadcrumb (only the standalone wizard tab was removed).

---

## NOT DONE — Track A: Core Map Editor (must work)

These are **required** for “it functions” with **positioning you can trust**.

| ID | Requirement | Status | Notes |
|----|-------------|--------|-------|
| A-01 | **Aligned map imagery** (top-down Everon/Arland tiles, same origin as Reforger) | **Deferred (Eden-first)** | After Eden P0–P2. Today: grid only. Ultra Plan §0.3 asset hosting. |
| A-02 | **Terrain wired to mission** (`meta.terrain` → viewport) | **Done (T-049)** | `terrainId` from `meta.terrain`, `key`-remounts `<TacticalMap>` on change (Everon 12800 / Arland 10240). |
| A-03 | **DEM / heightmap** (16-bit, per terrain) | **Deferred (Eden-first)** | After Eden P0–P2. No `dem/` module yet. |
| A-04 | **Z on place & move** (sample DEM at x,y) | **Deferred (Eden-first)** | After A-03. `addSlot` / `moveEntity` set `z: 0` until DEM. |
| A-05 | **Z in UI** (toolbelt + Attributes, editable) | **Done (T-049/T-050, manual)** | Transform Z editable (T-049); toolbelt shows selected-slot Z (SEL) **and cursor Z (CUR, =0 flat)** (T-050). Auto-sample from DEM still pending (A-03/A-04). |
| A-06 | **Numeric X/Y/Z edit** (no “eyeball only”) | **Done (T-049)** | `updateSlotPosition` + Attributes `NumberField`s (blur/Enter commit; x/y clamped to terrain). |
| A-07 | **Rotation** (numeric + map) | **Partial (T-049)** | Numeric rotation editable in Transform (normalized 0–360); on-map rotate handle still missing. |
| A-08 | **Export contract verified** | **Unknown** | Compiler emits positions in `editor` block; **mod must confirm** same coord system as in-game. Need golden JSON from Reforger mod team. |
| A-09 | **Title hydrate from API** | **Done (T-049)** | `applyMissionRowMeta` applies the mission row `title` (+ terrain/env) to `meta` on load, including empty-`json_payload` missions. No PATCH-back (deferred T-051). |
| A-10 | **Autosave to mission version** | **Partial** | Save Version works; continuous autosave debounce not fully wired per Ultra Plan Phase 9. |

**Accuracy note:** Deck.gl `unproject` is exact in **world meters** for the defined terrain bounds. “Off by 10%” failures usually mean **(1)** map tiles not aligned to world origin, **(2)** wrong terrain bounds vs game, or **(3)** Z always zero. Fix A-01 + A-03/A-04 + A-06 before tuning icons.

### Track A — suggested build order (after Eden P0–P2)

1. ~~A-02 Wire terrain + A-09 title~~ (done T-049)
2. A-01 Host/import aligned map tiles for Everon
3. A-03/A-04 DEM load + `sampleElevation(x,y)` on place/move/drag-end
4. A-08 Golden-file test with mod: one slot at known coords → spawn in-game at same point
5. A-10 Autosave polish

*(A-05/A-06 Z in UI — done T-049/T-050; manual Z until A-04.)*

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

## Scope of the Eden backlog vs truly deferred

**Eden-first** means much of what used to be parked is now **in the active backlog** as P1/P2 slices — only the post-Eden / P3 set stays deferred.

**In the Eden backlog (P1/P2 — do these before tiles/DEM):**
- Copy/paste (P1-02), Ctrl multi-place (P1-05), Shift/map rotate (P1-06), crew UI (P1-10)
- Compositions, triggers, waypoints, connection/sync, transform widget + snap grids (P2)
- Menu bar, class:/mod: search, fuller attribute fields (P2)

**Truly deferred (post-Eden / P3 / external blockers):**
- Track A **map tiles A-01** + **DEM A-03/A-04** (and DEM-dependent Phase 8 LoS/viewshed)
- **P3-01** Workshop compositions
- **P3-02** DEM / 3D snap
- **P3-03** Arsenal / full loadout matrix (Track C)
- **P3-04** y-websocket multiplayer

Item-level priorities: [`eden/gap_analysis.md`](eden/gap_analysis.md) P1–P3.

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

**Active strategy: Eden-first** (see §Current strategy). Phases 2–4 below run **after** Eden P0–P2.

| Phase | Track | Deliverable | Depends on |
|-------|-------|-------------|------------|
| **1** | A | Terrain wired, title hydrate, numeric X/Y | — ✅ T-049 |
| **1b** | Eden | P0 remaining + P1 + P2 (gap_analysis backlog) | T-052+ slices; P0 needs thin registry |
| **2** | A | Map tiles hosted + aligned; visual parity with island | **Eden P0–P2 done** + art/export |
| **3** | A | DEM + Z on place/move + editable Z | Phase 2 heightmap asset |
| **4** | A | Mod golden test (one coordinate round-trip) | Mod team JSON spec |
| **5** | B | Registry v1 API + ingest + palette (expand beyond Eden-minimal) | In-game classname list |
| **6** | B | Vehicles + markers on map (if not closed in Eden P0) | Phase 5 |
| **7** | B | ORBAT authoring (if not closed in Eden P0) | Phase 5 |
| **8** | C | Loadout Forge MVP | Phase 5 + Armory Forger export |
| **9** | C | Full item matrix + compiler loadouts | Phase 8 |

Phases **1b** = **Eden parity on flat grid.** Phases 2–4 = **map + accurate positions (heightmap).** Phases 5–7 = **real objects at scale.** Phases 8–9 = **kits.**

---

## Related docs

All linked in **Documentation** section above. Quick pointers:

| Need | Doc |
|------|-----|
| Code-evidenced feature list | [`feature_inventory.md`](feature_inventory.md) |
| Eden UI parity backlog | [`eden/gap_analysis.md`](eden/gap_analysis.md) |
| Engineering ADRs + compiler | [`engineering_plan.md`](engineering_plan.md) |
| Agent execution + Decisions log | [`agent_execution.md`](agent_execution.md) |

---

## Open decisions (need human input)

1. **Map assets** — Do we have Everon top-down tiles + heightmap exports, or must we generate them from Reforger/workshop tools? *(Gather in parallel; **implementation deferred** until Eden P0–P2 per §Current strategy.)*
2. **Mod JSON contract** — Who provides the golden `json_payload` / spawn format for position + loadout verification?
3. **Armory Forger export** — Exact file/API format for ingest (this unlocks Track C scope: “50 vests” = count rows in export).
4. **Mission Armory vs slot loadouts** — Should Forge changes update `MissionArmory` quantities automatically, or stay separate?
