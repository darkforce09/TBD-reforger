---
name: Mission Creator — Agent Execution Plan
overview: "Self-contained agent handoff for Mission Creator. T-057–T-062.1.1 shipped. Active: T-063..T-067. Eden T-068+."
todos:
  - id: step-0-publish
    content: "STEP 0: Plan published to Design_Docs/Mission_Creator_Architecture/agent_execution.md"
    status: completed
  - id: phase-pre-35
    content: "PHASE PRE-3.5: Wire Outliner + asset drag-to-map (T-033) — done"
    status: completed
  - id: phase-doc-0
    content: "PHASE DOC-0: Create 04_eden_editor_ux_spec.md + patch engineering_plan.md + mission_creator_design.md + CLAUDE.md"
    status: completed
  - id: phase-3-5
    content: "PHASE 3.5: Eden shell fidelity — docked layout, top bar, left sidebar sections, tabbed asset palette, modal-only inspector, topo skin"
    status: completed
  - id: phase-7b
    content: "PHASE 7b: Map drag-to-move, marquee multi-select, group move, Spacebar center, Delete key; remove click-to-teleport"
    status: completed
  - id: phase-7a
    content: "PHASE 7a: Outliner reparent DnD, folder rename/delete, slot delete; commit in-flight tree wiring"
    status: completed
  - id: phase-9
    content: "PHASE 9: Compiler + Export + useMissionEditor autosave (only after 3.5, 7b, 7a complete)"
    status: completed
  - id: eden-backlog
    content: "T-057–T-062.1.1 SHIPPED. Active: T-063..T-067. Eden T-068+."
    status: in_progress
  - id: phase-blocked
    content: "DEFERRED until after Eden P0-P2: Phase 2 DEM/tiles, full registry/Arsenal (Phases 5-6), Phase 8 tools — do not start without user approval"
    status: pending
isProject: false
---

# AGENT EXECUTION CONTRACT

> **Phase completion log (T-033–T-040):** PRE-3.5 ✅ DOC-0 ✅ 3.5 ✅ 7b ✅ 7a ✅ 9 ✅.
> **North star:** **1M–10M editable entities** via **T-059..T-067**. **T-060 shipped** (`b1fd25a`). **T-061 shipped (good enough)** — drag @ ~360k. **T-062 shipped** — incremental bindings @ 360k. **T-062.2 shipped** — editor session / alt-tab. **T-062.1 shipped** — chunked IDB slot restore. **T-062.1.1 shipped** — Save orbat dedup (editor-only POST; Go derives ORBAT for events). **Active: T-063..T-067.** Mega optimizations deferred ([ROADMAP.md](ROADMAP.md) §Deferred mega optimizations). Eden **T-068+**.

> **For the human:** Open a new Cursor Agent / CLI session and paste the prompt below. The agent reads this file; execute **open** phases only.

## One-line prompt (copy this)

```
Read CLAUDE.md first. Mission Creator is Eden-first (locked 2026-06): the shell phases
PRE-3.5–9 are DONE (T-033–T-040). Open work = the Eden parity backlog in
eden/gap_analysis.md — P0 remaining + P1 + P2 — shipped as T-053+ slices BEFORE Track A
Phase 2 (map tiles A-01, DEM A-03/A-04) and DEM-dependent Phase 8 tools. Authority:
ROADMAP.md §Current strategy → this file's Decisions log for UX locks. **T-060–T-062.1.1 shipped.** **Active: T-063..T-067** → Eden **T-068+**.
A thin Track B registry (B-01) for Eden P0 (P0-01..03) is in scope; full registry/Arsenal and
tiles/DEM are deferred. After each slice: `cd frontend && npm run build && npm run lint`. Do not
commit unless I ask.
```

Shorter variant:

```
ROADMAP.md §Current strategy → @agent_execution.md §ACTIVE SLICE. **T-062.1.1 shipped.** **Active: T-063..T-067** → Eden **T-068+**.
```

## Agent roles — Cursor vs Claude Code (locked 2026-06)

**Human workflow:** save tokens on Claude Code by splitting **code** vs **documentation**.

| Role | Tool | Does | Does NOT |
|------|------|------|----------|
| **Documentation owner** | **Cursor — Composer 2.5** | Write and sync all project docs (specs, ROADMAPs, `CLAUDE.md`, `agent_execution.md`, `TAGS.md`, page specs, acceptance checkboxes, Claude Code prompts, plan files) | — |
| **Code implementer** | **Claude Code** | Read docs as source of truth; implement code + tests; run verify commands; report outcomes (logs, curl results, manual verify) back to the human | Edit documentation files (no doc sync passes — Cursor handles that in a separate step or session) |

**Handoff pattern:**

1. **Cursor** — plan, diagnose, write/update specs + prompts + doc sync table; paste a **code-only** Claude Code prompt (no `PART N — DOCS`).
2. **Claude Code** — read listed docs; ship code; return verify output + bullet summary for Cursor.
3. **Cursor** — flip acceptance checkboxes, §Status, ACTIVE SLICE, TAGS, etc., in the same commit the human requests (or before the next Claude Code slice).

Claude Code prompts in `t0xx_*.md` files should end with **DO NOT edit documentation** — list which files Cursor will sync instead.

---

