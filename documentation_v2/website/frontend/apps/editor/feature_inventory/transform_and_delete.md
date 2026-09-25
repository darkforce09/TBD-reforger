**Status:** live

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

| XFORM-DEL-001 | Precondition is `kind !== 'none'`, not `kind === 'slot'` |
#### MAP-DRAG-PREVIEW-001 — Live drag offset preview

| Field | Value |
|-------|-------|
| **Domain** | XFORM |
| **Goal** | Icons follow pointer before Y.Doc commit |
| **Procedure** | Transient `drag` in store → `selectSlotIcons` offset |
| **Status** | working |
| **Evidence** | `useSelectTool.ts`, `selectors.ts` |

