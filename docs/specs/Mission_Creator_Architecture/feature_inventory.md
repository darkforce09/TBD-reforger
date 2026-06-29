# TBD Mission Creator — Feature Inventory

**Document:** `feature_inventory.md`  
**Schema:** [reference/feds_schema.md](reference/feds_schema.md) (FEDS)  
**Audited:** 2026-06-20 (initial) · **Second pass:** 2026-06-20 (code + scrape cross-check)  
**UX cross-check:** [ux_spec.md](ux_spec.md)

---

## Glossary (TBD)

| Term | Definition |
|------|------------|
| **Entity** | Placeable object in Y.Doc entity maps (`slots`, `vehicles`, `markers`, `objectives`, …) |
| **Slot** | Placed unit in `slotsById`; one export row under a squad |
| **ORBAT** | Export hierarchy Faction → Squad → Slot (`compile.ts` → `json_payload.orbat[]`) |
| **Editor Layer** | Workflow folder (`editorLayers`); stored in `editor` block; **not** Eden Layer |
| **Active layer** | `activeLayerId` — target folder for palette drops |
| **Selection** | `selection: { kind: 'none' \| 'slot' \| …, ids: string[] }` in Zustand |
| **Squad** | ORBAT unit grouping (`squadsById`); auto-created on first slot placement |
| **Faction** | ORBAT top level (`factionsById`) |

---

## Summary

| Status | Count |
|--------|-------|
| working | 59 |
| partial | 12 |
| stub | 8 |
| disabled | 3 |
| not_built | 6 |

---

## SHELL — Route & layout

#### SHELL-ROUTER-001 — Chromeless fullscreen editor

| Field | Value |
|-------|-------|
| **Domain** | SHELL |
| **Goal** | Eden-style editor owns full viewport without platform chrome |
| **Trigger** | Navigate to `/missions/:id/edit` as `mission_maker+` |
| **Preconditions** | Auth gate passes; lazy chunk may still load |
| **Procedure** | 1. `routes.ts` lazy-loads `MissionCreatorPage`. 2. Route handle `chromeless` + `fullBleed` suppresses `Sidebar`/`TopNav` in `AppLayout`. 3. `TacticalMap` `absolute inset-0`; frosted panels in `z-10` overlay. |
| **Postconditions** | Editor visible; map fills center between docked panels |
| **Inputs** | Route URL `:id` |
| **Outputs** | Render tree; Suspense fallback while chunk loads |
| **Edge cases** | Suspense shows "Loading editor…" |
| **Acceptance** | `- [ ] No platform sidebar/topnav on edit route` `- [ ] Map visible between w-64 / w-80 panels` |
| **Eden parity** | Eden:FILE-SESSION-001 |
| **Status** | working |
| **Evidence** | `mission-creator/routes.ts`, `MissionCreatorPage.tsx`, `AppLayout.tsx` |

#### SHELL-UUID-001 — Invalid mission ID banner

| Field | Value |
|-------|-------|
| **Domain** | SHELL |
| **Goal** | Warn when URL `:id` is not a UUID (Save Version would fail) |
| **Trigger** | Editor mount with non-UUID `:id` |
| **Preconditions** | `useMissionEditor` `UUID_RE` fails |
| **Procedure** | 1. `invalidMissionId=true`. 2. Yellow banner below top strip. 3. Save Version returns same error string. |
| **Postconditions** | User informed; local edit/export still possible |
| **Inputs** | URL param |
| **Outputs** | Banner UI; blocked save error |
| **Edge cases** | Export/hydrate may still run locally |
| **Acceptance** | `- [ ] /missions/test/edit shows banner` `- [ ] Save shows UUID error` |
| **Eden parity** | N/A (web routing) |
| **Status** | working |
| **Evidence** | `hooks/useMissionEditor.ts` (`UUID_RE`, `invalidMissionId`), `MissionCreatorPage.tsx` |

#### SHELL-CONFLICT-001 — Local vs server load conflict dialog

| Field | Value |
|-------|-------|
| **Domain** | SHELL |
| **Goal** | Let user choose IndexedDB draft vs server `current_version` when both have content |
| **Trigger** | Local v2 store (or legacy y-indexeddb) synced + cold `GET /missions/:id` returns `json_payload` + local doc has authored entities |
| **Preconditions** | `hasLocalContent()` true; server version exists |
| **Procedure** | 1. `useMissionDoc` `onSynced`. 2. Fetch mission. 3. `setConflict(payload)`. 4. User picks hydrate or keep local. |
| **Postconditions** | Doc matches chosen source; dirty set if keep local |
| **Inputs** | Dialog button clicks |
| **Outputs** | `hydrateMissionDoc` or dismiss; `INIT_ORIGIN` |
| **Edge cases** | Cannot dismiss without choice (`onOpenChange` noop); empty local auto-hydrates; API fail → local-only |
| **Acceptance** | `- [ ] Conflict when both have slots` `- [ ] "Load saved" hydrates server` `- [ ] "Keep local" preserves IndexedDB` |
| **Eden parity** | N/A |
| **Status** | working |
| **Evidence** | `hooks/useMissionEditor.ts` (`conflict`, `resolveConflict`, `hasLocalContent`), `MissionCreatorPage.tsx` |

#### SHELL-FPS-001 — FPS debug HUD

| Field | Value |
|-------|-------|
| **Domain** | SHELL |
| **Goal** | Dev perf readout during Deck.gl tuning |
| **Trigger** | Editor open (always on) |
| **Preconditions** | None |
| **Procedure** | RAF loop counts frames / 500ms → color-coded label bottom-right |
| **Postconditions** | FPS displayed |
| **Inputs** | N/A |
| **Outputs** | `FpsCounter` overlay (`pointer-events-none`) |
| **Edge cases** | Not gated behind dev flag |
| **Acceptance** | `- [ ] FPS number visible in editor` |
| **Eden parity** | N/A |
| **Status** | working |
| **Evidence** | `FpsCounter.tsx` |

---

## MAP — Viewport & camera

#### MAP-VIEW-001 — Orthographic north-up canvas

| Field | Value |
|-------|-------|
| **Domain** | MAP |
| **Goal** | Flat Arma meters, north-up tactical view |
| **Trigger** | `TacticalMap` mount |
| **Preconditions** | `terrain` prop (default Everon) |
| **Procedure** | 1. Deck `OrthographicView` `flipY:false`. 2. `COORDINATE_SYSTEM.CARTESIAN`. 3. Identity world↔pixel projection. |
| **Postconditions** | Map renders with terrain bounds |
| **Inputs** | `terrain="everon"` from shell |
| **Outputs** | DeckGL layer stack |
| **Edge cases** | Terrain not driven by mission meta in shell |
| **Acceptance** | `- [ ] Pan moves view; +Y is north` |
| **Eden parity** | Eden:MAP-VIEW-001 (3D vs 2D) |
| **Status** | working |
| **Evidence** | `tactical-map/view/useOrthographicView.ts`, `coords/projection.ts`, `TacticalMap.tsx` |

#### MAP-PAN-001 — Pan via MMB or RMB drag

| Field | Value |
|-------|-------|
| **Domain** | MAP |
| **Goal** | Navigate map without moving entities |
| **Trigger** | MMB or RMB down + drag on map container |
| **Preconditions** | Not left button |
| **Procedure** | 1. `useSelectTool` `onPointerDown` button 1/2. 2. Freeze viewport at start. 3. Offset target by unprojected delta. 4. `onContextMenu` prevented. |
| **Postconditions** | `viewState.target` updated (clamped) |
| **Inputs** | MMB, RMB |
| **Outputs** | Camera pan |
| **Edge cases** | `dragPan:false` on Deck; LMB never pans |
| **Acceptance** | `- [ ] MMB drag pans` `- [ ] RMB drag pans` `- [ ] No context menu` |
| **Eden parity** | Eden:MAP-CAM-001 |
| **Status** | working |
| **Evidence** | `tools/useSelectTool.ts`, `TacticalMap.tsx` |

#### MAP-ZOOM-001 — Mouse wheel zoom

| Field | Value |
|-------|-------|
| **Domain** | MAP |
| **Goal** | Zoom tactical view |
| **Trigger** | Wheel over map |
| **Preconditions** | Map focused/hovered |
| **Procedure** | Deck default scroll zoom → `onViewStateChange` clamps zoom `-6`…`6` |
| **Postconditions** | Zoom updated |
| **Inputs** | Wheel |
| **Outputs** | `viewState.zoom` |
| **Edge cases** | `doubleClickZoom` disabled |
| **Acceptance** | `- [ ] Wheel zooms in/out` `- [ ] Stops at min/max` |
| **Eden parity** | Eden:MAP-CAM-002 |
| **Status** | working |
| **Evidence** | `useOrthographicView.ts`, `TacticalMap.tsx` |

#### MAP-GRID-001 — 1 km procedural grid

| Field | Value |
|-------|-------|
| **Domain** | MAP |
| **Goal** | Tactical grid overlay (Eden map-grid analogue) |
| **Trigger** | `showGrid={true}` |
| **Preconditions** | Editor passes `showGrid` |
| **Procedure** | `useBaseMapLayer` draws 1 km `LineLayer` grid + 5 km majors + border |
| **Postconditions** | Grid visible |
| **Inputs** | Prop |
| **Outputs** | Base map layer |
| **Edge cases** | No DEM/imagery; default off if prop false |
| **Acceptance** | `- [ ] Grid visible on editor map` |
| **Eden parity** | partial (no terrain mesh) |
| **Status** | partial |
| **Evidence** | `layers/useBaseMapLayer.ts`, `MissionCreatorPage.tsx` |

#### MAP-FLY-001 — Fly-to world position (Spacebar)

| Field | Value |
|-------|-------|
| **Domain** | MAP |
| **Goal** | Center camera on selection without auto-fly on click |
| **Trigger** | `Space` key (wired in mission-creator) |
| **Preconditions** | `selection.kind !== 'none'`; slots exist |
| **Procedure** | 1. Compute centroid of selected slot positions. 2. `mapApi.flyTo({x,y})`. 3. Target clamped to terrain bounds. |
| **Postconditions** | Camera centered; zoom unchanged |
| **Inputs** | `Space` |
| **Outputs** | `viewState.target` |
| **Edge cases** | Ignored in INPUT/SELECT/contentEditable; no-op if empty selection |
| **Acceptance** | `- [ ] Select unit → Space centers camera` `- [ ] Click select does not move camera` |
| **Eden parity** | Eden:MAP-CAM-003 |
| **Status** | working |
| **Evidence** | `useOrthographicView.ts` `flyTo`, `MissionCreatorPage.tsx` keydown |

#### MAP-CURSOR-001 — Live X/Y/Z coordinate readout (T-050)

| Field | Value |
|-------|-------|
| **Domain** | MAP |
| **Goal** | Eden toolbelt-style position HUD (X/Y/Z) |
| **Trigger** | Mouse move over map |
| **Preconditions** | Map mounted |
| **Procedure** | Container `onPointerMove` → rAF-throttled self-unproject (`view.makeViewport(...).unproject`) → `useMapStore.setCursor` → `BottomToolbelt` mono display (CUR mode). **No Deck `onHover` pick** (T-057). |
| **Postconditions** | X/Y/Z shown; **Z = 0** on the flat map until **T-091.2** live sampling (Everon DEM shipped T-091.0) |
| **Inputs** | Pointer move over map container |
| **Outputs** | Transient `useMapStore.cursor` (not page React state) |
| **Edge cases** | Off-map / pointer leave → `cursor: null` → all axes show `—`; pointer is constant `crosshair` (no icon hover glyph — T-057 trade) |
| **Acceptance** | `- [x] Coords update on hover` `- [x] Z shows 0 on-map, — off-map (T-050)` `- [x] Page does not re-render on move (T-057)` |
| **Eden parity** | Eden:BOTTOM-HUD-001 |
| **Status** | working |
| **Ticket** | T-050 |
| **Evidence** | `TacticalMap.tsx`, `BottomToolbelt.tsx`, `MissionCreatorPage.tsx` |