| Priority | Document | Agent uses it for |
|----------|----------|-------------------|
| **0** | **`ROADMAP.md`** | **Planning authority** — Tracks A/B/C, DONE vs must-work, doc index. Start here. |
| **1** | **This file** (`agent_execution.md`) | **Execution authority** for UX phases. Decisions log. If UX conflicts, this file wins over ROADMAP priorities. |
| **2** | **Decisions log** (below) | Locked human choices. Do not re-litigate. |
| **3** | `ux_spec.md` | UX contract — copies Decisions log + interaction table. |
| **3b** | `reference/feds_schema.md` | **FEDS v2** — normative per-feature record format (UI Surface, Wiki anchor). |
| **3c** | `feature_inventory.md` | **What TBD has** — code-evidenced feature inventory. |
| **3d** | `eden/ui_anatomy.md` | **Eden UI** — panel-by-panel layout (Asset Browser, Toolbar, Entity List). |
| **3e** | `eden/attributes.md` | **Eden attributes** — `ATTR-FIELD-*` per entity type. |
| **3f** | `eden/interactions.md` | **Eden interactions** — wiki-anchored FEDS (toolbar, compositions, connect, …). |
| **3g** | `eden/gap_analysis.md` | **Gap + backlog** — ID-linked parity; P0–P3. |
| **3i** | `ROADMAP.md` | **Master roadmap** — DONE vs must-work (Tracks A/B/C); kits vs armory clarified. |
| **3h** | `eden/wiki_manifest.yaml` + `scripts/tools/scrape-eden-wiki.mjs` | Wiki scrape manifest + automation; cache in `artifacts/eden-wiki/`. |
| **4** | `engineering_plan.md` | Engineering ADRs, Y.Doc schema, compiler/export contract, file tree. |
| **5** | `CLAUDE.md` | Repo conventions, run commands, commit tags. |
| **6** | Aegis design tokens | `frontend/src/index.css` + label/spacing scale (`text-label-sm`, `overlayPanel`, etc.). Glass palette only — **not layout**. |

**Do not use for layout or interaction decisions** (historical HTML explorations — they **contradict each other** and the Decisions log):

- `Design_Docs/Mission_Creator_Mock_Up/**/code.html`, `screen.png`
- `Design_Docs/macOS_Blueprints/**/code.html`, `screen.png` (editor-related — see map below)

**Supplementary only** (style tokens / product vision — read when noted, never override this plan):

| Path | Use for |
|------|---------|
| `aegis_tokens/DESIGN.md` | Aegis color tokens, typography scale, **256px / 320px** panel widths |
| `frontend/src/index.css` + `overlay.ts` | Live glass palette, semantic classes |
| `mission_creator_design.md` | Long-term product vision (Forge, Visual-Git, Briefing UI) — **deferred** items |
| `problem_statement.md` | *Why* the four hard problems exist (200 slots, DEM, nesting, registry) |
| `engineering_plan.md` | Full engineering phases 0–9, file tree, compiler §8, workers, DEM |

Visual target: **Arma 3 Eden Editor** layout + interactions, **modernized with Aegis glass**. Dimensions: left **256px** (`w-64`), right **320px** (`w-80`), both docked flush; map between them.

| Code | Route |
|------|-------|
| `frontend/src/features/mission-creator/` + `frontend/src/features/tactical-map/` | `/missions/:id/edit` |
| `frontend/src/pages/missions.tsx` | Mission library (entry to editor) + **CreateMissionDialog** launch (T-048) |
| `frontend/src/features/mission-creator/CreateMissionDialog.tsx` | Create-mission dialog on `/missions` (T-048; replaced the `/missions/create` wizard) |

**STEP 0:** Done — this file is in the repo. Shell phases PRE-3.5–9 are DONE (T-033–T-040); new sessions start at **[`ROADMAP.md`](ROADMAP.md)** and execute only OPEN items.

---

## Repository documentation map

Every Mission Creator-related folder and its role. **Execution authority remains this file**; other docs provide engineering depth or historical context.

### `Design_Docs/Mission_Creator_Architecture/` — engineering

| File | Role |
|------|------|
| `agent_execution.md` | **This file** — phases, decisions, acceptance criteria |
| `engineering_plan.md` | ADRs, full file tree, phases 0–9, Y.Doc schema, compiler JSON §8, workers |
| `problem_statement.md` | Problem statement (200-slot DOM, DEM, nesting, registry) |
| `ux_spec.md` | Human-readable UX contract copied from Decisions log |
| `ROADMAP.md` | Master roadmap — Tracks A/B/C |
| `feature_inventory.md` | Code-evidenced feature inventory |
| `eden/` | Eden parity research (interactions, UI, attributes, gaps) |
| `reference/feds_schema.md` | FEDS v2 schema |

### `Design_Docs/Mission_Creator_Mock_Up/` — product + early UI explorations

| Path | Role |
|------|------|
| `mission_creator_design.md` | Product blueprint: Forge, Loadout Forge, Visual-Git, Briefing UI, JSON sync |
| `aegis_tokens/DESIGN.md` | Aegis tokens + panel dimensions (256 / 320) |
| `aegis_tokens/code.html` + `screen.png` | Historical layout exploration — **do not execute against** |
| `Arsenal/DESIGN.md` | Arsenal / Loadout Forge visual tokens (Phase 5–6) |

### `Design_Docs/macOS_Blueprints/` — editor-adjacent references

| Path | Role |
|------|------|
| `aegis_mission_editor_macos_edition/` | Early editor chrome exploration |
| `mission_editor_tactical_canvas/` | Map canvas styling reference |
| `tbd_mission_creator_visual_git_diffing/` | Future Visual-Git UI (Phase 9+) |
| `loadout_forge_tactical_equipment_management/` | Future Arsenal UI (Phase 6) |

### `frontend/src/features/` — implementation (source code)

| Module | Role |
|--------|------|
| `tactical-map/` | Deck.gl engine, Y.Doc state, layers, coords — **terrain-agnostic** |
| `mission-creator/` | Editor shell: layout panels, hooks, modals |

Key engine files already exist: `TacticalMap.tsx`, `state/{ydoc,schema,bindings,useMapStore,undo}.ts`, `layers/useIconLayer.ts`, `hooks/useMissionDoc.ts`.

Key shell files: `MissionCreatorPage.tsx`, `layout/{TopCommandStrip,BottomToolbelt,OutlinerPanel,AssetBrowser,InspectorPanel,AttributesModal}.tsx`.

**Not yet built** (per Ultra Plan): `dem/*`, `tools/*`, `registry/*`, `compiler/*`, `hooks/useMissionEditor.ts`, most extra layers.

### Other

| Path | Role |
|------|------|
| `CLAUDE.md` (T-029–T-032) | Shipped status snapshot — update in DOC-0 |
| `frontend/docs/pages/mission-library.md` | Create-mission dialog spec (T-048; superseded the `/missions/create` wizard) |
| `frontend/src/stitch-exports/mission_creator_setup_wizard/` | Wizard HTML mock (archived) |

---

## Architecture state (what exists today)

