**Status:** live

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

| PLACE-DROP-001 | Mock vehicles/props under **Factions** tree create slots |
| PLACE-CLICK-001 | Reclassified `stub` → `not_built` (no UI sets place tool) |
#### PLACE-DROP-002 — Mock vehicle/prop → slot (wrong entity type)

| Field | Value |
|-------|-------|
| **Domain** | PLACE |
| **Goal** | Place vehicles/objects as correct entity kinds |
| **Procedure** | **T-068.3:** mock MRAP/sandbag leaves removed — palette is character-only from registry. Vehicles/objects → **T-070** |
| **Status** | deferred (T-070) |
| **Evidence** | `buildCatalogTree.ts` filters `kind === 'character'`; `assetCatalogMock.ts` deleted |

