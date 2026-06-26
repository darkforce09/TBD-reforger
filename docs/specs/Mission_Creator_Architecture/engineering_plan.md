# TBD Reforger — Mission Creator: The Engineering Ultra Plan
**Document:** `engineering_plan.md`
**Status:** Authoritative engineering blueprint (supersedes the renderer/stack choices in `problem_statement.md`)
**Audience:** Implementing engineers. Read `problem_statement.md` (problem statement) and `mission_creator_design.md` (product/UX) first; this document is *how we build it, file by file.*

> **UX/layout authority is `agent_execution.md` + `ux_spec.md`.**
> The locked UX is the **Arma 3 Eden Editor docked shell** (fullscreen, panels flush to the
> edges, always-on Asset Palette, Attributes modal on double-click). Where §5 of this file still
> describes earlier explorations (floating panels, a right-panel inspector swap, a "popover"
> inspector), the Eden Decisions log in 05/04 **wins**. This file remains authoritative for the
> data model, workers, compiler, and DEM.

---

## 0. Architecture Decision Record (Read This First)

This plan makes three binding decisions that override earlier documents. They are
recorded here so nothing downstream contradicts.

| # | Decision | Supersedes | Rationale |
|---|----------|-----------|-----------|
| ADR-1 | **Renderer = Deck.gl** (`@deck.gl/core`, `/layers`, `/react`, `@luma.gl/core`) | `problem_statement.md` Pillar 1 (PixiJS/React Konva) | Deck.gl gives us a batched, GPU-driven layer system, free picking, and a clean path to custom GLSL layers (DEM hillshade, viewshed) without hand-rolling a scene graph. |
| ADR-2 | **Structure = in-app feature module + lazy route** inside the existing **Vite + React 19** SPA. The engine is `frontend/src/features/tactical-map/`; the editor mounts at `/missions/:id/edit`. | The brief's `packages/tactical-map` monorepo + isolated Next.js wrapper | The repo is a single-package Vite app today. The editor is a 100% client-side WebGL surface — Next.js SSR/RSC add nothing and a monorepo split adds tooling cost. We get isolation via a code-split route + a self-contained feature folder, and reuse the Aegis design system and toolchain for free. |
| ADR-3 | **Multiplayer = single-player v1, Yjs-ready.** The normalized store is `Y.Doc`-backed and offline-first from day one, but v1 ships solo. The y-websocket sync server, presence cursors, and live-conflict UX are **deferred** (Phase 4+ of the product roadmap) and documented as forward-looking only. | — | De-risks the v1 delivery of an enormously complex editor while guaranteeing the data model is correct for collaboration later — no rewrite required to turn multiplayer on. |

### New runtime dependencies (introduced when build begins — not part of this doc)
```
deck.gl @deck.gl/core @deck.gl/layers @deck.gl/react @luma.gl/core   # rendering
yjs y-indexeddb                                                       # CRDT store + offline durability
comlink                                                              # ergonomic Web Worker RPC
idb                                                                  # typed IndexedDB wrapper
# y-websocket  -> DEFERRED until multiplayer (ADR-3)
```
All already-present deps are reused: `zustand`, `@tanstack/react-query`, `@base-ui/react`,
`lucide-react`, `sonner`, `tailwindcss` v4, `clsx`/`tailwind-merge`.

### Backend prerequisites (dependencies — out of scope for this doc, must be confirmed/built separately)
1. **`GET /api/v1/registry?modpack=<version>`** — the Master Item Registry feed (every valid
   classname, category, slot-compatibility, attachment-compatibility, 2D icon URL). The design
   blueprint specifies the game server pushes modded gear into this DB via the REST API; the
   editor only *reads* it. Must return a stable `ETag` / version.