```mermaid
flowchart LR
  subgraph shipped [Shipped T-029 to T-062.1]
    Deck["Deck.gl viewport"]
    YDoc["Y.Doc + editorLayers"]
    IDB["v2 idb persist"]
    Undo["Y.UndoManager"]
    Icons["IconLayer slots"]
    Shell["Eden docked shell"]
    Save["Compiler + Save Version"]
  end
  subgraph active [Active T-063 to T-067]
    Scale["Scale program"]
  end
  subgraph later [After scale milestones]
    Eden["Eden T-068+"]
    Terrain["T-070+ terrain base"]
  end
  shipped --> active --> later
```

**Data flow (do not break):** mutations → `ydoc.ts` `transact()` → `bindings.ts` → `useMapStore` → Deck layers. Only `selection`, `activeTool`, `activeLayerId` are set directly on Zustand.

**Entity maps in Y.Doc:** `meta`, `factions`, `squads`, `slots`, `loadouts`, `items`, `objectives`, `vehicles`, `markers`, `editorLayers`.

**What works end-to-end today (post T-056):** fullscreen Eden docked shell (no platform chrome) on `/missions/:id/edit`; pan/zoom grid with terrain-driven bounds (`meta.terrain`, T-049); drag mock catalog unit onto map → active Editor Layer; **drag-to-move icons + marquee multi-select + group move** (T-036); **Ctrl/Cmd-click additive toggle select** (T-053); **Ctrl/Cmd+C/V copy-paste at cursor** (T-056); Delete/Backspace; Spacebar centers on selection; **keyboard undo/redo** Cmd/Ctrl+Z / Shift+Z / Ctrl+Y (T-052); **double-click → Attributes modal** from map icons, ORBAT slot rows, and Editor Layers slot rows (T-054; multi-select suppresses) with **editable numeric X/Y/Z + rotation** (T-049) and role/tag/stance; **Asset Browser search** filters the Factions catalog tree by name (T-055); **title/terrain/env hydrate** from the mission row on load (T-049); outliner reparent/rename/delete (T-037); **compiler → `json_payload`, manual Save Version + Export, IndexedDB↔server conflict prompt** (T-038); cursor X/Y/Z readout (Z=0 flat, T-050); local IndexedDB per mission id.

**Known regression (T-057 — resolved):** ~~~100–200 slots + pan → ~9 fps~~ Fixed T-057: cursor off render path, no hover pick, pan rAF-coalesce. Manual acceptance: ≥55 fps @ 200+ via `FpsCounter`.

**Open Eden gaps (active after T-060..T-067 scale milestones — `eden/gap_analysis.md`):**
- **P0 ship-blocking:** P0-01 real asset registry + catalog parity, P0-02 markers on map, P0-03 vehicles placeable, P0-05 ORBAT authoring UI (currently read-only).
- **P1 Eden feel:** P1-05 Ctrl multi-place, P1-06 Shift/map rotate, P1-07 faction submode, P1-08 Space conflict, P1-10 vehicle crew, P1-11 empty-vehicle Alt place. *(P1-01 → T-053; P1-02 copy/paste → T-056; P1-03 undo keyboard → T-052; P1-04 asset search → T-055; P1-09 Attributes entry → T-054.)*
- **P2 power-user:** compositions, triggers/waypoints/systems, connection/sync, transform widget + snap grids, full attribute fields, menu bar, class:/mod: search.
- **Deferred until after Eden P0–P2:** DEM Z-axis + Z auto-sample, aligned map tiles, full registry/Arsenal (Track C loadouts), Phase 8 ruler/LoS/viewshed.

---

## Full phase roadmap

| Phase | Name | Status | Deliverable |
|-------|------|--------|-------------|
| 0–1 | Viewport | **Done** | Deck.gl orthographic map, pan/zoom, procedural grid |
| 4 | State foundation | **Done** | Y.Doc, Zustand mirror, undo, IconLayer, v2 idb persist (T-062.1) |
| 3a | Shell scaffold | **Done** | Floating panels, TreeView, modals (T-031/032) |
| PRE-3.5 | Land tree wiring | **Done** (T-033) | editorLayers + palette DnD baseline |
| DOC-0 | Doc alignment | **Done** (T-034) | `ux_spec.md` + patch ultra plan, CLAUDE, design |
| **3.5** | **Eden shell** | **Done** (T-035) | Fullscreen, docked sidebars, palette tabs, modal inspector |
| **7b** | **Map manipulation** | **Done** (T-036) | Drag-move, marquee, Spacebar, Delete |
| **7a** | **Outliner ops** | **Done** (T-037) | Reparent, rename, delete folders/slots |
| **9** | **Compiler + save** | **Done** (T-038) | `json_payload` export, Save Version |
| 2 | DEM / Z-axis | Blocked | Heightmap assets |
| 5–6 | Registry + Arsenal | Blocked | `GET /api/v1/registry` |
| 8 | Tools + objectives | Blocked | Ruler, zones, LoS GLSL |
| T-048 | Create dialog | Done | `CreateMissionDialog` on `/missions` → POST mission → open editor (replaced `/missions/create`) |

```mermaid
flowchart TD
  pre["PRE-3.5 wiring"]
  doc["DOC-0 docs"]
  p35["3.5 Eden shell"]
  p7b["7b map drag"]
  p7a["7a outliner"]
  p9["9 save/export"]
  pre --> doc --> p35 --> p7b --> p7a --> p9
```

---

## Current gaps (Eden target vs code today)

| Eden / Decisions log | Current code | Fixed in |
|---------------------|--------------|----------|
| Fullscreen editor (no platform nav) | `Sidebar` + `TopNav` still visible | Phase 3.5 |
| Docked L/R panels, map between | Floating `inset-x-4` panels | Phase 3.5 |
| Asset palette always visible | Right panel swaps to `SlotInspector` | Phase 3.5 |
| ORBAT + Editor Layers sections | Workflow folders only | Phase 3.5 |
| Attributes modal on double-click | Modal stub; fields in SlotInspector | Phase 3.5 |
| Eden time slider/scrub | Hidden in MissionSettingsDialog | Phase 3.5 |
| Topo map + grid overlay | Procedural line grid only | Phase 3.5 |
| Click-drag icons to move | Click entity, click map to teleport | Phase 7b |
| Marquee multi-select | Single `selection.id` | Phase 7b |
| Spacebar to center | Auto `flyTo` on outliner click | Phase 3.5 + 7b |
| Delete key | No keyboard delete | Phase 7b |
| Export + API autosave | Export disabled; IndexedDB only | Phase 9 |
| Terrain drives viewport bounds | Hardcoded `terrain="everon"` | T-049 |
| Mission row title/terrain on load | Always "Untitled Mission"; empty-payload early-return | T-049 |
| Editable numeric X/Y/Z/rotation | Read-only Transform; stale "coming later" copy | T-049 |

