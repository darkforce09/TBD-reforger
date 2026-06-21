# Mission Editor (2D Canvas)

## Status

`in-progress`

## Summary

- **What:** Full-bleed Deck.gl tactical map editor for placing entities, ORBAT layers, and compiling mission versions.
- **Why:** Mission makers author `json_payload` for the mission library and event ORBAT materialization.
- **Route:** `/missions/:id/edit`
- **Live source:** `frontend/src/features/mission-creator/` (wrapper) + `frontend/src/features/tactical-map/` (engine); lazy route in `frontend/src/router.tsx`
- **Stitch reference:** none
- **Min role:** `mission_maker` (owner or admin)
- **Blueprint ref:** [Mission Creator 02 — roadmap](../../Design_Docs/Mission_Creator_Architecture/02_roadmap.md), [06 — feature inventory](../../Design_Docs/Mission_Creator_Architecture/06_tbd_mission_creator_feature_inventory.md)

## Element Inventory

| # | Element | Type | Text / Content | Purpose | Data source |
|---|---------|------|----------------|---------|-------------|
| 1 | Map viewport | canvas | Tactical grid + entities | Primary editor surface | Y.Doc + Deck.gl |
| 2 | Top strip | bar | Title, undo/redo, time/weather scrubber, settings, Export | Command chrome | Mission metadata |
| 3 | Left dock | panel | ORBAT tree + Editor Layers | Outliner / drop target | `editorLayers` map |
| 4 | Right dock | panel | Asset Palette tabs | Drag assets to map | Mock catalog (registry pending) |
| 5 | Toolbelt | bar | Select, Ruler, LoS | Map tools | Tool state (ruler/LoS stub) |
| 6 | Inspector | panel | Slot fields on double-click | `AttributesModal` | Selected slot |
| 7 | Save Version | button | POST new semver | Immutable versions API | `useMissionEditor` |
| 8 | Export | button | Download mod envelope | Compiler | `compiler/exportSchema.ts` |

## Behavior

### Primary flow
1. Navigate from wizard (`/missions/create`) or mission dossier Edit action → `/missions/:id/edit`.
2. `useMissionDoc` hydrates Y.Doc from y-indexeddb; `useMissionEditor` loads current version from API with conflict prompt.
3. Author places entities via palette drop, moves selection on map, organizes layers in outliner.
4. **Save Version** → `POST /missions/:id/versions` with compiled `json_payload`.
5. **Export** downloads camelCase mod envelope without saving.

### States
- **Chromeless:** No platform Sidebar/TopNav (`fullBleed` + `chromeless` route handles).
- **Loading:** Version fetch + IndexedDB sync.
- **Dirty:** Local autosave to y-indexeddb; server save is manual semver POST.
- **Blocked phases:** DEM/Z-axis (Phase 2), asset registry (Phase 5/6), ruler/LoS viewshed (Phase 8).

## API Dependencies

| Endpoint | Method | When called | Response shape |
|----------|--------|-------------|----------------|
| `GET /missions/:id` | GET | Boot | `Mission` |
| `GET /missions/:id/versions/current` | GET | Hydrate | `MissionVersion` |
| `POST /missions/:id/versions` | POST | Save Version | `MissionVersion` (409 on dup semver) |
| `GET /missions/:id/export` | GET | Export preview | camelCase envelope |

## Milestones

### M1 — [x] Route + chromeless shell
### M2 — [x] Y.Doc state, map viewport, Eden docked panels
### M3 — [x] Map drag, multi-select, outliner ops, compiler + Save/Export
### M4 — [ ] DEM, registry worker, ruler/LoS tools (blocked on external assets/API)

## Test Plan

1. Mission maker opens `/missions/:id/edit` → map + docks render full-bleed.
2. Drop asset from palette → slot appears on map and in active layer.
3. Save Version with new semver → 201; duplicate semver → 409 with error surfaced.
4. Export downloads JSON without POST.

## Open Questions / Blockers

- [FD-003](../TRACKING.md): Phases 2/5/6/8 — see [Mission Creator hub](../../Design_Docs/Mission_Creator_Architecture/README.md).