---

## SEL — Selection

#### SEL-MAP-001 — Click-select single slot

| Field | Value |
|-------|-------|
| **Domain** | SEL |
| **Goal** | Primary single selection |
| **Trigger** | LMB up on slot icon (click, not drag) |
| **Preconditions** | Select tool effective (always) |
| **Procedure** | `useSelectTool` pending-left pointerUp → `slotSpatialIndex.pickNearest` → `setSelection({ kind:'slot', ids:[id] })` (T-063; was Deck `onClick`) |
| **Postconditions** | One id selected; icon highlight |
| **Inputs** | LMB |
| **Outputs** | `selection`; IconLayer yellow/larger |
| **Edge cases** | Replaces prior selection; Ctrl/Cmd additive via T-053 |
| **Acceptance** | `- [x] Click unit selects` `- [x] Highlight visible` |
| **Eden parity** | Eden:SEL-001 |
| **Status** | working |
| **Ticket** | T-057 |
| **Evidence** | `useSelectTool.ts`, `slotSpatialIndex.ts`, `useMapStore.ts`, `useIconLayer.ts` |

#### SEL-MAP-002 — Click empty map deselect

| Field | Value |
|-------|-------|
| **Domain** | SEL |
| **Goal** | Clear selection |
| **Trigger** | LMB click on non-icon |
| **Preconditions** | Any selection |
| **Procedure** | `useSelectTool` pending-left pointerUp, no hit → `{ kind:'none', ids:[] }` (T-063) |
| **Postconditions** | Selection cleared |
| **Inputs** | LMB on empty |
| **Outputs** | `selection` cleared |
| **Edge cases** | Plain LMB only; **Ctrl/Cmd+empty preserves** selection (T-053). No camera teleport (removed Phase 7b) |
| **Acceptance** | `- [x] Plain click empty clears selection` `- [x] Ctrl/Cmd+empty preserves` |
| **Eden parity** | Eden:SEL-002 |
| **Status** | working |
| **Ticket** | T-057 |
| **Evidence** | `useSelectTool.ts` (T-053/T-063) |

#### SEL-MAP-003 — Marquee box-select

| Field | Value |
|-------|-------|
| **Domain** | SEL |
| **Goal** | Primary multi-select (Eden box select) |
| **Trigger** | LMB drag on empty map >4px threshold, release |
| **Preconditions** | Start on non-icon |
| **Procedure** | 1. `useSelectTool` marquee mode. 2. `useSelectionLayer` draws rect. 3. On release `slotSpatialIndex.pickRect` (world bbox). 4. Set `ids` from picks. |
| **Postconditions** | Multi or single selection; zero picks → deselect |
| **Inputs** | LMB drag |
| **Outputs** | `selection.ids[]`; outliner multi-highlight |
| **Edge cases** | Box ≥1×1 px; sub-threshold = click only |
| **Acceptance** | `- [x] Drag box selects multiple units` `- [x] Empty box clears` |
| **Eden parity** | Eden:SEL-003 |
| **Status** | working |
| **Ticket** | T-063 |
| **Evidence** | `useSelectTool.ts`, `useSelectionLayer.ts`, `slotSpatialIndex.ts` |

#### SEL-MAP-004 — Double-click open attributes

| Field | Value |
|-------|-------|
| **Domain** | SEL |
| **Goal** | Eden attributes entry via dbl-click |
| **Trigger** | Native `dblclick` on map container over a slot icon |
| **Preconditions** | `selection.ids.length <= 1` at activate |
| **Procedure** | Native `dblclick` on the map container → `slotSpatialIndex.pickNearest` → `onEntityActivate` → `setAttributesId` (T-054/T-063) |
| **Evidence** | `TacticalMap.tsx` (`onDoubleClick`), `slotSpatialIndex.ts`, `MissionCreatorPage.tsx`, `AttributesModal.tsx` |
| **Postconditions** | `AttributesModal` open |
| **Inputs** | LMB ×2 |
| **Outputs** | `attributesId` |
| **Edge cases** | Suppressed when multi-select (`onEntityActivate` `ids.length <= 1` guard) |
| **Acceptance** | `- [x] Dbl-click unit opens modal` |
| **Eden parity** | Eden:ATTR-OPEN-001 |
| **Status** | working |
| **Ticket** | T-054 |
| **Evidence** | `TacticalMap.tsx` (`onDoubleClick`), `MissionCreatorPage.tsx`, `AttributesModal.tsx` |

#### SEL-MOD-001 — Shift/Ctrl additive selection

| Field | Value |
|-------|-------|
| **Domain** | SEL |
| **Goal** | Eden modifier multi-select |
| **Trigger** | Ctrl/Cmd + LMB click on a slot icon |
| **Preconditions** | Select tool effective (always) |
| **Procedure** | `TacticalMap onClick` reads `event.srcEvent.ctrlKey/metaKey`; toggles id in/out of `useMapStore.getState().selection.ids` → `setSelection` (empties → `none`) |
| **Postconditions** | Slot added or removed from selection; one undo-irrelevant UI op |
| **Inputs** | Ctrl, Cmd (Shift unbound — reserved for future range-select) |
| **Outputs** | `selection.ids[]` |
| **Edge cases** | Ctrl/Cmd + empty-click preserves selection (no deselect); marquee still replaces; Ctrl-built multi (>1) suppresses dbl-click attributes |
| **Acceptance** | `- [x] Ctrl-click adds units` `- [x] Ctrl-click selected removes it` `- [x] Ctrl-click empty preserves` |
| **Eden parity** | Eden:SEL-MOD-001 |
| **Status** | working |
| **Ticket** | T-053 |
| **Evidence** | `TacticalMap.tsx` `onClick` (T-053) |

#### SEL-SYNC-001 — Map ↔ outliner selection sync

| Field | Value |
|-------|-------|
| **Domain** | SEL |
| **Goal** | Single selection state across panels |
| **Trigger** | Click slot in ORBAT, Editor Layers, or map |
| **Preconditions** | Slot id |
| **Procedure** | All panels read/write `useMapStore.selection` |
| **Postconditions** | Highlight sync |
| **Inputs** | Row click / map click |
| **Outputs** | Shared `selection` |
| **Edge cases** | Folder click sets `activeLayerId` not selection; ORBAT faction/squad rows no-op |
| **Acceptance** | `- [ ] Map select highlights layer tree row` `- [ ] ORBAT slot click selects on map` |
| **Eden parity** | Eden:SEL-SYNC-001 |
| **Status** | partial |
| **Evidence** | `OrbatSection.tsx`, `EditorLayersSection.tsx`, `useMapStore.ts` |

---

## XFORM — Transform & delete

#### XFORM-MOVE-001 — Drag-move slots (live preview)

| Field | Value |
|-------|-------|
| **Domain** | XFORM |
| **Goal** | Reposition units on map |
| **Trigger** | LMB drag selected (or newly selected) icon ≥4px |
| **Preconditions** | Slot icon under cursor |
| **Procedure** | 1. `useSelectTool` move mode. 2. Transient `drag {ids,dx,dy}` offsets icons. 3. On release `onEntitiesMove` → `moveEntities` one transact. |
| **Postconditions** | `position.x/y` updated; one undo step |
| **Inputs** | LMB drag |
| **Outputs** | Y.Doc `slots`; cleared `drag` |
| **Edge cases** | Zero delta no commit; multi-select moves all `selection.ids` |
| **Acceptance** | `- [ ] Drag moves unit` `- [ ] Undo restores position` `- [ ] Group move works` |
| **Eden parity** | Eden:XFORM-MOVE-001 |
| **Status** | working |
| **Evidence** | `useSelectTool.ts`, `ydoc.ts` `moveEntities`, `MissionCreatorPage.tsx` |

#### XFORM-DEL-001 — Delete selection (keyboard)

| Field | Value |
|-------|-------|
| **Domain** | XFORM |
| **Goal** | Remove selected slots |
| **Trigger** | `Delete` or `Backspace` |
| **Preconditions** | `selection.kind !== 'none'` and `ids.length > 0`; focus not in INPUT/SELECT/contentEditable |
| **Procedure** | `removeEntities(md,'slots',ids)` → clear selection (always targets `slots` map) |
| **Postconditions** | Slots removed; undoable |
| **Inputs** | Delete, Backspace |
| **Outputs** | Y.Doc delete |
| **Edge cases** | Skips INPUT/SELECT/contentEditable; no confirm dialog |
| **Acceptance** | `- [ ] Delete removes selected` `- [ ] Undo restores` |
| **Eden parity** | Eden:XFORM-DEL-001 |
| **Status** | working |
| **Evidence** | `MissionCreatorPage.tsx`, `ydoc.ts` `removeEntities` |

#### XFORM-ROT-001 — Rotation on map

| Field | Value |
|-------|-------|
| **Domain** | XFORM |
| **Goal** | Orient units |
| **Trigger** | N/A |
| **Preconditions** | — |
| **Procedure** | `Slot.position.rotation` in schema only |
| **Postconditions** | — |
| **Inputs** | — |
| **Outputs** | — |
| **Edge cases** | Attributes shows read-only rotation |
| **Acceptance** | `- [ ] N/A` |
| **Eden parity** | Eden:XFORM-ROT-001 |
| **Status** | not_built |
| **Evidence** | `schema.ts`; no map UI |

#### XFORM-SNAP-001 — Grid / surface snap

| Field | Value |
|-------|-------|
| **Domain** | XFORM |
| **Goal** | Snap to grid or terrain |
| **Trigger** | N/A |
| **Preconditions** | DEM Phase 2 for surface |
| **Procedure** | Not implemented |
| **Postconditions** | — |
| **Inputs** | — |
| **Outputs** | — |
| **Edge cases** | — |
| **Acceptance** | `- [ ] N/A` |
| **Eden parity** | Eden:XFORM-SNAP-001 |
| **Status** | not_built |
| **Evidence** | — |

#### XFORM-SYNC-001 — Sync position between entities

| Field | Value |
|-------|-------|
| **Domain** | XFORM |
| **Goal** | Eden sync/align entities |
| **Trigger** | N/A |
| **Preconditions** | — |
| **Procedure** | Not implemented |
| **Postconditions** | — |
| **Inputs** | — |
| **Outputs** | — |
| **Edge cases** | — |
| **Acceptance** | `- [ ] N/A` |
| **Eden parity** | Eden:XFORM-SYNC-001 |
| **Status** | not_built |
| **Evidence** | — |

---

## PLACE — Placement

#### PLACE-DROP-001 — Palette drag-drop create slot

| Field | Value |
|-------|-------|
| **Domain** | PLACE |
| **Goal** | Place units from asset browser |
| **Trigger** | Drop asset leaf on map (`application/x-tbd-asset`) |
| **Preconditions** | Factions tab; leaf draggable |
| **Procedure** | 1. Parse `AssetDropPayload`. 2. Unproject drop coords. 3. `addSlot(world, {role, layerId:activeLayerId, assetId})`. 4. Select new id. |
| **Postconditions** | New slot in squad + active layer |
| **Inputs** | HTML5 DnD |
| **Outputs** | Y.Doc slot; `z:0` |
| **Edge cases** | Only `kind:'slot'` payload; mock catalog includes **Vehicles** and **Objects** leaves under Factions tab — all still create **slots** (not vehicle entities); Vehicles tab is stub |
| **Acceptance** | `- [ ] Drag rifleman onto map creates unit` `- [ ] Unit in active layer folder` |
| **Eden parity** | Eden:PLACE-001 |
| **Status** | partial |
| **Evidence** | `AssetBrowser.tsx`, `TacticalMap.tsx`, `ydoc.ts` `addSlot`, `MissionCreatorPage.tsx` |