---

## Interaction contract

| User action | System response |
|-------------|-----------------|
| Drag asset from palette | Place entity on map; file in active Editor Layer |
| Single-click entity | Select + highlight + outliner sync (**no** camera move) |
| Double-click entity | Open **AttributesModal** (Transform, Identity, States, Arsenal tabs) |
| Click-drag entity on map | Move entity; one undo step on release |
| Left-drag on empty map | Marquee box-select |
| Middle-mouse / right-drag | Pan/zoom map |
| Click-drag selected group | Move all selected together; one undo step |
| Spacebar | Center camera on selection |
| Delete / Backspace | Delete selected entities (undoable) |
| Click empty map | Clear selection only |
| Click outliner row | Select entity (**no** camera move until Spacebar) |

---

## Decisions log (human-confirmed — agent must follow)

These resolve ambiguities from earlier drafts. **Do not re-litigate without user approval.**

| Topic | Decision |
|-------|----------|
| **Visual target** | **Arma 3 Eden Editor** layout + interactions, **modernized with Aegis glass** (macOS). Not HTML mockups. |
| **Platform chrome** | **Hide** platform `Sidebar` + `TopNav` on `/missions/:id/edit` — true fullscreen Eden-style editor (dedicated layout escape in `AppLayout` or editor wrapper). |
| **Left sidebar** | **Both sections visible** in one scroll: **ORBAT** (Factions→Squads→Slots) on top, **Editor Layers** (workflow folders) below. Stub sections for Waypoints/Zones/Logic until Phase 8. |
| **Right palette** | **Docked flush right** — mirror left sidebar (~`w-80` / 320px), full height below top bar, no floating gap. Map sits between two glass panels. |
| **Inspector** | Asset Palette always visible. **Attributes modal on double-click only** (no right-panel inspector swap). |
| **Map pan** | **Middle-mouse or right-drag** = pan/zoom. **Left-drag on empty map** = marquee box-select. |
| **Multi-select** | **Marquee box** is the primary multi-select method. Shift+click additive toggle is optional bonus, not required for v1. |
| **Center camera** | **No auto flyTo on click.** Select unit → press **Spacebar** to center camera on selection (map or outliner). |
| **Delete** | **Delete/Backspace** removes selected entities; **undoable** (one transaction). No confirmation dialog. |
| **Load conflict** | When API `json_payload` and local IndexedDB disagree on a **cold** load → **prompt user** to choose which to keep. **Warm return** (T-062.2): same-tab session marker + local IDB content → skip GET — no spurious conflict after alt-tab reload. |
| **Autosave** | **Debounced autosave** overwrites a single server **draft** on the mission. **Undo** = in-session. Manual **Save Version** creates semver snapshots for future Visual-Git/history. |
| **Time of day** | Match **Arma 3 Eden** environment control (slider/scrub in environment UI — not preset-only dropdowns). Expose quick readout in top bar; fine control in Mission Settings. |
| **Mission create entry** | **No standalone `/missions/create` route or sidebar tab** (T-048). `mission_maker+` creates from **New Mission** header (tooltip **⌘N/Ctrl+N**), **My Missions true-empty CTA** (no filters active), or **Cmd/Ctrl+N**. Close dossier Sheet before opening dialog. Form resets on every dialog close. Editor surfaces keep **Mission Creator** naming. |
| **Numeric transform** (T-049) | Editable X/Y/Z/rotation lives in the **Attributes modal Transform tab** (commit on blur/Enter, one undo step). The **bottom toolbelt is readout-only** — selection-aware (single slot → SEL X/Y/Z; else CUR cursor X/Y/Z). x/y clamp to terrain bounds; Z is manual until DEM. |
| **Cursor readout** (T-050) | The toolbelt **CUR** mode shows cursor **X/Y/Z**; **Z = 0** on the flat map (a real ground-plane value, not `—`), carrying real elevation once Phase 2 DEM feeds z. Off-map hover → `—` on all axes. |
| **Undo keyboard** (T-052) | Keyboard undo/redo lives in the `MissionCreatorPage` host keydown handler and **reuses the existing `UndoController`** (no second stack): **Cmd/Ctrl+Z** undo, **Cmd/Ctrl+Shift+Z** or **Ctrl+Y** redo. Skipped while focus is in `INPUT`/`SELECT`/`TEXTAREA`/contentEditable (same guard as Space/Delete); `preventDefault` on match, drives the stack only when `canUndo()`/`canRedo()`. Toolbar buttons unchanged. **`useMissionDoc` StrictMode fix:** one-shot `instanceKey` bump on teardown so dev `<StrictMode>` gets a live `UndoController` (without it, undo was permanently dead in dev). Undo = **session edits only** (`LOCAL_ORIGIN`); IndexedDB/server hydrate not undoable. |
| **Additive select** (T-053) | **Ctrl/Cmd-only** modifier multi-select lives in `TacticalMap`'s Deck `onClick` (reads `event.srcEvent.ctrlKey/metaKey`). Click a slot with the modifier → **toggle** it in/out of `selection.ids` (removing the last id → `none`); Ctrl/Cmd + empty-click **preserves** the selection (only a plain empty click deselects). **Shift stays unbound** (reserved for a future range-select); marquee still **replaces**. One-file change — no store/schema or `useSelectTool` change; a Ctrl-built multi (>1) keeps dbl-click attributes suppressed. |
| **Copy/paste at cursor** (T-056) | **Ctrl/Cmd+C** snapshots the slot selection to an in-editor clipboard (`ClipboardSlot[]` ref on `MissionCreatorPage`); **Ctrl/Cmd+V** pastes via new batched `pasteSlots(md, clip, { anchorAt, layerId })` in `state/ydoc.ts` — one transact (one undo step) that translates the clip so its **centroid lands at the map cursor** (mouse off-map → fixed **+20m/+20m** nudge), re-attaches each copy to its **source squad** (or `ensureDefaultSquad`), files into the **active layer** (or `ensureDefaultLayer`), clamps x/y to terrain bounds, and returns the new ids → selection. Two keydown branches behind the existing INPUT/SELECT/TEXTAREA/contentEditable guard (native text copy/paste preserved); cursor read via `cursorRef` (no keydown re-bind on mouse move). **Scope locked:** copy+paste, slots only — Cut (Ctrl+X) and paste-at-original (Ctrl+Shift+V) deferred. Four files; no backend/`useSelectTool`/compiler change. Closes gap_analysis **P1-02** / ACTION-COPY-001 / ACTION-PASTE-001. |
| **Asset browser search** (T-055) | The **Asset Browser** (Factions tab in the right palette) gets a search field over a recursive `filterCatalog(ASSET_CATALOG, q)` — **case-insensitive label substring**; a folder is kept on a self-match (→ full subtree, so "nato" shows all NATO) or on any descendant match (→ filtered children); retained folders force-`defaultExpanded`. The `TreeView` is **keyed on the query** so its mount-time `collectExpanded` re-runs and reveals matches; empty result → "No assets match"; X/Esc clears; filtered leaves still drag-to-place. Search is **scoped to AssetBrowser** (only live catalog) — stub tabs unchanged; no `class:` prefix (P2). One real file; no `TreeView`/`ASSET_CATALOG`/store change. Closes gap_analysis **P1-04** / RIGHT-SEARCH-001. |
| **Attributes entry points** (T-054) | One double-click contract for opening **Attributes**. **Map:** native `onDoubleClick` on the gesture-host container + `deckRef.pickObject('slot-icons')` → `onEntityActivate` — replaced the hand-rolled 350ms `lastClick` timer in `onClick`. **ORBAT tree:** `OrbatSection` gains `onActivateSlot` (threaded from `LeftSidebar`, same as `EditorLayersSection`) and passes `onActivate` to its `TreeView`, which fires on a slot row's native `onDoubleClick`. Multi-select suppression (`onEntityActivate` `ids.length <= 1`) and T-053 Ctrl/Cmd toggle unchanged; no TreeView/store change. Closes gap_analysis **P1-09** / SEL-ORBAT-DBL-001 (hardens SEL-MAP-004). |
| **Map performance** (T-057) | The toolbelt cursor read-out is **transient `useMapStore.cursor`** (set rAF-throttled), not page state — so a pointer move re-renders only `BottomToolbelt`, never the Outliner trees. Cursor coords come from **unprojecting the mouse ourselves** (`view.makeViewport(...).unproject` on the container `onPointerMove`), **not** Deck's `onHover` — `onHover` is removed and `getCursor` is constant `'crosshair'`, so Deck does **no per-move hover pick**. Picking is kept only for click / dbl-click / marquee / drag-start. Pan is **rAF-coalesced** in `useSelectTool` (one `setViewState`/frame, flushed on pointer-up). `React.memo` on `TacticalMap`, `LeftSidebar`, `AssetPalette`, `TopCommandStrip`, `BottomToolbelt`, `AttributesModal`. **Accepted UX trade:** the pointer no longer changes to a "pointer" glyph over an icon (no hover pick). No schema/compiler/backend change; all interactions unchanged. Spec: [`t057_map_performance_hotfix.md`](t057_map_performance_hotfix.md). |
| **Entity count readout** (T-058) | Bottom toolbelt shows **OBJ** = `slotCount` from store (T-062; was memoized `selectSlotCount`) + **SEL** = `selection.ids.length` when `kind==='slot'` else 0, in a mono `tabular-nums` block right of the X/Y/Z coords. Both subscribe **inside the already-memoized `BottomToolbelt`** so they track add/remove/paste/delete/selection but **never** a cursor move (T-057 channel untouched). Slots only — vehicles/markers join in a later P0 slice; plain integers (no commas) so 100000+ doesn't break layout. Closes `BOTTOM-OBJCOUNT-001`. |
| **Mission version API body limit** (T-060 — **code shipped**) | **Was:** global 1 MB rejected 360k payloads. **Fix (T-060 code):** `internal/middleware/bodylimit.go` — `GlobalBodyLimit` skips versions POST; route `BodyLimit(256 MB)`; **413** in `CreateVersion`. **Upload @ scale (T-060.1):** version POST `timeout: 600_000` + `maxBody/maxContentLength: Infinity`; Vite `/api` proxy `timeout`/`proxyTimeout: 600_000`; chunked `editor.slots` assembly; `!resp` catch surfaces axios `code`/`message`. |
| **Load gate + save progress** (T-060 **shipped** `b1fd25a`) | **Load:** four-phase overlay; partial pass @ 360k. **Save:** @ ~367k / ~142 MB → **201** (browser + curl 140 MB). Mid-upload reset fixed — 1 MB global cap on stale API; `isMissionVersionPOST` + production-like IT. Spec: [`t060_1_scale_load_save_completion.md`](t060_1_scale_load_save_completion.md). |
| **Save mid-upload @ 135 MB** (T-060.1.4) | **Proven root cause:** stale `go run` API let 1 MB `GlobalBodyLimit` wrap the version POST. **Fix shipped:** `isMissionVersionPOST`, `setupITProd`, `bodylimit_test.go`, `phaseAtFailure`, `scripts/mission-version-upload-repro.sh`. **Ops:** restart `make api` after middleware changes. |
| **Dual-layer scale model** (2026-06) | **Mission layer** (ORBAT slots, markers — Y.Doc, **T-061..T-062**) = authored entities. **Terrain layer** (millions of map props) → **T-070+** binary base + sparse deltas; **not** a Y.Doc rewrite. External Base+Delta adopted for terrain only. Spec: [`t070_terrain_base_mission_layers.md`](t070_terrain_base_mission_layers.md). |
| **Bulk paste at scale** (T-059) | `pasteSlots` batch O(n) append (`Map<squadId, ID[]>` + layer accumulator); post-paste selection cap (`BULK_SELECT_CAP = 500` → `none`); outliner leaf cap (`OUTLINER_LEAF_CAP = 500`) in **both** `EditorLayersSection.buildTree` and `OrbatSection.buildOrbat`. Chunked paste not needed. **Validated:** 6k paste loops smooth; **360k @ 100+ fps** pan. Spec: [`t059_bulk_paste_operations.md`](t059_bulk_paste_operations.md). |
| **Drag-move @ 360k** (T-061 — **shipped, good enough**) | **T-061.0:** dual IconLayer + split drag state + rAF delta → ~60 fps sustained. **T-061.0.1:** `slotIconCache` O(k) + bindings slot fast path → pickup/release materially improved (minor release frame possible — deferred). Mega opts → [ROADMAP.md](ROADMAP.md) §Deferred mega optimizations. Spec: [`t061_drag_move_hotfix.md`](t061_drag_move_hotfix.md). |
| **Incremental bindings @ 360k** (T-062 — **shipped**) | **T-062.0:** `classifyTransaction` → O(k) Zustand patches (drop, delete, meta, editor-layers). **T-062.0.1:** batched `removeEntities`, `slotCount`/`slotsRevision`, `REMOVE_PATCH_CAP` 10k. Verified delete 4k + undo 6k @ ~360k. Spec: [`t062_incremental_bindings.md`](t062_incremental_bindings.md). |
| **Chunked IDB slot restore** (T-062.1 — **shipped**) | v2 `idb` persistence; determinate restoring @ ~360k. Spec: [`t062_1_idb_streaming_load.md`](t062_1_idb_streaming_load.md). |
| **Save orbat dedup** (T-062.1.1 — **shipped**) | Save omits duplicate `orbat[]`; `services.ParseOrbatTemplate` derives from `editor` for Event attach. Export keeps full orbat. Spec: [`t062_1_1_batch_save.md`](t062_1_1_batch_save.md). **Next:** T-063 spatial index. |
| **Editor session / alt-tab** (T-062.2 — **shipped**) | Dev: `viteReloadGuard` blocks Vite HMR full reload on editor route. Warm session: `editorSession.ts` → skip multi-MB GET on same-tab return when IDB has content. Background-safe yields. **Tradeoff:** warm path trusts local IDB. Spec: [`t062_2_editor_session_persistence.md`](t062_2_editor_session_persistence.md). |
| **Eden-first program order** (2026-06) | … **Exception:** **T-057..T-067** perf/scale program runs first (**T-062.1.1 shipped**; **active T-063..T-067**). Eden **T-068+**; **T-070+** terrain base after that. … |
| **Mission title hydrate** (T-049) | On editor load the **PostgreSQL mission row** (`title`, `terrain`, time/weather) hydrates `meta` via `applyMissionRowMeta` (INIT_ORIGIN) — including new missions whose `json_payload` is `{}`. **No PATCH-back** in T-049; Save Version still compiles payload only. |
| **Phase order** | … **T-057–T-062.1.1 shipped.** **Active: T-063..T-067** → Eden **T-068+** → **T-070+** terrain base (optional). … |
| **Drag perf — good enough** (2026-06) | T-061 closed Eden-blocking drag @ ~360k. T-062 closed everyday edit bindings @ ~360k. Do **not** pursue T-061.1 / release repack collapse until T-063..T-067 + Eden milestones unless regression. See ROADMAP §Deferred mega optimizations. |
| **Eden completeness** | Eden parity checklist = `eden/interactions.md`, `eden/ui_anatomy.md`, `eden/attributes.md`, `eden/gap_analysis.md` + scrape artifacts. Read `eden/ui_anatomy.md` / `eden/attributes.md` before implementing UI/attrs. Implement the P0 backlog from `eden/gap_analysis.md`. Feature status lives in `feature_inventory.md` + `reference/feds_schema.md`; new TBD features → FEDS row in `feature_inventory.md`. Wiki cache = `eden/wiki_manifest.yaml` + `artifacts/eden-wiki/`; regenerate via `node scripts/tools/scrape-eden-wiki.mjs` when the wiki updates. |

