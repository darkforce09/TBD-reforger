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
- **Blueprint ref:** [Mission Creator ROADMAP](../../../Design_Docs/Mission_Creator_Architecture/ROADMAP.md), [feature inventory](../../../Design_Docs/Mission_Creator_Architecture/feature_inventory.md)

## Element Inventory

| # | Element | Type | Text / Content | Purpose | Data source |
|---|---------|------|----------------|---------|-------------|
| 1 | Map viewport | canvas | Tactical grid + entities | Primary editor surface | Y.Doc + Deck.gl |
| 2 | Top strip | bar | Title, undo/redo, time/weather scrubber, settings, Export | Command chrome; Undo/Redo buttons + **keyboard shortcuts** (Cmd/Ctrl+Z, Cmd/Ctrl+Shift+Z, Ctrl+Y) (T-052) | Mission metadata |
| 3 | Left dock | panel | ORBAT tree + Editor Layers | Outliner / drop target | `editorLayers` map |
| 4 | Right dock | panel | Asset Palette tabs + **Asset Browser search** | Drag assets to map; **search filters the Factions tree by name** (T-055) | Mock catalog (registry pending) |
| 5 | Toolbelt | bar | Select, Ruler, LoS + X/Y/Z readout | Map tools; **selection-aware** coords (SEL = single slot X/Y/Z, CUR = cursor X/Y/Z, Z=0 flat until DEM) (T-049, T-050) | Tool state + selection |
| 6 | Inspector | panel | Slot fields on double-click | `AttributesModal` opens from **map dbl-click**, **ORBAT** slot row, or **Editor Layers** slot row (T-054); **Transform X/Y/Z/rotation editable** (T-049) | Selected slot |
| 7 | Save Version | button | POST new semver | Immutable versions API | `useMissionEditor` |
| 8 | Export | button | Download mod envelope | Compiler | `compiler/exportSchema.ts` |

## Behavior

### Primary flow
1. Navigate from Mission Library **+ New Mission** dialog (T-048) or dossier **OPEN IN MISSION CREATOR** → `/missions/:id/edit`.
2. `useMissionDoc` hydrates Y.Doc from y-indexeddb; `useMissionEditor` loads current version from API with conflict prompt. The mission **row** (`title`, `terrain`, time/weather) is hydrated into `meta` on every load — including brand-new missions whose `json_payload` is still `{}` (T-049). Terrain drives the viewport bounds (Everon 12.8km vs Arland 10.24km).
3. Author places entities via palette drop, moves selection on map, organizes layers in outliner.
4. **Save Version** → `POST /missions/:id/versions` with compiled `json_payload`.
5. **Export** downloads camelCase mod envelope without saving.

### States
- **Chromeless:** No platform Sidebar/TopNav (`fullBleed` + `chromeless` route handles).
- **Loading:** Version fetch + IndexedDB sync.
- **Dirty:** Local autosave to y-indexeddb; server save is manual semver POST.
- **Blocked phases:** DEM/Z-axis (Phase 2), asset registry (Phase 5/6), ruler/LoS viewshed (Phase 8).

### Keyboard (host — `/missions/:id/edit`)
| Shortcut | Action |
|----------|--------|
| Space | Center camera on selection |
| Delete / Backspace | Remove selected slots (undoable) |
| Cmd/Ctrl+Z | Undo last edit |
| Cmd/Ctrl+Shift+Z or Ctrl+Y | Redo |
| Ctrl/Cmd + click (map icon) | Toggle slot in/out of selection; Ctrl/Cmd+empty map preserves selection (T-053) |
| Cmd/Ctrl+C / Cmd/Ctrl+V | Copy slot selection / paste at cursor (relative layout; off-map +20m nudge); pasted slots become the selection (T-056) |

Skipped when focus is in an input, select, textarea, or contentEditable field (T-052).

Double-click a slot on the **map**, in **ORBAT**, or in **Editor Layers** opens Attributes (T-054). Suppressed when multiple slots are selected.

Undo/redo applies to **session edits only** (drop, drag, delete, title/env changes via `LOCAL_ORIGIN`). Entities loaded from IndexedDB or server hydrate are not on the undo stack.

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
### M3.5 — [x] T-049 terrain wired to viewport; row title/terrain/env hydrate; editable Transform X/Y/Z/rotation; selection-aware toolbelt
### M3.6 — [x] T-050 cursor readout shows X/Y/Z (Z=0 on the flat map until DEM)
### M3.7 — [x] T-052 undo/redo keyboard shortcuts (Cmd/Ctrl+Z, Cmd/Ctrl+Shift+Z, Ctrl+Y)
### M3.8 — [x] T-053 Ctrl/Cmd+LMB additive toggle select on map
### M3.9 — [x] T-054 Attributes entry — map dbl-click + ORBAT tree dbl-click
### M3.10 — [x] T-055 Asset browser search (filters Factions tree; folder-name match shows subtree)
### M3.11 — [x] T-056 Ctrl+C/V copy-paste at cursor (slots; relative layout, off-map +20m nudge; one undo step)
### M4 — [ ] DEM, registry worker, ruler/LoS tools (blocked on external assets/API; Eden P0–P2 first)

## Test Plan

1. Mission maker opens `/missions/:id/edit` → map + docks render full-bleed.
2. Drop asset from palette → slot appears on map and in active layer.
3. Save Version with new semver → 201; duplicate semver → 409 with error surfaced.
4. Export downloads JSON without POST.
5. Drop or drag a unit → Undo enables → Cmd/Ctrl+Z reverts; redo restores.

## Open Questions / Blockers

- [FD-003](../TRACKING.md): Phases 2/5/6/8 — see [Mission Creator hub](../../../Design_Docs/Mission_Creator_Architecture/README.md).