#### PLACE-CLICK-001 — Click-to-place tool

| Field | Value |
|-------|-------|
| **Domain** | PLACE |
| **Goal** | Place selected asset with click |
| **Trigger** | N/A |
| **Preconditions** | `activeTool:'place'` unused |
| **Procedure** | `activeTool` not read by map |
| **Postconditions** | — |
| **Inputs** | — |
| **Outputs** | — |
| **Edge cases** | Placement palette-only |
| **Acceptance** | `- [ ] N/A` |
| **Eden parity** | Eden:PLACE-002 |
| **Status** | not_built |
| **Evidence** | `useMapStore.ts` `activeTool`; `useSelectTool.ts` ignores |

---

## LEFT — Left sidebar

#### LEFT-ORBAT-001 — ORBAT tree display (read-only)

| Field | Value |
|-------|-------|
| **Domain** | LEFT-ORBAT |
| **Goal** | Show export hierarchy Faction→Squad→Slot |
| **Trigger** | Slots exist in store |
| **Preconditions** | `buildOrbat` from factions/squads/slots |
| **Procedure** | `OrbatSection` renders `TreeView` in left outliner until **T-071**; then ORBAT display moves to **ORBAT Manager modal** |
| **Postconditions** | Tree visible (left today; modal after T-071) |
| **Inputs** | N/A |
| **Outputs** | UI |
| **Edge cases** | Empty state message; authoring UI in ORBAT Manager (**T-071**) |
| **Acceptance** | `- [ ] Placed units appear under default faction/squad` `- [ ] T-071: no duplicate ORBAT tree on left; Editor Layers only` |
| **Eden parity** | Eden:ORBAT-001 |
| **Status** | partial — read-only left tree today; **T-071** ORBAT Manager **queued** (blocked on **T-092**) |
| **Evidence** | `OrbatSection.tsx`, `ydoc.ts` `ensureDefaultSquad` |

#### LEFT-ORBAT-002 — ORBAT slot select

| Field | Value |
|-------|-------|
| **Domain** | LEFT-ORBAT |
| **Goal** | Select unit from ORBAT |
| **Trigger** | Click slot row |
| **Preconditions** | Row is slot kind |
| **Procedure** | `setSelection({ kind:'slot', ids:[id] })` |
| **Postconditions** | Map highlight |
| **Inputs** | LMB row |
| **Outputs** | `selection` |
| **Edge cases** | Faction/squad rows no handler |
| **Acceptance** | `- [ ] ORBAT click selects on map` |
| **Eden parity** | Eden:ORBAT-002 |
| **Status** | working |
| **Evidence** | `OrbatSection.tsx` |

#### LEFT-LAYER-001 — Editor Layers tree display

| Field | Value |
|-------|-------|
| **Domain** | LEFT-LAYER |
| **Goal** | Workflow folder hierarchy with slotted units |
| **Trigger** | `editorLayers` + slots |
| **Preconditions** | `seedDefaultLayer` on first sync |
| **Procedure** | `EditorLayersSection` `buildTree` → nested folders + entity nodes |
| **Postconditions** | Tree rendered |
| **Inputs** | N/A |
| **Outputs** | UI |
| **Edge cases** | Always ≥1 layer |
| **Acceptance** | `- [ ] Default folder exists` `- [ ] Units listed under layers` |
| **Eden parity** | TBD-only (workflow; Eden Layer analogous) |
| **Status** | working |
| **Evidence** | `EditorLayersSection.tsx`, `ydoc.ts` `seedDefaultLayer` |

#### LEFT-LAYER-002 — Select active drop folder

| Field | Value |
|-------|-------|
| **Domain** | LEFT-LAYER |
| **Goal** | Choose palette drop target |
| **Trigger** | Click folder row |
| **Preconditions** | Folder node |
| **Procedure** | `setActiveLayer(id)` → highlight `selectedId` |
| **Postconditions** | `activeLayerId` set |
| **Inputs** | LMB folder |
| **Outputs** | `activeLayerId` |
| **Edge cases** | Null falls back to default in `addSlot` |
| **Acceptance** | `- [ ] Selected folder receives new drops` |
| **Eden parity** | partial |
| **Status** | working |
| **Evidence** | `EditorLayersSection.tsx`, `useMapStore.ts` |

#### LEFT-LAYER-003 — Select slot from layers tree

| Field | Value |
|-------|-------|
| **Domain** | LEFT-LAYER |
| **Goal** | Select from workflow tree |
| **Trigger** | Click unit row |
| **Preconditions** | Non-folder node |
| **Procedure** | `setSelection` single id |
| **Postconditions** | Map sync |
| **Inputs** | LMB |
| **Outputs** | `selection` |
| **Edge cases** | Multi-highlight when map marquee |
| **Acceptance** | `- [ ] Layer tree click selects` |
| **Eden parity** | Eden:LAYER-SEL-001 |
| **Status** | working |
| **Evidence** | `EditorLayersSection.tsx` |

#### LEFT-LAYER-004 — Double-click slot → attributes

| Field | Value |
|-------|-------|
| **Domain** | LEFT-LAYER |
| **Goal** | Open attributes from tree |
| **Trigger** | Dbl-click unit row |
| **Preconditions** | Non-folder |
| **Procedure** | `TreeView.onActivate` → `onActivateSlot` |
| **Postconditions** | Modal open |
| **Inputs** | Dbl-click |
| **Outputs** | `attributesId` |
| **Edge cases** | Folders don't activate |
| **Acceptance** | `- [ ] Dbl-click tree row opens modal` |
| **Eden parity** | Eden:ATTR-OPEN-001 |
| **Status** | working |
| **Ticket** | T-054 |
| **Evidence** | `TreeView.tsx`, `EditorLayersSection.tsx` |

#### LEFT-LAYER-005 — New folder

| Field | Value |
|-------|-------|
| **Domain** | LEFT-LAYER |
| **Goal** | Create workflow folder |
| **Trigger** | Click `FolderPlus` in section header |
| **Preconditions** | Editable doc |
| **Procedure** | `addEditorLayer` → "New Folder N" → set active |
| **Postconditions** | New layer; undoable |
| **Inputs** | Button |
| **Outputs** | Y.Doc `editorLayers` |
| **Edge cases** | — |
| **Acceptance** | `- [ ] New folder appears` `- [ ] Undo removes` |
| **Eden parity** | Eden:LAYER-CREATE-001 |
| **Status** | working |
| **Evidence** | `EditorLayersSection.tsx`, `ydoc.ts` |

#### LEFT-LAYER-006 — Rename folder (inline)

| Field | Value |
|-------|-------|
| **Domain** | LEFT-LAYER |
| **Goal** | Rename workflow folder |
| **Trigger** | Click pencil → edit → Enter/blur |
| **Preconditions** | Folder row |
| **Procedure** | `renamingId` inline input → `renameEditorLayer` if changed |
| **Postconditions** | Name updated |
| **Inputs** | Keyboard, pencil button |
| **Outputs** | Y.Doc |
| **Edge cases** | Escape cancels; empty trim no-op |
| **Acceptance** | `- [ ] Rename persists` |
| **Eden parity** | Eden:LAYER-RENAME-001 |
| **Status** | working |
| **Evidence** | `TreeView.tsx`, `ydoc.ts` |

#### LEFT-LAYER-007 — Delete folder (destructive subtree)

| Field | Value |
|-------|-------|
| **Domain** | LEFT-LAYER |
| **Goal** | Remove folder and contained units |
| **Trigger** | Click trash on folder |
| **Preconditions** | `editorLayers.size > 1` |
| **Procedure** | `removeEditorLayer` deletes subtree + slots; reset active; reseed if empty |
| **Postconditions** | Folder gone; slots removed |
| **Inputs** | Trash button |
| **Outputs** | Y.Doc |
| **Edge cases** | No-op if only one layer left |
| **Acceptance** | `- [ ] Delete folder removes units` `- [ ] Cannot delete last folder` |
| **Eden parity** | Eden:LAYER-DEL-001 |
| **Status** | working |
| **Evidence** | `EditorLayersSection.tsx`, `ydoc.ts` |

#### LEFT-LAYER-008 — Delete unit from tree

| Field | Value |
|-------|-------|
| **Domain** | LEFT-LAYER |
| **Goal** | Remove single slot from outliner |
| **Trigger** | Trash on unit row |
| **Preconditions** | Slot row |
| **Procedure** | `removeEntity` slot; trim selection |
| **Postconditions** | Slot removed |
| **Inputs** | Trash |
| **Outputs** | Y.Doc |
| **Edge cases** | Differs from keyboard multi-delete |
| **Acceptance** | `- [ ] Tree delete removes one unit` |
| **Eden parity** | Eden:XFORM-DEL-001 |
| **Status** | working |
| **Evidence** | `EditorLayersSection.tsx`, `ydoc.ts` |

#### LEFT-LAYER-009 — Drag reparent folder / refile slot

| Field | Value |
|-------|-------|
| **Domain** | LEFT-LAYER |
| **Goal** | Reorganize workflow tree |
| **Trigger** | DnD `application/x-tbd-tree-node` onto folder |
| **Preconditions** | Valid drop target |
| **Procedure** | Folder→folder `reparentEditorLayer`; slot→folder `moveSlotToLayer` |
| **Postconditions** | Tree structure updated |
| **Inputs** | HTML5 DnD |
| **Outputs** | Y.Doc |
| **Edge cases** | Cycle reparent rejected; slot can't drop root |
| **Acceptance** | `- [ ] Reparent folder` `- [ ] Move unit between folders` |
| **Eden parity** | partial |
| **Status** | working |
| **Evidence** | `EditorLayersSection.tsx`, `TreeView.tsx`, `ydoc.ts` |

#### LEFT-LAYER-010 — Move folder to root dropzone

| Field | Value |
|-------|-------|
| **Domain** | LEFT-LAYER |
| **Goal** | Un-nest folder to root |
| **Trigger** | Drag folder to dashed "Move folder to root" zone |
| **Preconditions** | Payload `kind:'layer'` |
| **Procedure** | `reparentEditorLayer(md, id, null)` |
| **Postconditions** | Folder at root |
| **Inputs** | DnD |
| **Outputs** | Y.Doc |
| **Edge cases** | Slots ignored in root zone |
| **Acceptance** | `- [ ] Folder moves to root` |
| **Eden parity** | partial |
| **Status** | working |
| **Evidence** | `EditorLayersSection.tsx` |

#### LEFT-TABS-001 — Bottom icon tabs (stub)

| Field | Value |
|-------|-------|
| **Domain** | LEFT |
| **Goal** | Eden left-nav mode switching |
| **Trigger** | Click tab icons |
| **Preconditions** | — |
| **Procedure** | Buttons render; first tab styled active; no `onClick` state |
| **Postconditions** | No panel change |
| **Inputs** | LMB |
| **Outputs** | None |
| **Edge cases** | ORBAT+Layers always shown |
| **Acceptance** | `- [ ] Tabs visible but don't switch` |
| **Eden parity** | Eden:LEFT-TABS-001 |
| **Status** | stub |
| **Evidence** | `LeftSidebar.tsx` `BOTTOM_TABS` |

#### LEFT-TREE-001 — Tree expand/collapse

| Field | Value |
|-------|-------|
| **Domain** | LEFT |
| **Goal** | Navigate hierarchical lists |
| **Trigger** | Click folder with children |
| **Preconditions** | `hasChildren` |
| **Procedure** | `onSelect` + toggle `expanded` set |
| **Postconditions** | Children shown/hidden |
| **Inputs** | LMB |
| **Outputs** | Local UI state |
| **Edge cases** | Empty folders no chevron |
| **Acceptance** | `- [ ] Folders expand/collapse` |
| **Eden parity** | Eden:LEFT-TREE-001 |
| **Status** | working |
| **Evidence** | `TreeView.tsx` |