---

## Agent rules (mandatory)

1. **Read first:** `CLAUDE.md` (conventions), then this file, then `engineering_plan.md` §0–§2.
2. **Start at `ROADMAP.md` §Current strategy + §Map performance:** **T-062.1.1 shipped.** **Active: T-063** spatial index → T-064..T-067. Eden **T-068+** after scale milestones.
3. **Verify gate** after every phase:
   ```bash
   cd frontend && npm run build && npm run lint
   ```
4. **Do not commit** unless the user explicitly asks.
5. **Eden-first deferrals:** do **not** start Phase 2 (map tiles/DEM), full Phases 5–6 (registry completeness / Arsenal), or Phase 8 (tools) without user approval — these wait until after Eden P0–P2. **Exception:** a **thin Track B registry (B-01)** — classname, displayName, category, iconUrl — is **in scope** when needed to unblock Eden P0 (P0-01..03); keep it minimal, not full registry completeness.
6. **Visual target:** Arma 3 Eden Editor + Aegis tokens. **Never** derive layout from `code.html` / `screen.png` mockups — use Decisions log + this plan only.
7. **State rule:** Entity mutations go through `tactical-map/state/ydoc.ts` → `bindings.ts` → `useMapStore`. Never set entity data directly on Zustand.
8. **Inspector rule:** Asset Palette stays on the right always. Properties edit via **AttributesModal on double-click only** — no right-panel inspector swap.
9. **Move rule:** Click-drag icons on the map to move (Phase 7b). Remove click-empty-map-to-teleport. Marquee box-select on left-drag empty map; middle-mouse/right-drag pans.
10. **Camera rule:** Spacebar centers on current selection. No automatic flyTo on single-click (map or outliner).
11. **Delete rule:** Delete/Backspace removes selected entities in one undoable transaction.
12. **Fullscreen rule:** Hide platform Sidebar + TopNav on the editor route.

