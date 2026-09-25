**Status:** live

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

#### MAP-MARQUEE-VIS-001 — Live marquee overlay

| Field | Value |
|-------|-------|
| **Domain** | SEL |
| **Goal** | Visual box during marquee select |
| **Procedure** | `useSelectionLayer` PolygonLayer during drag |
| **Status** | working |
| **Evidence** | `layers/useSelectionLayer.ts`, `TacticalMap.tsx` |

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