---

## RIGHT — Asset palette

#### RIGHT-TABS-001 — Palette tab strip

| Field | Value |
|-------|-------|
| **Domain** | RIGHT |
| **Goal** | Eden asset category tabs |
| **Trigger** | Click Factions/Vehicles/Markers/Objectives |
| **Preconditions** | Palette open |
| **Procedure** | Local `tab` state; Factions → `AssetBrowser`; others → stub text |
| **Postconditions** | Tab content switches |
| **Inputs** | LMB tab |
| **Outputs** | UI |
| **Edge cases** | Only Factions functional |
| **Acceptance** | `- [x] Factions shows tree` `- [ ] Other tabs show placeholder` |
| **Eden parity** | Eden:RIGHT-TABS-001 |
| **Status** | partial |
| **Evidence** | `AssetPalette.tsx` |

#### RIGHT-CAT-001 — Factions registry catalog tree

| Field | Value |
|-------|-------|
| **Domain** | RIGHT |
| **Goal** | Browse placeable units |
| **Trigger** | Factions tab active |
| **Preconditions** | `mission_maker+`; `GET /api/v1/registry` returns rows |
| **Procedure** | `useRegistry()` → `buildCatalogTree` (character rows only) → `TreeView`; T-055 `filterCatalog`; leaf drag `ASSET_DND_MIME` with **`resource_name`** as `assetId` |
| **Postconditions** | Draggable leaves with full Enfusion **`resource_name`** in DnD payload |
| **Inputs** | Expand/select/drag; search filters tree |
| **Outputs** | DnD payload → `addSlot(..., { assetId: resource_name, role: display_name })` |
| **Edge cases** | Loading spinner-only (no tree flash); error + Retry; empty modpack message |
| **Acceptance** | `- [x] NATO tree from API` `- [x] Leaf drags to map with resource_name GUID` |
| **Eden parity** | Eden:RIGHT-CAT-001 |
| **Status** | **working** |
| **Ticket** | T-068.3 |
| **Evidence** | `registry/buildCatalogTree.ts`, `AssetBrowser.tsx`, `hooks/queries.ts` `useRegistry()` |

#### RIGHT-SEARCH-001 — Asset browser search

| Field | Value |
|-------|-------|
| **Domain** | RIGHT |
| **Goal** | Eden asset search field |
| **Trigger** | Type in the Asset Browser search box (Factions tab) |
| **Preconditions** | Palette open, Factions tab |
| **Procedure** | T-055: `AssetBrowser` `filterCatalog(catalog, q)` on registry-built tree (case-insensitive label substring; folder kept on self-match → full subtree, else on descendant match → filtered children; retained folders `defaultExpanded`); `TreeView` keyed on query so mount-time expand re-runs |
| **Postconditions** | Tree narrows to matches (ancestors expanded); empty → "No assets match"; clear (X/Esc) restores |
| **Inputs** | Text query; Esc / X to clear |
| **Outputs** | Filtered `TreeView`; filtered leaves still draggable (`ASSET_DND_MIME`) |
| **Edge cases** | Whitespace-only = no filter; stub tabs have no search (no catalog); no `class:` prefix (RIGHT-SEARCH-002, P2) |
| **Acceptance** | `- [x] Query filters tree` `- [x] Folder-name match shows subtree` `- [x] Clear restores` |
| **Eden parity** | Eden:RIGHT-SEARCH-001 |
| **Status** | working |
| **Ticket** | T-055 |
| **Evidence** | `AssetBrowser.tsx` |

#### RIGHT-STUB-001 — Vehicles tab placeholder

| Field | Value |
|-------|-------|
| **Domain** | RIGHT |
| **Goal** | Future vehicle catalog |
| **Trigger** | Vehicles tab |
| **Preconditions** | — |
| **Procedure** | Static message "catalog arrives with the asset registry feed" |
| **Postconditions** | No catalog |
| **Inputs** | Tab click |
| **Outputs** | UI text |
| **Edge cases** | Phase 5-6 |
| **Acceptance** | `- [ ] Stub message shown` |
| **Eden parity** | Eden:VEH-CAT-001 |
| **Status** | stub |
| **Evidence** | `AssetPalette.tsx` |

#### RIGHT-STUB-002 — Markers tab placeholder

| Field | Value |
|-------|-------|
| **Domain** | RIGHT |
| **Goal** | Future marker catalog |
| **Trigger** | Markers tab |
| **Preconditions** | — |
| **Procedure** | Same stub message |
| **Postconditions** | — |
| **Inputs** | Tab |
| **Outputs** | UI |
| **Edge cases** | — |
| **Acceptance** | `- [ ] Stub shown` |
| **Eden parity** | Eden:MRK-CAT-001 |
| **Status** | stub |
| **Evidence** | `AssetPalette.tsx` |

#### RIGHT-STUB-003 — Objectives tab placeholder

| Field | Value |
|-------|-------|
| **Domain** | RIGHT |
| **Goal** | Future objectives catalog |
| **Trigger** | Objectives tab |
| **Preconditions** | — |
| **Procedure** | Same stub message |
| **Postconditions** | — |
| **Inputs** | Tab |
| **Outputs** | UI |
| **Edge cases** | — |
| **Acceptance** | `- [ ] Stub shown` |
| **Eden parity** | Eden:WP-CAT-001 |
| **Status** | stub |
| **Evidence** | `AssetPalette.tsx` |

---

## TOP — Command strip

#### TOP-MENU-001 — Menu bar stubs

| Field | Value |
|-------|-------|
| **Domain** | TOP |
| **Goal** | Eden File/Edit/View/Mission/Environment menus |
| **Trigger** | Click menu labels |
| **Preconditions** | — |
| **Procedure** | `MENUS` buttons with title "(soon)"; no handlers |
| **Postconditions** | Nothing |
| **Inputs** | LMB |
| **Outputs** | None |
| **Edge cases** | — |
| **Acceptance** | `- [ ] Menus visible, no dropdown` |
| **Eden parity** | Eden:TOP-MENU-001 |
| **Status** | stub |
| **Evidence** | `TopCommandStrip.tsx` |

#### TOP-TITLE-001 — Inline mission title edit

| Field | Value |
|-------|-------|
| **Domain** | TOP |
| **Goal** | Edit mission title in doc |
| **Trigger** | Type in title input |
| **Preconditions** | Doc loaded |
| **Procedure** | `setTitle(md, value)` → Y.Doc `meta.title` |
| **Postconditions** | Title updated; dirty |
| **Inputs** | Text |
| **Outputs** | Y.Doc meta |
| **Edge cases** | T-049: hydrated from the server mission row on load via `applyMissionRowMeta` (even when `json_payload` is `{}`); placeholder "Untitled Mission" only until the GET resolves. No PATCH-back yet. |
| **Acceptance** | `- [x] Title edits persist in IndexedDB` · `- [x] Mission row title hydrates on load (T-049)` |
| **Eden parity** | Eden:FILE-TITLE-001 |
| **Status** | partial |
| **Ticket** | T-049 |
| **Evidence** | `TopCommandStrip.tsx`, `ydoc.ts` |

#### TOP-DIRTY-001 — Unsaved changes dot

| Field | Value |
|-------|-------|
| **Domain** | TOP |
| **Goal** | Indicate local edits since last save |
| **Trigger** | Y.Doc update with `LOCAL_ORIGIN` |
| **Preconditions** | — |
| **Procedure** | `useMissionEditor` listener → yellow dot beside title |
| **Postconditions** | `dirty=true` until save/hydrate |
| **Inputs** | Any edit |
| **Outputs** | UI dot |
| **Edge cases** | INIT/persistence origins don't dirty |
| **Acceptance** | `- [ ] Dot after edit` `- [ ] Clears on Save Version` |
| **Eden parity** | N/A |
| **Status** | working |
| **Evidence** | `useMissionEditor.ts`, `TopCommandStrip.tsx` |

#### TOP-ENV-001 — Time-of-day scrubber

| Field | Value |
|-------|-------|
| **Domain** | ENV |
| **Goal** | Eden time control |
| **Trigger** | Drag range 0–1439 min |
| **Preconditions** | — |
| **Procedure** | `updateEnvironment({ time })` → HH:MM label |
| **Postconditions** | `meta.environment.time` |
| **Inputs** | Slider |
| **Outputs** | Y.Doc |
| **Edge cases** | Duplicates Mission Settings |
| **Acceptance** | `- [ ] Time slider updates label` |
| **Eden parity** | Eden:ENV-TIME-001 |
| **Status** | working |
| **Evidence** | `TopCommandStrip.tsx` |

#### TOP-ENV-002 — Weather quick select

| Field | Value |
|-------|-------|
| **Domain** | ENV |
| **Goal** | Quick weather preset |
| **Trigger** | Change `<select>` |
| **Preconditions** | — |
| **Procedure** | `updateEnvironment({ weather })` — clear/overcast/heavy_rain/dense_fog |
| **Postconditions** | Environment updated |
| **Inputs** | Select |
| **Outputs** | Y.Doc |
| **Edge cases** | Synced with settings dialog |
| **Acceptance** | `- [ ] Weather changes persist` |
| **Eden parity** | Eden:ENV-WX-001 |
| **Status** | working |
| **Evidence** | `TopCommandStrip.tsx` |

#### TOP-HIST-001 — Version history (disabled)

| Field | Value |
|-------|-------|
| **Domain** | TOP |
| **Goal** | Visual-Git timeline |
| **Trigger** | History button |
| **Preconditions** | — |
| **Procedure** | Button `disabled` title "soon" |
| **Postconditions** | — |
| **Inputs** | — |
| **Outputs** | — |
| **Edge cases** | Deferred (Visual-Git timeline) |
| **Acceptance** | `- [ ] Button disabled` |
| **Eden parity** | TBD-only |
| **Status** | disabled |
| **Evidence** | `TopCommandStrip.tsx` |

#### TOP-UNDO-001 — Undo button

| Field | Value |
|-------|-------|
| **Domain** | KEY |
| **Goal** | Revert last edit |
| **Trigger** | Click Undo **or Cmd/Ctrl+Z** (T-052) |
| **Preconditions** | `canUndo()` |
| **Procedure** | `UndoController.undo()` Yjs stack |
| **Postconditions** | Prior state restored |
| **Inputs** | Button / keyboard |
| **Outputs** | Y.Doc revert |
| **Edge cases** | Hydrate not undoable; keyboard skipped while a form field is focused (T-052) |
| **Acceptance** | `- [x] Undo after move (button + Cmd/Ctrl+Z)` |
| **Eden parity** | Eden:KEY-UNDO-001 |
| **Status** | working |
| **Ticket** | T-052 |
| **Evidence** | `TopCommandStrip.tsx`, `MissionCreatorPage.tsx` keydown, `undo.ts`, `useMissionDoc.ts` (StrictMode `instanceKey`) |

#### TOP-REDO-001 — Redo button

| Field | Value |
|-------|-------|
| **Domain** | KEY |
| **Goal** | Reapply undone edit |
| **Trigger** | Click Redo **or Cmd/Ctrl+Shift+Z / Ctrl+Y** (T-052) |
| **Preconditions** | `canRedo()` |
| **Procedure** | `UndoController.redo()` |
| **Postconditions** | State forward |
| **Inputs** | Button / keyboard |
| **Outputs** | Y.Doc |
| **Edge cases** | Keyboard skipped while a form field is focused (T-052) |
| **Acceptance** | `- [x] Redo after undo (button + Cmd/Ctrl+Shift+Z / Ctrl+Y)` |
| **Eden parity** | Eden:KEY-REDO-001 |
| **Status** | working |
| **Ticket** | T-052 |
| **Evidence** | `TopCommandStrip.tsx`, `MissionCreatorPage.tsx` keydown |