---

## ACTIVE SLICE — T-063 spatial index (then T-064..T-067)

**T-060 + T-060.1.* shipped** in commit `b1fd25a`. **T-061 shipped (good enough)** — spec: [`t061_drag_move_hotfix.md`](t061_drag_move_hotfix.md). **T-062 shipped** — spec: [`t062_incremental_bindings.md`](t062_incremental_bindings.md). **T-062.2 shipped** — spec: [`t062_2_editor_session_persistence.md`](t062_2_editor_session_persistence.md). **T-062.1 shipped** — spec: [`t062_1_idb_streaming_load.md`](t062_1_idb_streaming_load.md). **T-062.1.1 shipped** — Save orbat dedup; spec: [`t062_1_1_batch_save.md`](t062_1_1_batch_save.md).

**T-062.1.1 acceptance (shipped):** `make test-it` + services unit tests + FE build/lint clean; IT editor-only version → event attach → ORBAT derived (roles/tags/order). Manual Save @ ~360k size reduction: pending.

**Next:** **T-063** rbush spatial index → T-064..T-067. **Eden T-068+** after scale milestones.

---

## Execution checklist (historical — shell phases complete)

### STEP 0 — Publish plan ✓
- [x] `Design_Docs/Mission_Creator_Architecture/agent_execution.md` is in the repo

