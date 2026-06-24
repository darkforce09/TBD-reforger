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
| 5 | Toolbelt | bar | Select, Ruler, LoS + X/Y/Z readout + **OBJ/SEL counts** (T-058) | Map tools; CUR/SEL coords; **OBJ** = total slots, **SEL** = selected count | Tool state + `slotsById` + selection |
| 6 | Inspector | panel | Slot fields on double-click | `AttributesModal` opens from **map dbl-click**, **ORBAT** slot row, or **Editor Layers** slot row (T-054); **Transform X/Y/Z/rotation editable** (T-049) | Selected slot |
| 7 | Save Version | button | POST new semver | Immutable versions API | `useMissionEditor` |
| 8 | Export | button | Download mod envelope | Compiler | `compiler/exportSchema.ts` |

## Behavior

### Primary flow
1. Navigate from Mission Library **+ New Mission** dialog (T-048) or dossier **OPEN IN MISSION CREATOR** → `/missions/:id/edit`.
2. `useMissionDoc` hydrates Y.Doc from y-indexeddb; `useMissionEditor` reconciles with the server on **cold** load (`GET /missions/:id` + conflict prompt when both IDB and server have content). **T-062.2:** warm same-tab return (`editorSession` + `sessionStorage`) skips the multi-MB GET when IndexedDB already holds the mission; dev `viteReloadGuard` blocks Vite HMR full reload on alt-tab. The mission **row** (`title`, `terrain`, time/weather) hydrates into `meta` on cold load — including new missions whose `json_payload` is `{}` (T-049). Terrain drives viewport bounds. **T-060:** bulk-sync window + `docStatus` loading overlay (restoring → download → apply → local on cold; restoring → local only on warm); LeftSidebar deferred until ready.
3. Author places entities via palette drop, moves selection on map, organizes layers in outliner.
4. **Save Version** → `POST /missions/:id/versions` with compiled `json_payload`.
5. **Export** downloads camelCase mod envelope without saving.

### States
- **Chromeless:** No platform Sidebar/TopNav (`fullBleed` + `chromeless` route handles).
- **Loading:** Four-phase overlay on **cold** load: **restoring** (T-060.1.1 ✅) → download → apply → local flush. **Warm return** (T-062.2): restoring → local flush only (no download/apply). Manual @ ~360k cold: ~30 s–1 min; 0→~300k jump. **Dev alt-tab:** overlay should not reappear after extended background (T-062.2). **Save:** T-060.1.4 FIXED.
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
| `GET /missions/:id` | GET | **Cold** boot only (warm return skips — T-062.2) | `Mission` |
| `GET /missions/:id/versions/current` | GET | Hydrate | `MissionVersion` |
| `POST /missions/:id/versions` | POST | Save Version | `MissionVersion` (409 dup semver; **256 MB** route cap → **413** over it, T-060; other JSON routes stay 1 MB) |
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
### M3.12 — [x] T-057 Map perf hotfix (≥55 fps pan/zoom @ 200+ slots: cursor→store rAF, no Deck `onHover` pick, pan rAF-coalesce, `React.memo` panels)
### M3.13 — [x] T-058 Toolbelt OBJ/SEL entity counts (total placed slots + selected count; scale telemetry)
### M3.14 — [x] T-059 Bulk paste/delete at scale (batch O(n) append; selection cap 500; outliner leaf cap 500 both trees; validated **360k @ 100+ fps** pan)
### M3.15 — [x] T-060 scale load/save (shipped `b1fd25a` — load partial pass @ ~360k; Save ~142 MB → 201)
### M3.16 — [x] T-061 drag-move @ 360k (good enough — motion ~60 fps; boundaries via `slotIconCache` — spec: [t061_drag_move_hotfix.md](../../../Design_Docs/Mission_Creator_Architecture/t061_drag_move_hotfix.md))
### M3.17 — [x] T-062 incremental bindings @ 360k (classifier + bulk delete ≤10k — spec: [t062_incremental_bindings.md](../../../Design_Docs/Mission_Creator_Architecture/t062_incremental_bindings.md))
### M3.18 — [x] T-062.2 editor session / alt-tab resilience (Vite reload guard + warm session — spec: [t062_2_editor_session_persistence.md](../../../Design_Docs/Mission_Creator_Architecture/t062_2_editor_session_persistence.md))
### M4 — [ ] T-063+ scale program + DEM/registry (see MC ROADMAP §Map performance)

## Test Plan

1. Mission maker opens `/missions/:id/edit` → map + docks render full-bleed.
2. Drop asset from palette → slot appears on map and in active layer.
3. Save Version with new semver → 201; duplicate semver → 409 with error surfaced.
4. Export downloads JSON without POST.
5. Drop or drag a unit → Undo enables → Cmd/Ctrl+Z reverts; redo restores.

## Open Questions / Blockers

- **[PERF-001] ~~Map pan/zoom FPS collapse~~** — **Resolved T-057** (100+ fps @ 10k validated); **T-058** OBJ/SEL entity-count telemetry shipped.
- **[PERF-002] ~~Bulk paste 10k freeze~~** — **Resolved T-059** (validated **360k @ 100+ fps** pan; 6k paste loops smooth).
- **[PERF-003] Initial load** — **Shipped T-060:** restoring label within 1–2 s; ~30 s–1 min @ 360k; 0→300k jump remains (**T-062.1+** IDB streaming). Pan **100+ fps** @ 360k when idle.
- **[PERF-004] Save Version** — **Resolved T-060.1.4 / shipped T-060.** Verified: curl 140 MB → 201; browser Save @ ~367k/~142 MB → 201 (2026-06-23).
- **[PERF-005] Drag-move @ 360k** — **Resolved T-061 (good enough).** Motion ~60 fps sustained; pickup/release materially improved via `slotIconCache` + bindings slot fast path. Mega optimizations deferred ([MC ROADMAP §Deferred mega optimizations](../../../Design_Docs/Mission_Creator_Architecture/ROADMAP.md)). Spec: [t061_drag_move_hotfix.md](../../../Design_Docs/Mission_Creator_Architecture/t061_drag_move_hotfix.md).
- **[PERF-006] Incremental bindings @ 360k** — **Resolved T-062.** Spec: [t062_incremental_bindings.md](../../../Design_Docs/Mission_Creator_Architecture/t062_incremental_bindings.md).
- **[PERF-007] Alt-tab / session reload @ 360k** — **Resolved T-062.2.** Extended alt-tab (Firefox dev) no longer re-triggers full load overlay; warm session skips server GET on same-tab return. Spec: [t062_2_editor_session_persistence.md](../../../Design_Docs/Mission_Creator_Architecture/t062_2_editor_session_persistence.md).
- [FD-003](../TRACKING.md): Phases 2/5/6/8 — see [Mission Creator hub](../../../Design_Docs/Mission_Creator_Architecture/README.md).