#### TOP-SAVE-001 — Save Version dialog + POST

| Field | Value |
|-------|-------|
| **Domain** | DATA |
| **Goal** | Immutable semver snapshot to server |
| **Trigger** | Save → confirm dialog |
| **Preconditions** | Valid UUID; semver non-empty |
| **Procedure** | 1. `compileMissionWithProgress` (chunked, T-060). 2. `POST /missions/:id/versions` (256 MB route cap). 3. Clear dirty; update `currentSemver`. |
| **Postconditions** | Server version created |
| **Inputs** | Semver, notes |
| **Outputs** | API |
| **Edge cases** | 409 duplicate semver; invalid UUID; **413 payload >256 MB** → "Mission too large…" (T-060); backend `error` surfaced (no more generic-only) |
| **Acceptance** | `- [x] Save 0.1.1 succeeds on real mission` `- [x] Save @ ~360k fully diagnosed (T-060.1.3)` `- [x] Version POST 140 MB → 201 via curl (T-060.1.4)` `- [x] Browser Save @ ~367k → 201 (2026-06-23)` `- [x] >1 MB + production-like GlobalBodyLimit version POST round-trips (make test-it)` |
| **Status** | **working** — T-060 shipped (`b1fd25a`); Save @ ~367k/~142 MB → 201 (browser + curl) |
| **Eden parity** | Eden:FILE-SAVE-001 |
| **Evidence** | `TopCommandStrip.tsx`, `useMissionEditor.ts`, `compile.ts` |

#### TOP-EXPORT-001 — Export JSON download

| Field | Value |
|-------|-------|
| **Domain** | DATA |
| **Goal** | Download mission export envelope |
| **Trigger** | Export button |
| **Preconditions** | — |
| **Procedure** | `compileMission` + `toMissionExport` → blob download |
| **Postconditions** | File saved |
| **Inputs** | Button |
| **Outputs** | JSON file |
| **Edge cases** | Some envelope fields defaulted empty |
| **Acceptance** | `- [ ] Export downloads JSON` |
| **Eden parity** | Eden:FILE-EXPORT-001 |
| **Status** | working |
| **Ticket** | T-060 |
| **Evidence** | `TopCommandStrip.tsx`, `exportSchema.ts` |

#### TOP-SETTINGS-001 — Mission Settings gear

| Field | Value |
|-------|-------|
| **Domain** | ATTR |
| **Goal** | Global environment dialog |
| **Trigger** | Gear icon |
| **Preconditions** | — |
| **Procedure** | Open `MissionSettingsDialog` |
| **Postconditions** | Dialog visible |
| **Inputs** | Button |
| **Outputs** | UI |
| **Edge cases** | Terrain read-only |
| **Acceptance** | `- [ ] Settings dialog opens` |
| **Eden parity** | Eden:ENV-DIALOG-001 |
| **Status** | partial |
| **Evidence** | `MissionSettingsDialog.tsx`, `TopCommandStrip.tsx` |

---

## BOTTOM — Toolbelt

#### BOTTOM-TOOL-001 — Select tool button

| Field | Value |
|-------|-------|
| **Domain** | BOTTOM |
| **Goal** | Tool mode indicator |
| **Trigger** | Click Select |
| **Preconditions** | — |
| **Procedure** | `setActiveTool('select')` — map ignores, always select |
| **Postconditions** | Button highlighted |
| **Inputs** | LMB |
| **Outputs** | `activeTool` |
| **Edge cases** | Cosmetic only |
| **Acceptance** | `- [ ] Select highlighted` `- [ ] Map always selects` |
| **Eden parity** | Eden:BOTTOM-TOOL-001 |
| **Status** | partial |
| **Evidence** | `BottomToolbelt.tsx` |

#### BOTTOM-TOOL-002 — Ruler tool

| Field | Value |
|-------|-------|
| **Domain** | MEAS |
| **Goal** | Distance measurement |
| **Trigger** | Click Ruler |
| **Preconditions** | — |
| **Procedure** | `enabled:false` "(soon)" |
| **Postconditions** | — |
| **Inputs** | — |
| **Outputs** | — |
| **Edge cases** | Phase 8 |
| **Acceptance** | `- [ ] Disabled` |
| **Eden parity** | Eden:MEAS-RULER-001 |
| **Status** | disabled |
| **Evidence** | `BottomToolbelt.tsx` |

#### BOTTOM-TOOL-003 — Line of Sight tool

| Field | Value |
|-------|-------|
| **Domain** | MEAS |
| **Goal** | LoS analysis |
| **Trigger** | Click LoS |
| **Preconditions** | DEM |
| **Procedure** | `enabled:false` |
| **Postconditions** | — |
| **Inputs** | — |
| **Outputs** | — |
| **Edge cases** | Phase 8 + DEM |
| **Acceptance** | `- [ ] Disabled` |
| **Eden parity** | Eden:MEAS-LOS-001 |
| **Status** | disabled |
| **Evidence** | `BottomToolbelt.tsx` |

---

#### BOTTOM-OBJCOUNT-001 — Entity count readout (OBJ total + SEL selected) (T-058)

| Field | Value |
|-------|-------|
| **Domain** | BOTTOM |
| **Goal** | Scale telemetry — live placed-slot total + selection count for the T-059+ scale program |
| **Trigger** | Slot added / removed / pasted / deleted, or selection change |
| **Preconditions** | Map mounted |
| **Procedure** | `BottomToolbelt` subscribes to `slotsById` → memoized `selectSlotCount` (OBJ) and to `selection.ids.length` when `kind==='slot'` (SEL); mono `tabular-nums` block right of the X/Y/Z coords |
| **Postconditions** | OBJ = total placed slots; SEL = selected slot count (0 when none) |
| **Inputs** | `slotsById`, `selection` store slices |
| **Outputs** | Display only (no state written) |
| **Edge cases** | Empty mission → OBJ 0 / SEL 0; **does not** update on cursor move (separate T-057 channel); plain integer (no commas) so 100000+ doesn't break layout |
| **Acceptance** | `- [x] OBJ counts slotsById` `- [x] SEL tracks marquee/Ctrl/paste/delete` `- [x] No re-render on cursor move (T-057 guard)` |
| **Eden parity** | Eden:BOTTOM-HUD-001 (entity counter) |
| **Status** | working |
| **Ticket** | T-058 |
| **Evidence** | `selectors.ts` (`selectSlotCount`), `BottomToolbelt.tsx` |

---

#### PERF-BULK-PASTE-001 — Bulk paste/delete at scale (T-059)

| Field | Value |
|-------|-------|
| **Domain** | PERF |
| **Goal** | Paste **10k** slots without browser hard-freeze; one undo step; pan ≥55 fps after |
| **Trigger** | Ctrl/Cmd+V with large clipboard; bulk delete |
| **Preconditions** | T-056 copy/paste working; T-058 OBJ readout for verification |
| **Procedure** | Batch `slotIds`/`entityIds` append in `pasteSlots`; cap selection ids >500; outliner virtualization (T-064 supersedes T-059 leaf cap) |
| **Postconditions** | OBJ correct; tab responsive; undo reverts entire paste |
| **Inputs** | `ClipboardSlot[]`, cursor anchor, active layer |
| **Outputs** | New slot ids (selection capped when bulk) |
| **Edge cases** | Paste 10k → selection cleared not 10k ids; large folders scroll via virtual outliner (T-064) |
| **Acceptance** | `- [x] Paste 10k no hard freeze` `- [x] OBJ correct` `- [x] Pan ≥55 fps after` `- [x] Undo one step` `- [x] 360k @ 100+ fps pan validated` |
| **Eden parity** | Eden:ACTION-PASTE-001 (bulk scale) |
| **Status** | working |
| **Ticket** | T-059 |
| **Evidence** | `state/ydoc.ts` (`pasteSlots`), `MissionCreatorPage.tsx`, `VirtualOutliner.tsx`, `flattenOutliner.ts` |

---

#### PERF-OUTLINER-001 — Virtualized outliner @ scale (T-064)

| Field | Value |
|-------|-------|
| **Domain** | PERF |
| **Goal** | Scroll through **100k–360k+** slot rows in ORBAT + Editor Layers without DOM explosion or tab freeze |
| **Trigger** | Expand a large squad/layer folder; scroll the left sidebar |
| **Preconditions** | T-060 deferred sidebar until `docStatus === 'ready'`; trees populated on first mount |
| **Procedure** | `@tanstack/react-virtual` in `VirtualOutliner.tsx`; segment-index flatten (`flattenOutliner.ts`); `virtualSlotIds` on folders with ≥50 slots (`VIRTUAL_SLOT_THRESHOLD`); callback-ref `scrollEl` (T-064.1) |
| **Postconditions** | Only ~viewport rows mounted; DnD/rename/select/dbl-click preserved |
| **Inputs** | ORBAT + Editor Layers tree models from Zustand |
| **Outputs** | Virtual row list in single `LeftSidebar` scroll container |
| **Edge cases** | Blank first frame if scroll element null — fixed T-064.1 via `scrollElement` state not RefObject |
| **Acceptance** | `- [x] Outliner visible on first paint @ ~367k` `- [x] Scroll 367k virtual rows` `- [x] No tab freeze` `- [x] DnD/reparent/root-drop/rename/delete` `- [x] Map pan/pick unchanged` |
| **Eden parity** | Eden outliner scroll (scale) |
| **Status** | working |
| **Ticket** | T-064 |
| **Evidence** | `VirtualOutliner.tsx`, `flattenOutliner.ts`, `TreeRow.tsx`, `LeftSidebar.tsx`, `EditorLayersSection.tsx`, `OrbatSection.tsx` |

---

#### PERF-CLUSTER-001 — Cluster / LOD @ extreme zoom (T-065)

| Field | Value |
|-------|-------|
| **Domain** | PERF |
| **Goal** | At extreme zoom-out on mega missions (>500 slots), show pan-stable cluster discs; at normal edit zoom keep full detail IconLayer @ ~160 fps |
| **Trigger** | Zoom to ≤ `ZOOM_CLUSTER_MAX` (-4) on mission with > `CLUSTER_SLOT_THRESHOLD` (500) slots |
| **Preconditions** | T-061 pan-stable icon cache; T-063 spatial pick; slotClusterIndex kept in sync via `slotIconCache` mutators |
| **Procedure** | `supercluster` in `slotClusterIndex.ts`; `getClusterMarkers` full-terrain pan-stable cache (T-065.2); `useClusterIconLayer` memoized on `markersVersion` + `iconCacheVersion`; `clusterMode` gates detail vs selected-only in `useIconLayer` |
| **Postconditions** | Default open zoom -2 = all rings; cluster drill-in via `pickClusterAt` → `flyTo` +1; selected slots visible in cluster band |
| **Inputs** | `slotIconCache` dense positions; Deck zoom; terrain bounds |
| **Outputs** | Cluster disc `IconLayer` (count-sized discs, no TextLayer) |
| **Edge cases** | ≤500 slots → never cluster; T-065.1 viewport-bbox pan stutter — fixed T-065.2; further cluster perf tuning deferred unless regression |
| **Acceptance** | `- [x] Detail @ -2 ~160 fps @ 367k` `- [x] Pan-stable cluster path (T-065.2)` `- [x] Git tag T-065` |
| **Eden parity** | Group icons when zoomed out (geo clusters v1 only) |
| **Status** | working |
| **Ticket** | T-065 |
| **Evidence** | `slotClusterIndex.ts`, `useClusterIconLayer.ts`, `constants.ts`, `TacticalMap.tsx`, `useSelectTool.ts`, `slotIconCache.ts` |

---

#### PERF-WORKER-001 — Compiler worker offload (T-066 + T-066.1)