### PHASE PRE-3.5 — Land tree wiring (**historical — done T-033**)

> **Completed.** Outliner bound to Y.Doc, asset drag→map, `placedEntitiesMock` deleted. See phase completion log (T-033–T-040).

---

### PHASE DOC-0 — Documentation alignment
**Goal:** Docs agree on Eden layout + interactions before more code.

**Tasks:**
1. ~~Create `ux_spec.md`~~ — **Done (T-034)**
2. Update `engineering_plan.md`: docked shell, fullscreen, phases PRE-3.5/3.5/7b/7a, `EditorLayer`, multi-select `Selection`, point UX authority to this file
3. Update `mission_creator_design.md` §1: Attributes dialog; palette always visible; note HTML mockups are historical
4. Update `CLAUDE.md`: T-033, current phase status, uncommitted wiring note
5. Document `AppLayout` fullscreen escape for editor route

**Done when:** All four files updated; no code changes required.

---

### PHASE 3.5 — Eden shell fidelity
**Goal:** Editor shell matches **Arma 3 Eden layout** (Aegis glass skin). Includes fullscreen chrome + Spacebar camera.

**Layout target:**
```
┌─────────────────────────────────────────────────────────────┐
│ TopCommandStrip (h-12) — NO platform TopNav/Sidebar          │
├──────────┬──────────────────────────────────────┬───────────┤
│ Left     │         TacticalMap                  │ Right     │
│ w-64     │         ml-64 mr-80                  │ w-80      │
│ ORBAT +  │         topo + grid overlay          │ Asset     │
│ Layers   │         [BottomToolbelt in map area] │ Palette   │
└──────────┴──────────────────────────────────────┴───────────┘
```

**Key files:** `MissionCreatorPage.tsx`, `AppLayout.tsx` or editor wrapper, `TopCommandStrip.tsx`, `LeftOutliner/` (→ `LeftSidebar.tsx`), `RightInspector/AssetBrowser.tsx`, `AttributesModal.tsx`, `overlay.ts`, `BottomToolbelt.tsx`, `router.tsx` (fullscreen handle)

**Tasks:**
0. **Fullscreen:** Hide platform `Sidebar` + `TopNav` on `/missions/:id/edit`
1. **Layout:** Docked left `w-64` + right `w-80` flush; map `ml-64 mr-80`; top bar full width
2. **Top bar:** Mission title (inline edit); menu stubs (File/Edit/View/Mission/Environment); **Eden time slider/scrub** + weather wired to `updateEnvironment`; undo/redo; Export (still disabled until Phase 9); settings gear → `MissionSettingsDialog` (view distance, thermals)
3. **Left sidebar — both sections in one scroll:**
   - **ORBAT** (top): `factions` → `squads` → `slots` (export truth; read-only OK if no ORBAT UI yet)
   - **Editor Layers** (below): `editorLayers` workflow folders (current outliner)
   - **Stubs:** Waypoints, Zones, Logic & Events (empty until Phase 8)
   - **Bottom icon tabs:** Hierarchy, Layers, Assets, History, Settings (stubs switch content later)
   - Header: OUTLINER + mission name + New folder
4. **Right asset palette (always visible):**
   - Tabs: Factions | Vehicles | Markers | Objectives
   - Pattern: 2-col **grid cards** at tab top level → drill-down **tree** (Men → Rifleman)
   - Keep `ASSET_DND_MIME` drag onto map; mock data OK until registry API
   - Remove `InspectorPanel` → `SlotInspector` swap entirely
5. **AttributesModal** (double-click only) — migrate `SlotInspector` fields:
   - **Transform:** X/Y/Z, rotation (Z read-only until DEM)
   - **Identity:** role, tag, callsign, squad
   - **States:** medic/engineer flags (stub)
   - **Arsenal:** Open Loadout Forge button (stub until Phase 6)
6. **Map skin:** Topo placeholder under Deck.gl + procedural grid at low opacity
7. **Spacebar** → `flyTo` selection centroid; remove auto `flyTo` on outliner click
8. **TreeView polish:** `border-l-2 border-primary` on selected row; folder open/closed icons