2. **Mission-version persistence** — an endpoint to create/update a `MissionVersion` with a new
   `json_payload` (the editor's autosave + "Save" target). Confirm exact route/verb against
   `internal/handlers/missions.go`.
3. **Asset hosting** — top-down map tiles (Everon/Arland) and the 16-bit DEM heightmap for each
   terrain, served as static assets the engine can `fetch`. **Tickets:** **T-090** (aligned map tiles),
   **T-091** (DEM + Z-axis on place/move). Blocked on hosted binaries — do not start without approval
   ([`agent_execution.md`](agent_execution.md) §Deferrals).

---

## 1. Exact Folder & File Structure

Everything lives under `frontend/src/`. Two cooperating modules: the **engine**
(`tactical-map`, terrain-agnostic, reusable by the future Mission Planner) and the
**Creator wrapper** (`mission-creator`, the mission-authoring app shell).

### 1.1 The engine — `features/tactical-map/`
```
features/tactical-map/
├── index.ts                       # public barrel — the engine's only import surface
├── TacticalMap.tsx                # <DeckGL> root; owns view state; assembles layer array
├── README.md                      # engine contract + how the Planner will consume it
├── types.ts                       # engine-public types (props, tool ids, view state)
├── context/
│   └── MapContext.tsx             # React context: deck ref, coord transforms, flyTo()
├── view/
│   └── useOrthographicView.ts     # OrthographicView config; zoom/pan clamps; 60fps tuning
├── coords/
│   ├── projection.ts              # worldToPixel / pixelToWorld (Arma meters <-> canvas)
│   └── terrains.ts                # per-terrain world bounds, scale, DEM max-elevation
├── dem/
│   ├── DemTexture.ts              # load 16-bit heightmap -> luma Texture + CPU Float32Array
│   ├── sampleElevation.ts         # (x,y) -> meters, CPU O(1) read of the Float32Array
│   └── DemController.ts           # async load, optional tiling, fallback state
├── layers/
│   ├── useBaseMapLayer.ts         # TileLayer (tiled) or BitmapLayer (single image)
│   ├── useDemLayer.ts             # hillshade/contour overlay from the DEM DataTexture
│   ├── useIconLayer.ts            # NATO icons: units, vehicles, waypoints (data-driven)
│   ├── useLineLayer.ts            # PathLayer: phase lines, arrows, ruler segments
│   ├── usePolygonLayer.ts         # PolygonLayer: safe-start areas, objective radii
│   ├── useViewshedLayer.ts        # CUSTOM Layer + GLSL: line-of-sight raster
│   └── useSelectionLayer.ts       # marquee box + selected-entity highlight ring
├── tools/
│   ├── ToolContext.tsx            # active-tool state machine
│   ├── useSelectTool.ts
│   ├── useRuler.ts                # slope-aware distance + walk-time
│   ├── useViewshedTool.ts         # observer placement -> viewshed layer params
│   └── useWaypointTool.ts
└── state/
    ├── ydoc.ts                    # creates Y.Doc + named Y.Maps (SOURCE OF TRUTH)
    ├── schema.ts                  # normalized entity interfaces (see §2)
    ├── useMapStore.ts             # Zustand read-mirror of the Y.Doc
    ├── bindings.ts                # observeDeep(Y.Doc) -> useMapStore.setState
    ├── undo.ts                    # Y.UndoManager wrapper (Visual-Git timeline)
    └── selectors.ts               # memoized derived selectors (per-layer data arrays)
```

### 1.2 The Creator wrapper — `features/mission-creator/`

> **Superseded (T-035):** Actual layout uses docked `LeftSidebar`/`AssetPalette`/`AttributesModal`.
> File tree below is historical — see live tree in [`agent_execution.md`](agent_execution.md).

```
features/mission-creator/
├── MissionCreatorPage.tsx         # full-bleed route shell (uses AppLayout fullBleed handle)
├── routes.ts                      # React.lazy registration -> /missions/:id/edit (mission_maker+)
├── hooks/
│   └── useMissionEditor.ts        # GET mission version -> hydrate Y.Doc; debounced autosave
├── layout/
│   ├── TopCommandStrip.tsx        # title • Visual-Git scrubber • env controls • Export
│   ├── BottomToolbelt.tsx         # tool buttons + live X/Y/Z readout (JetBrains Mono)
│   ├── LeftOutliner/
│   │   ├── OutlinerPanel.tsx      # Miller-columns container
│   │   ├── FactionColumn.tsx
│   │   ├── SquadColumn.tsx
│   │   ├── SlotColumn.tsx
│   │   └── OutlinerRow.tsx
│   └── RightInspector/
│       ├── InspectorPanel.tsx     # switches on selection.kind
│       ├── GlobalSettingsInspector.tsx
│       ├── ObjectiveInspector.tsx
│       └── ArsenalInspector/
│           ├── ArsenalInspector.tsx
│           ├── SoldierDoll.tsx    # 2D paper-doll (helmet/vest/uniform/weapon/backpack)
│           ├── SlotCategory.tsx   # one collapsible gear category
│           └── ItemPicker.tsx     # registry-backed search/grid
├── arsenal/
│   └── useArsenalValidation.ts    # bridges drops to the registry worker (canEquip/canAttach)
├── registry/
│   ├── registry.worker.ts         # Web Worker: Master Item Registry (see §3)
│   ├── registryClient.ts          # Comlink proxy + IndexedDB cache orchestration
│   └── registryTypes.ts
├── objectives/
│   ├── ObjectiveLayerAdapter.ts   # maps objectivesById -> polygon layer data
│   └── triggers.ts                # trigger/condition catalog + defaults
└── compiler/
    ├── compiler.worker.ts         # Web Worker: normalized state -> json_payload
    ├── compile.ts                 # pure traversal logic (unit-testable)
    └── exportSchema.ts            # camelCase mod-export shape for /missions/:id/export
```

### 1.3 Wiring into the existing app
- Register the lazy route in the app router behind the `mission_maker` role gate (mirror the
  existing `RequireMinRole` pattern / `AdminGate` used elsewhere). Apply the `fullBleed` route
  handle so `AppLayout` runs the page full-height (same mechanism T-011 introduced).
- **Fullscreen chrome escape (Phase 3.5, locked in 05/04):** the editor renders **without** the
  platform `Sidebar` + `TopNav`. `AppLayout`'s `fullBleed` handle only changes `<main>` padding —
  it does **not** hide chrome. Add a stronger escape: an `editorFullscreen` route handle that
  suppresses `<Sidebar/>` + `<TopNav/>`, or mount `/missions/:id/edit` outside the `AppLayout`
  `<Outlet/>`. `/missions/:id/edit` owns the entire viewport.
- Point the editor entry in `pages/missions.tsx` and the **"Open in Mission Planner"** stub on
  the Event Hub (`pages/events.tsx`) at `/missions/:id/edit`.
- Vite automatically code-splits the `React.lazy` route, so Deck.gl/Yjs/worker bundles never
  load for users who don't open the editor.

---

## 2. The Normalized State Model

**Principle (from `problem_statement.md` Pillar 3):** never nest. Every entity lives
in a flat, ID-keyed dictionary; relationships are ID arrays. This is what lets us swap one
optic without re-rendering a squad. The dictionaries are **`Y.Map`s inside a single `Y.Doc`**
(the source of truth); a Zustand store is a *read-mirror* updated by an `observeDeep`
subscription, so React components stay on the familiar `useMapStore(selector)` idiom (mirroring
`frontend/src/store/useAuthStore.ts`) while edits flow through Yjs and are automatically
undoable and (later) collaborative.

### 2.1 Entity interfaces — `state/schema.ts`
```ts
export type ID = string

export interface MissionMeta {
  id: ID
  title: string
  terrain: 'everon' | 'arland' | 'custom'
  customTerrainName?: string
  environment: {
    time: string                 // "HH:MM"
    weather: 'clear' | 'overcast' | 'heavy_rain' | 'dense_fog'
    viewDistance?: number        // meters; auto-derived but overridable
    thermals?: boolean
  }
}

export interface Faction {
  id: ID
  key: string                    // "BLUFOR" | "OPFOR" | "INDFOR" | "CIV" — export side key
  name: string                   // "US Army 2005"
  squadIds: ID[]
}

// Workflow-only Outliner folder (the Eden "Layers" / Left "Placed Entities" tree).
// Users nest arbitrary folders to organize entities; layers DO NOT affect the export
// (the compiler reads factions/squads/slots, not layers). null parentId = root.
// Shipped T-033; the tenth entity map.
export interface EditorLayer {
  id: ID
  name: string
  parentId: ID | null
  entityIds: ID[]                // slots/vehicles/markers filed directly in this folder
}

export interface Squad {
  id: ID
  factionId: ID
  callsign?: string              // "Platoon HQ"
  name: string                   // "Alpha 1-1"  -> exported as squad
  slotIds: ID[]
}

export interface Slot {
  id: ID
  squadId: ID
  index: number                  // 0-based authored order -> json_payload slot_index
  role: string                   // "Squad Leader"
  tag?: string                   // "MED" | "ENG"
  position: { x: number; y: number; z: number; rotation: number } // x/y meters, z from DEM
  stance: 'stand' | 'crouch' | 'prone'
  loadoutId: ID | null
}

export interface Loadout {
  id: ID
  containers: { uniform?: ID; vest?: ID; backpack?: ID; helmet?: ID } // -> InventoryItem ids
  weapons: { primary?: ID; secondary?: ID; launcher?: ID }           // -> InventoryItem ids
  itemIds: ID[]                  // loose items (map, first-aid kit, …)
  templateKey?: string           // set when applied from a mass-template
}

export interface InventoryItem {
  id: ID
  classname: string              // Arma classname; the registry key
  parentId: ID | null            // container nesting (a vest holding magazines)
  slotType: string               // 'uniform'|'vest'|'optic'|'muzzle'|'magazine'|'item'…
  attachments: Record<string, ID | null> // 'optic'|'muzzle'|'underbarrel' -> InventoryItem
  count: number                  // stack size (magazines, grenades)
}

export interface Trigger { type: 'presence'|'elimination'|'timer'; condition?: string }

export interface Objective {
  id: ID
  type: 'attack' | 'defend' | 'capture' | 'destroy'
  factionId: ID
  position: { x: number; y: number; z: number }
  radius: number                 // meters
  triggers: Trigger[]
  text?: string
}

export interface Vehicle {
  id: ID
  classname: string
  factionId: ID
  position: { x: number; y: number; z: number; rotation: number }
  inventoryItemIds: ID[]         // crate/cargo contents
}

export interface MapMarker {
  id: ID
  kind: 'line' | 'arrow' | 'phase' | 'icon' | 'polygon'
  points: [number, number][]     // world meters
  color: string
  label?: string
  authorId?: string              // for the Planner's per-user markers
}
```

### 2.2 The Zustand mirror — `state/useMapStore.ts`
```ts
export interface MapStoreState {
  meta: MissionMeta
  factionsById: Record<ID, Faction>
  squadsById:   Record<ID, Squad>
  slotsById:    Record<ID, Slot>
  loadoutsById: Record<ID, Loadout>
  itemsById:    Record<ID, InventoryItem>
  objectivesById: Record<ID, Objective>
  vehiclesById: Record<ID, Vehicle>
  markersById:  Record<ID, MapMarker>
  editorLayersById: Record<ID, EditorLayer>   // Outliner folders (T-033)
  factionOrder: ID[]

  // UI/runtime (not persisted to json_payload)
  // NOTE (Phase 7b): `selection` becomes multi-entity — `{ kind, ids: ID[] }` — to back
  // marquee box-select and group move. T-033 ships the single-id form below.
  selection: { kind: 'none'|'slot'|'squad'|'objective'|'vehicle'|'marker'; id: ID | null }
  activeLayerId: ID | null        // Outliner folder new entities are filed into (drop target)
  viewState: OrthographicViewState
  activeTool: ToolId
}
```
Components read via `useMapStore(s => s.slotsById[id])`. **Mutations never call `setState`
directly** — they call action helpers that write to the Y.Doc; `bindings.ts` reflects the
change back into the store.

### 2.3 The Y.Doc layout — `state/ydoc.ts`
- One `Y.Doc`. Top-level **`Y.Map`s** named `meta, factions, squads, slots, loadouts, items,
  objectives, vehicles, markers, editorLayers` (ten maps; `editorLayers` added T-033). Each is a
  `Y.Map<ID, Y.Map>` (an entity is a nested `Y.Map`).
- **Never use `Y.Array` of objects for entities** — ID-keyed `Y.Map`s make concurrent
  insert/delete/reparent commute, which is exactly what makes ADR-3's future multiplayer safe.
- All writes wrap in `ydoc.transact(fn, origin)` so one user gesture = one undo step.
- `state/undo.ts` wraps `Y.UndoManager` over those maps → powers undo/redo **and** the
  Top Bar "Visual-Git" timeline scrubber.
- `state/bindings.ts`: `ydoc` `observeDeep` → batched `useMapStore.setState`. Offline durability
  via **v2 chunked IDB** (`tbd-mission-persist`, T-062.1) — meta JSON + 5k slot chunks; legacy
  y-indexeddb replay + one-time migrate only. Debounced persist on `LOCAL_ORIGIN`; flush on tab hide.

---

## 3. The Web Worker Architecture — Master Item Registry

**Goal:** validate every drag/drop ("does this ACOG fit this MX?") with **0 ms main-thread
blocking**, over a registry of potentially thousands of items.

### 3.1 `registry.worker.ts`
- **On init:** open IndexedDB (`idb`), look up the cached registry by `modpackVersion`.
  - *Hit & fresh* (ETag matches) → load blob, build indices.
  - *Miss/stale* → `fetch('/api/v1/registry?modpack=<v>')`, persist the blob keyed by version, then build indices.
- **In-memory indices (worker heap only):**
  - `byClassname: Map<string, Item>` — O(1) lookup.
  - `byCategory: Map<string, Item[]>` — populate the ItemPicker.
  - `compatBySlot: Map<slotType, Set<classname>>` — what may go in a uniform/vest/optic slot.
  - `attachmentCompat: Map<weaponClass, { optic: Set; muzzle: Set; underbarrel: Set }>`.
- **Comlink-exposed API (all return structured-cloneable plain data):**
  ```ts
  query(filter): Item[]
  search(text, category?): Item[]
  getItem(classname): Item | null
  canEquip(itemClass, slotType, containerClass?): { ok: boolean; reason?: string }
  canAttach(weaponClass, attachClass, attachSlot): { ok: boolean; reason?: string }
  ```

### 3.2 `registryClient.ts` (main thread)
- Spawns the worker, wraps it in `Comlink.wrap`, exposes the same async API to React.
- The `ArsenalInspector` / `useArsenalValidation` call `canEquip`/`canAttach` on every drop;
  invalid drops are rejected with the returned `reason` (toast), valid drops mutate the Y.Doc.

### 3.3 Memory vs IndexedDB — the rule
| Tier | Holds | Why |
|------|-------|-----|
| **Worker memory (Maps)** | Hot indices | O(1) validation, zero main-thread cost, GC'd with the worker |
| **IndexedDB** | Raw registry blob, keyed by modpack version | Offline-durable source of truth; survives reloads; avoids re-downloading a large feed |
| **Zustand store** | *Nothing from the registry* | The registry is large and read-only; keeping it out of React state prevents accidental re-renders |

**Invalidation:** modpack version + HTTP ETag. **Failure mode:** if the fetch fails and no cache
exists, the worker enters *degraded mode* — placement is allowed, validation **warns instead of
blocks**, and a banner tells the user gear data is unavailable.

---

## 4. The Deck.gl Rendering Pipeline

### 4.1 View & coordinate system
- **`OrthographicView` + `COORDINATE_SYSTEM.CARTESIAN`.** Arma terrain is a flat local grid in
  meters — **no Web Mercator, no geographic projection.** Origin at bottom-left; +Y = Arma
  north. `coords/terrains.ts` holds each terrain's world bounds (e.g. Everon ≈ 12.8 km × 12.8 km)
  and max DEM elevation; `coords/projection.ts` converts world meters ↔ canvas pixels and feeds
  `flyTo()` (centering the map when the Outliner selects an entity).
- Zoom/pan clamps in `useOrthographicView.ts` keep the camera inside the terrain and hit 60 fps.

### 4.2 The DEM (Z-axis)
- `DemTexture.ts` loads the 16-bit grayscale heightmap **once**, producing **two** artifacts:
  1. a **luma.gl `Texture`** (GPU) consumed by the hillshade and viewshed shaders;
  2. a **CPU `Float32Array`** for `sampleElevation(x, y)` → O(1) cursor elevation readout
     **without per-frame GPU readback**.
- Mapping: black = 0 m, white = terrain max (from `terrains.ts`). Placed entities snap their
  `z` via `sampleElevation` so nothing spawns underground or floating.

### 4.3 Layer stack (bottom → top)
| # | Layer | Deck.gl type | Data source |
|---|-------|--------------|-------------|
| 1 | Base map | `TileLayer` (tiled) / `BitmapLayer` (single image) | terrain imagery |
| 2 | DEM hillshade / contours | **custom** DataTexture layer (toggleable) | `DemTexture` GPU texture |
| 3 | Areas & radii | `PolygonLayer` | safe-start areas, `objectivesById` radii (with Visual-Git red=deleted / green=added tinting) |
| 4 | Lines | `PathLayer` | `markersById` (phase lines, arrows), ruler segments |
| 5 | Icons | `IconLayer` (atlas, pixel-sized, pickable) | `slotsById`, `vehiclesById`, waypoint markers |
| 6 | Viewshed | **custom** `Layer` + GLSL | observer x/y/z → ray-march the DEM sampler → green visible / red blind |
| 7 | Selection | `PolygonLayer` (marquee) + highlight ring | `selection` + active marquee |

### 4.4 Layer management & 60 fps
- Each layer is produced by a `use*Layer` hook reading a **memoized selector** (`state/selectors.ts`).
- `TacticalMap.tsx` assembles the `layers` array; `updateTriggers` are keyed to the specific
  store slices a layer depends on, so editing one slot doesn't rebuild unrelated layers.
- Picking via Deck's `onClick`/`onHover` → sets `selection` and the live X/Y/Z readout.
- The frame budget holds because **Deck owns rendering** (React never renders per-entity DOM —
  this is the answer to the "200 Slot Problem") and **Y.Doc edits are batched in transactions**.
- **Viewshed** recomputes only when the observer moves (not per frame). GLSL sketch: for each
  fragment, march samples along the ray from observer to fragment, comparing interpolated terrain
  height (DEM sampler) against the line-of-sight height; if any sample occludes → red, else green.

---

## 5. The UI Component Hierarchy (with exact props)

Layout: **full-bleed map** behind a thin frosted **Top Command Strip**, a left **Outliner**, a
right **Inspector**, and a bottom **Toolbelt** — all glass panels built on the existing
`components/ui` primitives (`SplitPane`, `GlassPanel`, `Sheet`, `Dialog`, `Badge`, `Switch`) and
Aegis tokens (`--color-primary #adc6ff`, `.glass`, `.bg-topo-map`, `.bg-grid-overlay`).

### 5.1 Left Outliner — Placed Entities (Tree View)
```
OutlinerPanel
  TreeView (recursive)
    EditorLayerNode (Folder) → EditorLayerNode (Folder) → SlotNode
```
| Component | Key props |
|-----------|-----------|
| `OutlinerPanel` | `layers: EditorLayer[]`, `selection`, `onSelect(kind, id)`, `onReparent(childId, parentFolderId)` |
| `TreeNode` | `label`, `children`, `isExpanded`, `selected`, `onSelect`, drag/drop handlers |

- **Design:** Classic Eden Editor nested file-tree.
- **Custom Folders (Layers):** Users can create infinite arbitrary folders (e.g., "Assault Team", "Support", "BLUFOR") to organize entities. These folders are purely for workflow and do not affect the exported mission file. Default folders (BLUFOR, OPFOR) can be provided but are not rigidly enforced.
- Drag and drop works to reorganize entities between folders.
- Selecting any row calls `MapContext.flyTo(entity.position)` to center the map.

### 5.2 Right Panel — Asset Browser & Inspector

> **Superseded (T-035):** The right panel is **always-on `AssetPalette`** — it does NOT swap to
> `InspectorPanel` on selection. Attributes edit via **double-click → AttributesModal**. See
> [`agent_execution.md`](agent_execution.md) Decisions log.

The right panel defaults to the **Asset Browser**. This MUST be a nested, collapsible tree view (e.g., Faction → Category → Class) mimicking the Eden Editor, NOT a flat list of pill buttons. Users drag items from this tree directly onto the map.
When an entity is selected, it switches to the `InspectorPanel` based on `selection.kind`:
| State | Component | Props |
|-------|-----------|-------|
| `none` | `AssetBrowser` | Catalog of placeable assets |
| `slot` | `ArsenalInspector` | `slot: Slot`, `loadout: Loadout`, `registry: RegistryClient`, `onEquip`, `onUnequip`, `onAttach` |
| `objective` | `ObjectiveInspector` | `objective: Objective`, `onChange(patch)` |

*Note: Mission Settings are NOT in the right panel. They belong in a dedicated modal dialog.*
*Note: Double-clicking an entity on the map opens the **Attributes Modal**. This is vital for editing positions, skills (medic/engineer), and launching the full Arsenal.*

`ArsenalInspector` renders `SoldierDoll` (2D paper-doll: helmet/vest/uniform/primary/backpack) +
a list of `SlotCategory` (collapsible) each opening an `ItemPicker` (registry-backed search grid).
Every pick round-trips through `useArsenalValidation` → the registry worker before committing.

### 5.3 Bottom Toolbelt
`BottomToolbelt` props: `activeTool: ToolId`, `onToolChange(tool)`, `cursorWorld: {x,y,z}` (live
JetBrains-Mono readout). Tools: Select, Ruler, Line-of-Sight. (Unit placement is exclusively drag-and-drop from the Asset Browser).

### 5.4 Top Command Strip
`TopCommandStrip`: mission title (inline edit), **Visual-Git timeline scrubber**, a button to open the **Mission Settings Dialog** (time/weather/environment), and the **Export** button.

---

## 6. Edge Cases & Error Handling

| Scenario | Behavior |
|----------|----------|
| **DEM heightmap fails to load** | Fall back to a flat plane (`z = 0`); raise a non-blocking `sonner` toast; disable elevation-dependent tools (viewshed, Z-snap) with an explanatory tooltip + a Retry button. The canvas **never crashes** — base map + 2D placement keep working. |
| **Registry fetch fails** | Serve the last IndexedDB cache; if none, enter degraded mode (placement allowed, validation **warns** not blocks) with a persistent banner. |
| **Autosave / persistence failure** | Local Y.Doc is optimistic and durable via **v2 chunked IDB** (`tbd-mission-persist`, T-062.1); the POST to save a mission version retries with backoff; an "unsaved changes" indicator shows until confirmed. Local work is never lost on a failed save (flush on tab hide/pagehide). |
| **WebSocket disconnect** *(deferred — ADR-3)* | v1 is offline-first on the local Y.Doc, so there is no live socket to drop. *Forward-looking:* once y-websocket lands, Yjs buffers offline edits and auto-merges on reconnect; a "reconnecting" presence chip appears; the offline queue persists in IndexedDB. |
| **Conflicting multiplayer edits** *(deferred — ADR-3)* | Yjs CRDT guarantees convergence: disjoint-field edits merge; same-scalar edits resolve deterministically (Lamport ordering); ID-keyed `Y.Map`s make structural moves (reparent/insert/delete) commute. N/A for solo v1, but §2.3's schema is built for it — **no migration needed** to turn multiplayer on. |
| **200-slot performance** | All entity rendering is GPU/Deck-driven (no per-entity DOM); selectors are memoized; edits are batched transactions. Target: 60 fps pan/zoom with 200+ entities. |
| **Custom terrain with no DEM** | Editor loads in flat-Z mode (as the DEM-fail path) and flags that elevation export will be `0`. |
| **Oversized / malformed payload on load** | `useMissionEditor` validates the loaded `json_payload`; on parse failure it starts an empty doc and surfaces the error rather than crashing. |

---

## 7. Ultra-Granular, File-by-File Execution Roadmap

> The repo is an existing Vite app, so there is no `npm init`; Phase 0 begins at dependency
> install. Each phase ends with a concrete, demoable deliverable.
>
> **Refined ordering (authoritative in `agent_execution.md`):** Phases 0/1/4 and a first-cut
> Phase 3 shell shipped as **T-029…T-032**; the Outliner↔Y.Doc + asset-drag wiring shipped as
> **T-033 (PRE-3.5)**. The remaining near-term order is **DOC-0 → 3.5 (Eden docked shell) → 7b (map
> drag + marquee) → 7a (outliner reparent/rename/delete) → 9 (compiler + autosave)**. Phases 2
> (**T-090**/**T-091**), 5 (**T-068** registry), 6, 8 stay blocked on external assets/backend or Eden
> queue. **T-110** terrain base (millions of map props) is separate from mission-layer scale — see
> [`t110_terrain_base_mission_layers.md`](t110_terrain_base_mission_layers.md). Use
> [`agent_execution.md`](agent_execution.md) for live status of each sub-phase.

**Phase 0 — Dependencies & scaffold**
`npm i deck.gl @deck.gl/core @deck.gl/layers @deck.gl/react @luma.gl/core yjs y-indexeddb comlink idb`.
Create both feature trees (§1) as stubs. Register the `React.lazy` route `/missions/:id/edit`
(mission_maker+, `fullBleed` handle). **Deliverable:** route loads an empty shell.

**Phase 1 — Core viewport** → `coords/projection.ts`, `coords/terrains.ts`,
`view/useOrthographicView.ts`, `TacticalMap.tsx`, `layers/useBaseMapLayer.ts`, `context/MapContext.tsx`.
**Deliverable:** a blank base map with 60 fps pan/zoom.

**Phase 2 — DEM / Z-axis** (**T-091**, blocked on heightmap assets) → `dem/DemTexture.ts`, `dem/sampleElevation.ts`, `dem/DemController.ts`,
`layers/useDemLayer.ts`, `layout/BottomToolbelt.tsx` (X/Y/Z readout). **Deliverable:** hover the
map and read true elevation; hillshade overlay toggles. Map tile imagery is **T-090** (Phase 2
prerequisite for visual parity — see [`ROADMAP.md`](ROADMAP.md) §Recommended program order).

**Phase 3 — Shell / layout** → `MissionCreatorPage.tsx`, `layout/TopCommandStrip.tsx`,
`LeftOutliner/*`, `RightInspector/InspectorPanel.tsx` + `GlobalSettingsInspector.tsx`.
**Deliverable:** the full Aegis-glass application shell around the live map.

**Phase 4 — State foundation** → `state/ydoc.ts`, `state/schema.ts`, `state/useMapStore.ts`,
`state/bindings.ts`, `state/undo.ts`, `state/selectors.ts`, `layers/useIconLayer.ts`.
**Deliverable:** add/move test icons that persist across reload (v2 IDB, T-062.1) and undo/redo.

**Phase 5 — Registry worker** (**T-068** dependency) → `registry/registry.worker.ts`, `registry/registryClient.ts`,
`registry/registryTypes.ts`. (Depends on backend `/api/v1/registry`.) **Deliverable:** off-thread
`canEquip`/`canAttach` answers in the console; IndexedDB cache verified.

**Phase 6 — Arsenal Inspector** → `RightInspector/ArsenalInspector/*`, `arsenal/useArsenalValidation.ts`.
**Deliverable:** select a slot, dress the soldier via drag/drop with live registry validation.

**Phase 7 — Outliner + map sync** → finish `LeftOutliner/*` (Tree View + Custom Layers + reparent DnD),
bind `useIconLayer` to `slotsById`, wire click-to-center. **Deliverable:** manage 200 slots from
the tree with the map staying in sync.

**Phase 8 — Tools & objectives** → `tools/*`, `layers/useLineLayer.ts`, `layers/usePolygonLayer.ts`,
`layers/useViewshedLayer.ts` (GLSL), `objectives/ObjectiveLayerAdapter.ts`, `objectives/triggers.ts`,
`RightInspector/ObjectiveInspector.tsx`. **Deliverable:** ruler, line-of-sight, and drawable
objectives with triggers.

**Phase 9 — The Compiler** → `compiler/compile.ts`, `compiler/compiler.worker.ts`,
`compiler/exportSchema.ts`, `hooks/useMissionEditor.ts` (autosave), Visual-Git timeline UI in
`TopCommandStrip`. **Deliverable:** traverse the normalized state → `json_payload` and the
camelCase mod export; save versions; scrub history.

---

## 8. Appendix — The JSON Contract (non-negotiable)

The compiler's output **must remain compatible** with the existing backend, which already
parses ORBAT out of `MissionVersion.JSONPayload`:

- `internal/services/mission_payload.go` `ParseOrbatTemplate` (T-062.1.1): if legacy `orbat[]` present, use it; else derive from `editor` (factions → squads → slots, index order). `events.go` `parseOrbatTemplate` is a thin wrapper.
  ```jsonc
  { "orbat": [ { "faction": "...", "callsign": "...", "squad": "...",
                 "slots": [ { "role": "...", "loadout": "...", "tag": "..." } ] } ] }
  ```
- `internal/handlers/events.go` `materializeSlots` expands each squad's slots into ORBAT
  records (`Faction, Callsign, Squad, Role, Loadout, Tag, SlotIndex`).

**Save Version (T-062.1.1):** POST body is **editor-only** — `orbat[]` omitted; Go derives on event attach. **Export** and legacy versions still include explicit `orbat[]`. Extended keys the mod needs —
`map`, `environment`, `objectives`, `vehicles`, `markers`, `loadouts`. Example skeleton:

```jsonc
{
  "schemaVersion": 1,
  "map": { "terrain": "everon", "bounds": [0, 0, 12800, 12800] },
  "environment": { "time": "06:00", "weather": "clear", "viewDistance": 1600, "thermals": false },
  "orbat": [
    { "faction": "BLUFOR", "callsign": "Platoon HQ", "squad": "Alpha 1-1",
      "slots": [ { "role": "Squad Leader", "loadout": "L85A3 + GL", "tag": "SL" } ] }
  ],
  "loadouts": { /* loadoutId -> resolved containers/weapons/items (Arma classnames) */ },
  "objectives": [], "vehicles": [], "markers": []
}
```

The separate **camelCase** `/missions/:id/export` payload (for the Arma mod) is produced by
`compiler/exportSchema.ts`, matching the `missionJSON` export struct in
`internal/handlers/missions.go` (the documented camelCase exception in `CLAUDE.md`).

---

*End of Ultra Plan. The 2D Mission Creator is the last unbuilt piece of the platform; this
document is the execution contract from dependency install to JSON compiler.*