| Field | Value |
|-------|-------|
| **Domain** | PERF |
| **Goal** | Run `compileMissionWithProgress` + `buildVersionBlob` off main thread @ 367k+; stretch ≤10 s @ 1M |
| **Trigger** | Save Version or Export on large mission |
| **Preconditions** | T-060 compile progress + T-060.1.2 blob streaming; **T-066.1:** `pickMapSnapshot(getState())` before Comlink RPC |
| **Procedure** | `compiler.worker.ts` + `compilerClient.ts` (Comlink); `useMissionEditor` Save + Export; `terminateCompiler` on unmount |
| **Postconditions** | Same `MissionPayload` / POST contract; Save progress phases unchanged |
| **Inputs** | `MapSnapshot` via `pickMapSnapshot(useMapStore.getState())` — **never raw getState()** |
| **Outputs** | `MissionPayload` or version POST `Blob` |
| **Edge cases** | Raw `getState()` → DataCloneError 25 (T-066.1 fixed); Export `JSON.stringify` still on main thread |
| **Acceptance** | `- [x] Save 201 @ ~367k` `- [x] pickMapSnapshot hotfix` `- [x] Git tag T-066` |
| **Eden parity** | n/a (infra) |
| **Status** | shipped |
| **Ticket** | T-066 |
| **Evidence** | `compiler.worker.ts`, `compilerClient.ts`, `compile.ts`, `useMissionEditor.ts`, `pickMapSnapshot` in `useMapStore.ts`; spec [`t066_worker_compile.md`](t066_worker_compile.md) |

---

#### PERF-CHUNK-001 — Spatial chunks / bulk-paste scale (T-067)