**Acceptance:** All boxes under **Phase 3.5 — Eden shell** in [Acceptance criteria](#acceptance-criteria) below.

**Verify:** `npm run build && npm run lint`

---

### PHASE 7b — Map drag & multi-select
**Goal:** Eden manipulation — grab icons on the map; marquee select; group move.

**Problem today:** `MissionCreatorPage` `onMapClick` → `moveEntity` requires click-then-click. Selection is single `{ kind, id }`.

**Key files:** `TacticalMap.tsx`, `tools/useSelectTool.ts` (create), `layers/useIconLayer.ts`, `layers/useSelectionLayer.ts` (create), `state/schema.ts`, `state/useMapStore.ts`, `state/ydoc.ts`, `state/selectors.ts`, `MissionCreatorPage.tsx`

**Tasks:**
1. **Schema:** `Selection` → `{ kind, ids: ID[] }`; update store, selectors, icon highlights, outliner multi-highlight
2. **Drag-move:** pointer down on icon → transient preview (do **not** write Y.Doc every frame); pointer up → one `transact()` / one undo step
3. **`moveEntities(md, ids, delta)`** in `ydoc.ts` — atomic group move
4. **Marquee:** left-drag on empty map draws selection box (`useSelectionLayer`); middle-mouse / right-drag pans
5. **Controller:** disable Deck pan while dragging entities; disable left-drag pan (marquee replaces it)
6. Remove `onMapClick` teleport path entirely
7. **Delete/Backspace** → batch `removeEntity`, undoable
8. **Spacebar** → `flyTo` centroid of `selection.ids`

**AttributesModal rule:** double-click opens modal only when **one** entity selected; multi-select shows count or disables modal.

**Acceptance:** All boxes under **Phase 7b — Map manipulation** in [Acceptance criteria](#acceptance-criteria) below.

**Verify:** `npm run build && npm run lint`

---

### PHASE 7a — Outliner tree operations
**Goal:** Eden left-tree workflow — reparent, rename, delete.

**Key files:** `OutlinerPanel.tsx` / `LeftSidebar.tsx`, `TreeView.tsx`, `ydoc.ts` (add rename/delete layer actions if missing)

**Tasks:**
1. Outliner reparent DnD between `editorLayers` folders
2. Folder rename + delete UI
3. Delete slot from outliner (wire `removeEntity`)
4. Wire `assetId` from palette payload into slot metadata

**Verify:** `npm run build && npm run lint`

---

### PHASE 9 — Compiler + persistence
**Goal:** Export `json_payload` and autosave to backend.

**JSON contract (Ultra Plan §8 — non-negotiable):** Output must be a **superset** containing existing `orbat[]` shape for `parseOrbatTemplate` in `internal/handlers/events.go`, plus `map`, `environment`, `loadouts`, `objectives`, `vehicles`, `markers`, `schemaVersion`. Separate camelCase export via `exportSchema.ts` for Arma mod.

**API (already exists):** `POST /api/v1/missions/:id/versions` (draft autosave / Save Version), `GET .../versions/:vid` (hydrate). On IndexedDB vs API conflict → **user prompt**.

**Key files:** `compiler/compile.ts`, `compiler/exportSchema.ts`, `compiler/compiler.worker.ts`, `hooks/useMissionEditor.ts`, `TopCommandStrip.tsx`

**Tasks:**
1. `compile.ts` traverses normalized state → `orbat[]` superset
2. Enable Export → download JSON
3. `useMissionEditor`: hydrate on load; debounced draft autosave; manual Save Version → new semver
4. Unsaved-changes indicator
5. Visual-Git scrubber stub in top bar (full UI deferred)

**Verify:** `npm run build && npm run lint` + dev-login smoke on `/missions/:id/edit`

---

### DEFERRED — Do not start without user approval

| Phase | Blocker / notes |
|-------|-----------------|
| 2 DEM / Z-axis | Hosted 16-bit heightmaps + topo tiles; `dem/*`, `useDemLayer.ts` |
| 5–6 Registry + Arsenal | `GET /api/v1/registry`; `registry.worker.ts`, `ArsenalInspector`, paper-doll |
| 8 Tools + objectives | Ruler, LoS GLSL, zones, `useLineLayer`, `usePolygonLayer`; needs DEM for LoS |
| Product (future) | Visual-Git diff ghosts, Mission Planner, in-game Briefing UI, multiplayer y-websocket — see `mission_creator_design.md` |

---

## Do not break (preserve these)

- **Deck.gl + Y.Doc architecture** — Eden is a shell on top; never per-entity DOM on the map
- **Y.Doc mutation path** — `ydoc.ts` `transact()` only; one user gesture = one undo step
- **Palette → map placement** — `ASSET_DND_MIME` + `addSlot` flow
- **`editorLayers`** — workflow folders; export uses factions/squads/slots not layers
- **Undo/redo** — `Y.UndoManager` with `LOCAL_ORIGIN`; `useMissionDoc` must keep a **live** `UndoController` after React StrictMode teardown (`instanceKey` bump)
- **Lazy route** — `/missions/:id/edit` code-split; `mission_maker+` gate
- **IndexedDB** — local durability via `useMissionDoc` even after API lands

---

## Acceptance criteria

### Phase 3.5 — Eden shell

- [ ] Left sidebar docked flush left (`w-64`); right palette docked flush right (`w-80`); map between them
- [ ] **No** platform Sidebar/TopNav on `/missions/:id/edit`
- [ ] Right Asset Palette always visible with tabs (Factions / Vehicles / Markers / Objectives)
- [ ] Double-click opens Attributes modal with editable fields (role, tag, stance at minimum)
- [ ] Time control matches Eden (slider/scrub — not preset-only dropdowns)
- [ ] Map has topo appearance (placeholder OK) + grid overlay
- [ ] Left panel shows **both** ORBAT section and Editor Layers section
- [ ] Spacebar centers on selection (no auto-center on click)
- [ ] Bottom toolbelt shows X/Y/Z in mono, centered in map area

### Phase 7b — Map manipulation

- [ ] Click-drag a placed unit to move it (no second click on the map)
- [ ] Marquee box-select on left-drag empty map
- [ ] Middle-mouse / right-drag pans the map
- [ ] Spacebar centers camera on selection
- [ ] Delete removes selection; undo restores
- [ ] Group move is a single undo step
- [ ] Clicking empty map only deselects

---

## How to run this plan

1. Start a new Agent session in this repo.
2. Paste the [one-line prompt](#one-line-prompt-copy-this) from the top of this file.
3. Shell phases PRE-3.5–9 are DONE (T-033–T-040) — open [`ROADMAP.md`](ROADMAP.md) and execute only OPEN items.
4. To resume a specific shell phase for reference: `Continue agent_execution.md from PHASE 7b`.
5. To commit after a phase passes verification: `commit with tag T-033`.

**Agent reminder:** Read **Document hierarchy** → **Decisions log** → **Architecture state** before code. Use **Interaction contract** for behavior. Ultra Plan §8 for compiler. HTML mockups are historical only.