| Field | Value |
|-------|-------|
| **Domain** | PERF |
| **Goal** | Bulk paste O(k) not O(n) snapshot; chunk scaffolding for 1M+ lazy RAM / GPU cull |
| **Trigger** | Ctrl+V bulk paste; (future) viewport crosses chunk @ 1M |
| **Preconditions** | T-062 incremental bindings; T-066 worker save unchanged |
| **Procedure** | **Shipped:** `slot-add-bulk` in `incPatchPlan` → `_patchAddSlotsBulk`; dormant `chunkBuckets` in `slotIconCache`. **Render:** `getBaseIcons()` (CPU cull reverted T-067.0.1). **Follow-on (`idea`):** **T-111** lazy RAM; **T-112** GPU `DataFilterExtension` |
| **Postconditions** | Paste ≤10k avoids full snapshot; pan ~160 fps @ 367k zoom -2 |
| **Inputs** | Added slot ids from Y.Doc txn; (future) viewport bounds |
| **Outputs** | O(k) store + cache updates on bulk paste |
| **Edge cases** | Structural squad/layer paste → full snapshot fallback (unchanged) |
| **Acceptance** | `- [x]` pan ~160 fps @ 367k `- [x]` build/lint `- [x]` 6k paste `- [x]` pick/drag/cluster `- [~]` Save 201 (no T-067 save-path change; repro mission needs local DB seed) |
| **Eden parity** | n/a (infra) |
| **Status** | **shipped** (bulk paste + scaffolding; CPU cull deferred) |
| **Ticket** | T-067 |
| **Evidence** | `incPatchPlan.ts`, `useMapStore._patchAddSlotsBulk`, `spatialChunks.ts`, `slotIconCache.ts` chunk buckets, `useIconLayer.ts` → `getBaseIcons()`; spec [`t067_spatial_chunks.md`](t067_spatial_chunks.md); follow-ons [`T-111`/`T-112`](../../TICKET_BRAINSTORM.md#scale) |

---

#### PERF-LOAD-001 — Fast initial load / hydrate gate (T-060 + T-060.1 + T-060.1.1)

| Field | Value |
|-------|-------|
| **Domain** | PERF |
| **Goal** | Open **10k–1M** missions with **determinate progress bar**; coalesce boot snapshots; **≤10 s ideal @ 1M** (stretch → **T-066** worker) |
| **Trigger** | Navigate to `/missions/:id/edit` |
| **Preconditions** | Y.Doc persisted in IndexedDB (possibly 100k–360k+ slots) |
| **Procedure** | Bulk-sync window; `docStatus` gate; **four-phase** overlay: **restoring** (v2 chunked `loadSlotsWithProgress` T-062.1, or legacy y-indexeddb poll T-060.1.1) → download → apply → local flush; `docToSnapshotWithProgress` + `hydrateMissionDocWithProgress` + `onDownloadProgress`; `endBulkSync` async after hydrate; LeftSidebar deferred until ready |
| **Postconditions** | Map interactive; OBJ correct; pan ≥55 fps |
| **Inputs** | v2 `tbd-mission-persist` (`idb`) and/or legacy y-indexeddb; optional server `json_payload` |
| **Outputs** | `docStatus: ready`; `loadProgress` with determinate % (v2 restoring has `done/total`) |
| **Edge cases** | Legacy v1: one-time blocking replay + migrate. v2: smooth restoring @ ~360k. Server-adopted mission not cached to v2 until first `LOCAL_ORIGIN` edit. Warm return skips GET (T-062.2) |
| **Acceptance** | `- [x] Overlay + bulk-sync (T-060)` `- [x] Determinate % + chunked snapshot/hydrate (T-060.1)` `- [x] Hydrate inside bulk window (T-060.1)` `- [x] Restoring phase (T-060.1.1 legacy)` `- [x] v2 chunked IDB restore (T-062.1)` `- [ ] Pan regression clean (manual)` |
| **Eden parity** | — |
| **Status** | **shipped (T-060 + T-062.1)** — v2 determinate restore @ ~360k |
| **Ticket** | T-060 |
| **Evidence** | `useMissionDoc.ts`, `persistence/*`, `bindings.ts`, `ydoc.ts`, `MissionCreatorPage.tsx`, `t062_1_idb_streaming_load.md` |

---

#### PERF-SAVE-001 — Fast Save Version + progress (T-060 + T-060.1 + T-060.1.2 + T-060.1.3 + T-060.1.4)

| Field | Value |
|-------|-------|
| **Domain** | PERF |
| **Goal** | **Save Version** with **progress bar**; compile + POST without hard-freeze at **50k+**; **API accepts payloads >> 1 MB** (256 MB route limit — T-060); **≤10 s ideal @ 1M** |
| **Trigger** | User clicks Save → Save Version in TopCommandStrip |
| **Preconditions** | Valid mission UUID; dirty or explicit save |
| **Procedure** | `compileMissionWithProgress` → **`preparing`** (✅ chunked `buildVersionBlob`) → **`uploading`** (✅ Blob POST; **E3b:** auto direct `:8080` in dev when body >1 MB) → 256 MB cap, `timeout: 600_000` |
| **Postconditions** | Version 201; dirty cleared |
| **Inputs** | `useMapStore` snapshot |
| **Outputs** | POST body; progress UI |
| **Edge cases** | Mid-upload `ERR_NETWORK` @ ~4% / ~135 MB → **FIXED T-060.1.4**; payload >256 MB → pre-gate 413; **T-062.1.1:** Save omits duplicate `orbat[]` (editor-only POST); Go derives ORBAT for events |
| **Acceptance** | `- [x] E1/E2/E3b` `- [x] SZ + Save dialog (T-060.1.3)` `- [x] browser Save @ ~367k → 201` `- [x] T-062.1.1 IT: editor-only → event ORBAT` `- [x] Manual Save @ ~367k: ~94.8 MB estimated (~33% smaller vs ~141 MB)` |
| **Status** | **shipped** — T-060..T-060.1.4 + **T-062.1.1** orbat dedup |
| **Ticket** | T-060 |
| **Evidence** | `useMissionEditor.ts`, `compiler/compile.ts`, `lib/missionSize.ts`, `internal/services/mission_payload.go`, `internal/handlers/missions_orbat_integration_test.go`, `t062_1_1_batch_save.md` |

---

#### PERF-DRAG-001 — Drag-move preview @ 360k (T-061)

| Field | Value |
|-------|-------|
| **Domain** | PERF |
| **Goal** | **Drag-move** of selected slots acceptable @ ~360k (motion, pickup, release); preview transient (Y.Doc commit on pointer-up) |
| **Trigger** | Left-drag a selected slot icon (or selection) on the map |
| **Preconditions** | T-057 pan/cursor path shipped; mission has many slots (stress @ 360k) |
| **Procedure** | T-061.0: dual IconLayer + split drag state + rAF delta. T-061.0.1: `slotIconCache` O(k) + bindings slot fast path (`fastSlotPatchIds`) |
| **Postconditions** | Icons follow pointer smoothly; commit on release; undo reverts |
| **Inputs** | Pointer gesture in `useSelectTool` |
| **Outputs** | Transient store preview; Y.Doc update on release |
| **Edge cases** | 1 vs N selected; asset palette drop lag → **resolved T-062** (slot-add path). Release: possible single dropped frame (two cache bumps) — **deferred** |
| **Acceptance** | `- [x] Drag motion 1 slot @ 360k ≥55 fps` `- [x] Drag motion ~10 selected @ 360k ≥55 fps` `- [x] Pickup/release good enough (not perfect)` `- [x] build + lint clean` `- [x] Full regression sweep documented` |
| **Eden parity** | Eden:XFORM-MOVE-001 |
| **Status** | **shipped (good enough)** — spec [`t061_drag_move_hotfix.md`](t061_drag_move_hotfix.md) |
| **Ticket** | T-061 |
| **Evidence** | `slotIconCache.ts`, `useMapStore.ts`, `bindings.ts`, `useIconLayer.ts`, `useSelectTool.ts`, `selectors.ts`, `TacticalMap.tsx` |

---

#### PERF-PICK-001 — Spatial index for pick/marquee @ 360k (T-063)

| Field | Value |
|-------|-------|
| **Domain** | PERF |
| **Goal** | **Click, dbl-click, drag-start, marquee** pick without scanning all ~367k icons |
| **Trigger** | LMB click, dbl-click, drag-start, marquee release on map |
| **Preconditions** | Mission loaded with many slots; pan/drag paths already fast (T-057, T-061) |
| **Procedure** | rbush R-tree in `slotSpatialIndex.ts`; maintained alongside `slotIconCache`; `pickNearest` / `pickRect` replace `deck.pickObject` / `pickObjects`; `slot-icons` `pickable: false` |
| **Postconditions** | Selection unchanged vs Eden contract; no GPU pick pass on IconLayer |
| **Inputs** | Screen px + viewport (point picks); world bbox (marquee) |
| **Outputs** | `selection.ids[]`; Attributes via dbl-click |
| **Edge cases** | Overlapping icons → nearest to click; Ctrl/Cmd toggle (T-053); drag exclude no tree change |
| **Acceptance** | `- [x] Click @ 367k instant` `- [x] Marquee no multi-s freeze` `- [x] Dbl-click Attributes` `- [x] Pan/drag unchanged` `- [x] build + lint clean` |
| **Status** | **shipped** — spec [`t063_spatial_index.md`](t063_spatial_index.md) |
| **Ticket** | T-063 |
| **Evidence** | `slotSpatialIndex.ts`, `slotIconCache.ts`, `useSelectTool.ts`, `TacticalMap.tsx`, `useIconLayer.ts` |

---

#### PERF-BIND-001 — Incremental bindings @ 360k (T-062)

| Field | Value |
|-------|-------|
| **Domain** | PERF |
| **Goal** | Patch Zustand O(k) on everyday Y.Doc edits @ ~360k instead of full `docToSnapshot(n)` |
| **Trigger** | Asset drop, Delete, title/env edit, outliner layer rename/reparent/move |
| **Preconditions** | T-061 drag path shipped; mission @ scale (stress @ 360k) |
| **Procedure** | **T-062.0:** `incPatchPlan.classifyTransaction` → `PatchPlan` (`slot-fields`, `slot-add`, `slot-remove`, `meta`, `editor-layers`) → store patch methods + `slotIconCache.append`/`remove`. **T-062.0.1:** batched `removeEntities('slots')`; `slotCount`/`slotsRevision`; `REMOVE_PATCH_CAP` 10_000 |
| **Postconditions** | Store mirror updated without `e.slots.toJSON()` over all slots for classified txns |
| **Inputs** | Y.Doc observer events |
| **Outputs** | Incremental Zustand patches; `iconCacheVersion` bumps |
| **Edge cases** | Bulk paste / `addEditorLayer` / empty-doc bootstrap / `removeEditorLayer` → full snapshot fallback. Undo large multi-delete → full snapshot (verified OK @ 6k undo). IDB 0→300k jump → **fixed T-062.1** (v2 chunked restore). `_patchSlots` drag release still O(n) spread — deferred mega opt |
| **Acceptance** | `- [x] Asset drop instant @ 360k` `- [x] Delete 150/4000 no crash` `- [x] Drag not regressed` `- [x] Undo 6000 delete OK` `- [x] build + lint clean` |
| **Eden parity** | Eden:XFORM-PLACE-001 (drop), Eden:DELETE-001 |
| **Status** | **shipped** — spec [`t062_incremental_bindings.md`](t062_incremental_bindings.md) |
| **Ticket** | T-062 |
| **Evidence** | `incPatchPlan.ts`, `bindings.ts`, `useMapStore.ts`, `slotIconCache.ts`, `ydoc.ts`, `BottomToolbelt.tsx`, `EditorLayersSection.tsx`, `OrbatSection.tsx` |

#### PERF-SESSION-001 — Editor session / alt-tab (T-062.2)

| Field | Value |
|-------|-------|
| **Domain** | PERF |
| **Goal** | Alt-tab back to editor without automatic full load overlay; warm same-tab return skips multi-MB server GET |
| **Trigger** | Extended background tab (dev: Vite WS disconnect); same-tab reload with warm `sessionStorage` marker |
| **Procedure** | Dev: `viteReloadGuard` blocks `vite:beforeFullReload` on `/missions/:id/edit`. `editorSession.ts` marks ready → on reboot, `onSynced` skips GET when warm + `hasLocalContent(md)`. `yieldToUi` + restore poll visibility-aware |
| **Edge cases** | Warm path trusts local v2 store — remote server changes undetected until cold load. New tab = cold. `dirty` UI flag resets on reload (data in IDB). Undo stack session-only |
| **Acceptance** | `- [x] Alt-tab 30+ min → no overlay (Firefox dev @ ~360k)` `- [x] Edits preserved` `- [x] Cold load unchanged` |
| **Status** | **shipped** — spec [`t062_2_editor_session_persistence.md`](t062_2_editor_session_persistence.md) |
| **Ticket** | T-062.2 |
| **Evidence** | `viteReloadGuard.ts`, `editorSession.ts`, `useMissionEditor.ts`, `useMissionDoc.ts`, `yieldToUi.ts` |

#### PERF-IDB-001 — Chunked IDB slot restore (T-062.1)

| Field | Value |
|-------|-------|
| **Domain** | PERF |
| **Goal** | Restore ~360k slots from local persistence with **determinate** restoring progress (no 0→300k jump on 2nd+ load) |
| **Trigger** | Navigate to `/missions/:id/edit` when v2 `tbd-mission-persist` exists (or legacy v1 → migrate once) |
| **Preconditions** | T-062.2 warm path; T-060 bulk-sync window |
| **Procedure** | v2: `loadMissionMetaIntoDoc` → `loadSlotsWithProgress` (5k/chunk, `INIT_ORIGIN`, `yieldToUi`) — **no** y-indexeddb. Legacy: y-indexeddb replay → `migrateLegacyToV2` → delete `tbd-mission-${id}`. Writes: debounced meta + slot save on `LOCAL_ORIGIN`; flush on tab hide/pagehide |
| **Postconditions** | Y.Doc populated; overlay shows smooth `done/total` during v2 restoring |
| **Edge cases** | Server-adopted mission not v2-cached until first edit. SPA navigate-away within ~2s debounce may drop last edits. `docAlive` / `isCancelled` guards prevent corrupt writes on teardown |
| **Acceptance** | `- [x] Migration once` `- [x] 2nd+ load smooth progress @ ~360k` `- [x] Legacy DB deleted` `- [x] build/lint/tsc clean` |
| **Status** | **shipped** — spec [`t062_1_idb_streaming_load.md`](t062_1_idb_streaming_load.md) |
| **Ticket** | T-062.1 |
| **Evidence** | `persistence/*`, `useMissionDoc.ts`, `useMissionEditor.ts`, `ydoc.ts` (`entityToYMap`) |

---

## ATTR — Attributes & settings

#### ATTR-MODAL-001 — Attributes dialog shell

| Field | Value |
|-------|-------|
| **Domain** | ATTR |
| **Goal** | Per-entity property editor |
| **Trigger** | Dbl-click slot (map or tree) |
| **Preconditions** | Valid `slotId` |
| **Procedure** | `Dialog open={slotId!=null}`; resolve slot from store |
| **Postconditions** | Modal open/closed |
| **Inputs** | Dbl-click, close |
| **Outputs** | UI |
| **Edge cases** | Missing slot → fallback title |
| **Acceptance** | `- [ ] Modal shows slot role` |
| **Eden parity** | Eden:ATTR-OPEN-001 |
| **Status** | working |
| **Evidence** | `AttributesModal.tsx` |

#### ATTR-TAB-001 — Transform tab (editable position, T-049)

| Field | Value |
|-------|-------|
| **Domain** | ATTR |
| **Goal** | View/edit transform |
| **Trigger** | Transform tab |
| **Preconditions** | Slot open |
| **Procedure** | Editable X/Y/Z/rotation via mono `NumberField` (blur/Enter commit) → `updateSlotPosition` (x/y clamped to terrain, rotation normalized 0–360); stance `SelectField` → `updateSlot` |
| **Postconditions** | Slot position / stance update (one undo step per commit); map icon re-renders |
| **Inputs** | X/Y/Z/rotation numbers, stance select |
| **Outputs** | Y.Doc slot `position` / `stance` |
| **Edge cases** | T-049: stale "coming later" copy replaced; non-finite input ignored (no NaN write); Z manual until DEM |
| **Acceptance** | `- [x] Position editable (T-049)` `- [x] Stance editable` |
| **Eden parity** | Eden:ATTR-XFORM-001 |
| **Status** | partial |
| **Ticket** | T-049 |
| **Evidence** | `AttributesModal.tsx` |

#### ATTR-TAB-002 — Identity tab

| Field | Value |
|-------|-------|
| **Domain** | ATTR |
| **Goal** | Role, tag, squad display |
| **Trigger** | Identity tab (default) |
| **Preconditions** | Slot open |
| **Procedure** | `updateSlot` role/tag; squad name read-only |
| **Postconditions** | Identity fields saved |
| **Inputs** | Text fields |
| **Outputs** | Y.Doc |
| **Edge cases** | Tab state not reset per slot open |
| **Acceptance** | `- [ ] Role edit persists` |
| **Eden parity** | Eden:ATTR-ID-001 |
| **Status** | working |
| **Evidence** | `AttributesModal.tsx` |

#### ATTR-TAB-003 — States tab (stub)

| Field | Value |
|-------|-------|
| **Domain** | ATTR |
| **Goal** | Medic/engineer flags |
| **Trigger** | States tab |
| **Preconditions** | — |
| **Procedure** | Toggles `(soon)` `onChange={()=>{}}` |
| **Postconditions** | None |
| **Inputs** | — |
| **Outputs** | — |
| **Edge cases** | Not in compiler |
| **Acceptance** | `- [ ] Toggles non-functional` |
| **Eden parity** | Eden:ATTR-STATES-001 |
| **Status** | stub |
| **Evidence** | `AttributesModal.tsx` |

#### ATTR-TAB-004 — Arsenal tab (dumb loadout export @ T-068.4)

| Field | Value |
|-------|-------|
| **Domain** | ATTR |
| **Goal** | Loadout editor |
| **Trigger** | Arsenal tab |
| **Preconditions** | Exactly one slot selected |
| **Procedure (today)** | **4 enabled gear dropdowns** + **Download loadout JSON** (character slots only) |
| **Procedure (T-068.4 ship)** | Same — shipped @ `a85f16b` |
| **Postconditions** | Downloaded JSON validates against `loadout-export.schema.json` |
| **Inputs** | Registry gear rows (`gear_primary`, `gear_uniform`, `gear_vest`, `gear_helmet`) |
| **Outputs** | Browser download of flat loadout JSON |
| **Edge cases** | Paper-doll / attachments deferred to T-068.10 |
| **Acceptance** | `- [x] Stub only (pre-T-068.4)` · `- [x] T-068.4: stub removed; dropdowns + download work` |
| **Eden parity** | Eden:ATTR-ARSENAL-001 (dumb export Phase 1; smart Forge Phase 2) |
| **Status** | working (dumb export) |
| **Evidence** | `AttributesModal.tsx` (`ArsenalTab`) · `loadout/loadoutExport.ts` · ship spec [`t068_4_dumb_loadout_ui.md`](t068_4_dumb_loadout_ui.md) @ `a85f16b` |

---

## DATA — Persistence & compile

#### DATA-IDB-001 — Local persistence (v2 chunked IDB, T-062.1)

| Field | Value |
|-------|-------|
| **Domain** | DATA |
| **Goal** | Local crash-safe draft per mission with **incremental restore** @ scale |
| **Trigger** | `LOCAL_ORIGIN` Y.Doc edits once `docStatus === 'ready'` (debounced ~2s) |
| **Preconditions** | Editor mounted; v2 path uses `idb` `tbd-mission-persist` |
| **Procedure** | **v2:** `saveMissionMetaFromDoc` + `saveSlotsFromDocDebounced` (5k chunks) → flush on tab hide/pagehide/unmount. **Boot:** v2 chunked restore, or legacy v1 y-indexeddb replay → one-time `migrateLegacyToV2` → delete `tbd-mission-${id}`. **Fresh:** no v2 write until first user edit |
| **Postconditions** | Meta + slot chunks in `tbd-mission-persist`; legacy DB removed after migration |
| **Inputs** | Y.Doc edits (`LOCAL_ORIGIN`) |
| **Outputs** | IndexedDB (`meta` + `slots` stores) |
| **Edge cases** | Server-adopted mission not cached until first edit. SPA navigate-away within debounce may drop last edits. `isCancelled` guard on teardown. `missionId` undefined → draft key |
| **Acceptance** | `- [x] F5 restores edits (v2)` `- [x] 2nd+ load determinate progress @ ~360k` `- [x] Legacy migrate once` |
| **Eden parity** | N/A |
| **Status** | **working (v2)** — legacy y-indexeddb only on first-return migrate path |
| **Ticket** | T-062.1 |
| **Evidence** | `persistence/*`, `useMissionDoc.ts`, `useMissionEditor.ts`, `t062_1_idb_streaming_load.md` |

#### DATA-SEED-001 — Default meta + layer seed

| Field | Value |
|-------|-------|
| **Domain** | DATA |
| **Goal** | Bootstrapped empty mission |
| **Trigger** | First IndexedDB sync empty doc |
| **Preconditions** | No meta/layers |
| **Procedure** | `seedMeta`, `seedDefaultLayer` with `INIT_ORIGIN` |
| **Postconditions** | Untitled + default folder |
| **Inputs** | — |
| **Outputs** | Y.Doc |
| **Edge cases** | Not undoable |
| **Acceptance** | `- [ ] New mission has default folder` |
| **Eden parity** | N/A |
| **Status** | working |
| **Evidence** | `useMissionDoc.ts`, `ydoc.ts` |

#### DATA-HYD-001 — Server hydrate + ORBAT fallback

| Field | Value |
|-------|-------|
| **Domain** | DATA |
| **Goal** | Load server version into Y.Doc |
| **Trigger** | Auto or conflict resolution |
| **Preconditions** | `json_payload` available |
| **Procedure** | `hydrateMissionDoc` — prefer `editor` block; else lossy `orbat[]` rebuild |
| **Postconditions** | Doc matches server |
| **Inputs** | API payload |
| **Outputs** | Y.Doc `INIT_ORIGIN` |
| **Edge cases** | ORBAT-only path loses layer positions |
| **Acceptance** | `- [ ] Load saved version restores slots` |
| **Eden parity** | Eden:FILE-OPEN-001 |
| **Status** | working |
| **Evidence** | `ydoc.ts` `hydrateMissionDoc`, `useMissionEditor.ts` |

#### DATA-COMP-001 — Compile to json_payload

| Field | Value |
|-------|-------|
| **Domain** | DATA |
| **Goal** | Backend contract export |
| **Trigger** | Save / Export |
| **Preconditions** | Store snapshot |
| **Procedure** | `compileMission` → `orbat[]`, environment, `editor` block, empty vehicles/markers/objectives arrays |
| **Postconditions** | `MissionPayload` |
| **Inputs** | Store |
| **Outputs** | JSON |
| **Edge cases** | `loadout` always `''` |
| **Acceptance** | `- [ ] Compiled orbat matches placed units` |
| **Eden parity** | Eden:FILE-EXPORT-001 |
| **Status** | working |
| **Evidence** | `compiler/compile.ts` |

#### DATA-COLLAB-001 — Real-time multiplayer editing

| Field | Value |
|-------|-------|
| **Domain** | DATA |
| **Goal** | Multi-user Yjs sync |
| **Trigger** | N/A |
| **Preconditions** | — |
| **Procedure** | Not implemented — local only |
| **Postconditions** | — |
| **Inputs** | — |
| **Outputs** | — |
| **Edge cases** | Future y-websocket |
| **Acceptance** | `- [ ] N/A` |
| **Eden parity** | N/A (Eden solo) |
| **Status** | not_built |
| **Evidence** | `useMissionDoc.ts` |

---

## KEY — Keyboard (host-wired)

| ID | Status | Ticket | Trigger | Evidence |
|----|--------|--------|---------|----------|
| KEY-SPACE-001 | working | — | `Space` → flyTo selection | `MissionCreatorPage.tsx` |
| KEY-DEL-001 | working | — | Delete/Backspace → remove slots | `MissionCreatorPage.tsx` |
| KEY-UNDO-001 | working | T-052 | Buttons + Cmd/Ctrl+Z/Shift+Z/Ctrl+Y keyboard (T-052) | `TopCommandStrip.tsx`, `MissionCreatorPage.tsx`, `useMissionDoc.ts` |
| KEY-COPY-001 | working | T-056 | Ctrl/Cmd+C copy slot selection + Ctrl/Cmd+V paste at cursor (relative layout; off-map +20m nudge) (T-056). **Bulk scale (T-059):** `pasteSlots` batch O(n) append (no O(n²) spreads); paste auto-selects only when ≤500 (`BULK_SELECT_CAP`), else clears to `none`. **Outliner (T-064):** virtualized scroll replaces T-059 `OUTLINER_LEAF_CAP` → 10k+ paste without hard freeze | `MissionCreatorPage.tsx`, `state/ydoc.ts` (`pasteSlots`), `state/schema.ts` (`ClipboardSlot`), `VirtualOutliner.tsx`, `LeftOutliner/EditorLayersSection.tsx`, `LeftOutliner/OrbatSection.tsx` |
| KEY-SELALL-001 | not_built | — | Ctrl+A | — |

---

## FILE — Routing

#### FILE-ROUTE-001 — Lazy code-split editor route

| Field | Value |
|-------|-------|
| **Domain** | FILE |
| **Goal** | Defer Deck.gl/Yjs bundle |
| **Trigger** | First visit `/missions/:id/edit` |
| **Preconditions** | `mission_maker+` |
| **Procedure** | `lazy(() => import('./MissionCreatorPage'))` + Suspense |
| **Postconditions** | Chunk loaded |
| **Inputs** | Navigation |
| **Outputs** | Editor mount |
| **Edge cases** | Loading fallback |
| **Acceptance** | `- [ ] Network shows lazy chunk on first edit` |
| **Eden parity** | N/A |
| **Status** | working |
| **Evidence** | `routes.ts`, `router.tsx` |

---

## TBD-only features (not Eden)

| ID | Name | Status |
|----|------|--------|
| TBD-LAYER-001 | Editor Layers workflow folders | working |
| TBD-SAVE-001 | Semver Save Version (immutable) | working |
| TBD-EXPORT-001 | `editor` block lossless reload | working |
| TBD-CONFLICT-001 | IndexedDB vs server conflict UI | working |

---

## Second-pass audit supplement (2026-06-20)

Items verified in code + scrape cross-check; added after initial `06` draft.

### Critical corrections

| ID | Fix |
|----|-----|
| LEFT-LAYER-009 | MIME is `application/x-tbd-tree-node` (not `x-tbd-tree`) |
| XFORM-DEL-001 | Precondition is `kind !== 'none'`, not `kind === 'slot'` |
| PLACE-DROP-001 | Mock vehicles/props under **Factions** tree create slots |
| PLACE-CLICK-001 | Reclassified `stub` → `not_built` (no UI sets place tool) |

### New TBD features (were missing from 06)

#### MAP-TERRAIN-001 — Terrain wired to viewport (T-049)

| Field | Value |
|-------|-------|
| **Domain** | MAP |
| **Goal** | Map uses the mission's terrain bounds |
| **Trigger** | Page mount / terrain change |
| **Procedure** | `terrainId = meta.terrain ?? 'everon'`; `<TacticalMap key={terrainId} terrain={terrainId}>` — `key` remounts the viewport so camera + base map resize (Everon 12800 / Arland 10240) |
| **Status** | done (T-049) |
| **Ticket** | T-049 |
| **Evidence** | `MissionCreatorPage.tsx` (`terrainId` selector + `key`/`terrain` props); `MissionSettingsDialog.tsx` reads `meta.terrain` |

#### MAP-MARQUEE-VIS-001 — Live marquee overlay

| Field | Value |
|-------|-------|
| **Domain** | SEL |
| **Goal** | Visual box during marquee select |
| **Procedure** | `useSelectionLayer` PolygonLayer during drag |
| **Status** | working |
| **Evidence** | `layers/useSelectionLayer.ts`, `TacticalMap.tsx` |

#### MAP-DRAG-PREVIEW-001 — Live drag offset preview

| Field | Value |
|-------|-------|
| **Domain** | XFORM |
| **Goal** | Icons follow pointer before Y.Doc commit |
| **Procedure** | Transient `drag` in store → `selectSlotIcons` offset |
| **Status** | working |
| **Evidence** | `useSelectTool.ts`, `selectors.ts` |

#### SEL-ORBAT-DBL-001 — ORBAT dbl-click opens attributes

| Field | Value |
|-------|-------|
| **Domain** | SEL |
| **Goal** | Eden parity: tree dbl-click opens attrs |
| **Procedure** | T-054: `OrbatSection` gains `onActivateSlot` (threaded from `LeftSidebar`) and passes `onActivate` to its `TreeView` — mirrors `EditorLayersSection`; `TreeView` fires it on a slot row's native `onDoubleClick` → `setAttributesId` |
| **Status** | working |
| **Ticket** | T-054 |
| **Evidence** | `OrbatSection.tsx`, `LeftSidebar.tsx`, `TreeView.tsx` L190 |

#### SEL-ORBAT-MULTI-001 — ORBAT click collapses multi-select

| Field | Value |
|-------|-------|
| **Domain** | SEL |
| **Trigger** | Click ORBAT slot row |
| **Procedure** | Always `setSelection({ ids: [id] })` — drops marquee selection |
| **Status** | partial |
| **Evidence** | `OrbatSection.tsx` |

#### PLACE-DROP-002 — Mock vehicle/prop → slot (wrong entity type)

| Field | Value |
|-------|-------|
| **Domain** | PLACE |
| **Goal** | Place vehicles/objects as correct entity kinds |
| **Procedure** | **T-068.3:** mock MRAP/sandbag leaves removed — palette is character-only from registry. Vehicles/objects → **T-070** |
| **Status** | deferred (T-070) |
| **Evidence** | `buildCatalogTree.ts` filters `kind === 'character'`; `assetCatalogMock.ts` deleted |

#### ENV-SETTINGS-002 — Thermals + view distance

| Field | Value |
|-------|-------|
| **Domain** | ENV |
| **Goal** | Global environment beyond time/weather strip |
| **Procedure** | `MissionSettingsDialog`: view distance, thermals toggle |
| **Status** | working |
| **Evidence** | `MissionSettingsDialog.tsx` L57–74 |

#### DATA-HYD-TITLE-001 — Title not from API mission row

| Field | Value |
|-------|-------|
| **Domain** | DATA |
| **Goal** | Show server mission title on load |
| **Procedure** | `hydrateMissionDoc` sets env/terrain; `GET /missions/:id` title not applied to `meta.title` |
| **Status** | partial |
| **Ticket** | T-049 |
| **Evidence** | `useMissionEditor.ts`, `ydoc.ts` `hydrateMissionDoc` |

#### DATA-CONFLICT-EDGE-001 — Conflict scope

| Field | Value |
|-------|-------|
| **Domain** | DATA |
| **Procedure** | `hasLocalContent()` checks factions/slots/vehicles/markers/objectives only — title-only or layer-only local edits skip prompt |
| **Status** | working (by design) |
| **Evidence** | `useMissionEditor.ts` `hasLocalContent` |

#### API-ORBAT-001 — Programmatic faction/squad (no UI)

| Field | Value |
|-------|-------|
| **Domain** | ORBAT |
| **Procedure** | `addFaction`, `addSquad` in `ydoc.ts` — no editor UI |
| **Status** | not_built (UI) |
| **Evidence** | `ydoc.ts` L471+ |

#### LEFT-HIST-TAB-001 — History tab icon (visual only)

| Field | Value |
|-------|-------|
| **Domain** | LEFT |
| **Procedure** | `BOTTOM_TABS` includes `history` — no panel switch |
| **Status** | stub |
| **Evidence** | `LeftSidebar.tsx` |

#### ATTR-TAB-STALE-001 — Transform help text outdated

| Field | Value |
|-------|-------|
| **Domain** | ATTR |
| **Procedure** | T-049: copy now reads "Drag on the map or edit coordinates above. Z is manual until terrain elevation (DEM) ships." |
| **Status** | done (T-049) |
| **Evidence** | `AttributesModal.tsx` Transform tab |

#### KEY-TEXTAREA-001 — Shortcuts fire in TEXTAREA

| Field | Value |
|-------|-------|
| **Domain** | KEY |
| **Procedure** | Guard skips INPUT/SELECT/contentEditable but not TEXTAREA |
| **Status** | partial (edge case) |
| **Evidence** | `MissionCreatorPage.tsx` L63 |

### Expanded KEY table

| ID | Status | Ticket | Trigger | Evidence |
|----|--------|--------|---------|----------|
| KEY-REDO-001 | working | T-052 | Buttons + Cmd/Ctrl+Shift+Z / Ctrl+Y keyboard (T-052) | `TopCommandStrip.tsx`, `MissionCreatorPage.tsx`, `useMissionDoc.ts` |
| KEY-RENAME-001 | working | — | Enter/Escape in tree rename | `TreeView.tsx` L216–221 |
| KEY-DIALOG-001 | working | — | Conflict dialog cannot dismiss without choice | `MissionCreatorPage.tsx` `onOpenChange` noop |
