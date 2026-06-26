# Eden Gap Analysis — TBD vs Arma 3 Eden (Phase 2)

**Document:** `eden/gap_analysis.md`  
**Inputs:** [feature_inventory.md](../feature_inventory.md) + [interactions](./interactions.md) + [ui_anatomy](./ui_anatomy.md) + [attributes](./attributes.md)  
**Schema:** [reference/feds_schema.md](../reference/feds_schema.md)

**Execution order (locked):** … → **T-061..T-067** → Eden **T-068+** → **T-110** terrain base ([`t110_terrain_base_mission_layers.md`](../t110_terrain_base_mission_layers.md)) → **T-090** / **T-091** map tiles + DEM (blocked on hosted assets).

---

## Parity legend

| Status | Meaning |
|--------|---------|
| **match** | Equivalent (2D acceptable) |
| **partial** | Exists but incomplete |
| **missing** | Not built |
| **deferred** | Intentional later phase |
| **na** | 3D-only / out of scope |
| **tbd_only** | TBD addition |

---

## Asset Browser & placement

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| RIGHT-MODE-001 | RIGHT-CAT-001 | match | T-068.3 shipped | Live registry palette — `useRegistry` + `buildCatalogTree`; 8 NATO characters @ current seed |
| RIGHT-MODE-002 | — | missing | — | Compositions / custom save |
| RIGHT-MODE-003 | RIGHT-STUB-003 | missing | — | Triggers |
| RIGHT-MODE-004 | — | missing | — | Waypoints |
| RIGHT-MODE-005 | — | missing | — | Systems/modules |
| RIGHT-MODE-006 | RIGHT-STUB-002 | missing | T-069 | Markers |
| RIGHT-SUBMODE-001 | — | missing | T-074 | BLUFOR/OPFOR submode |
| RIGHT-SEARCH-001 | — | working | T-055 | Asset search field (T-055; filters Factions tree) |
| RIGHT-SEARCH-002 | — | missing | — | class: prefix |
| RIGHT-CREW-001 | — | missing | — | Vehicle crew toggle |
| PLACE-001 | PLACE-DROP-001 | partial | — | TBD drag-only; no click-then-click |
| PLACE-002 | PLACE-DROP-001 | match | — | Drag place works (mock) |
| PLACE-003 | — | missing | — | Dbl-click empty picker |
| PLACE-004 | — | missing | T-072 | Ctrl multi-place |
| PLACE-005 | — | missing | — | Area draw triggers/markers |
| PLACE-COMMENT-001 | — | missing | — | Editor annotations |
| PLACE-CREW-001 | — | missing | T-077 | Alt empty vehicle |

---

## Transform, widget, toolbar

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| XFORM-MOVE-001 | XFORM-MOVE-001 | match | — | 2D drag |
| XFORM-ALT-001 | — | na | — | DEM Phase 2 |
| XFORM-SHIFT-001 | XFORM-ROT-001 | missing | T-073 | No Shift rotate |
| XFORM-VERT-001 | — | na | — | 3D vertical mode |
| XFORM-SNAP-001 | XFORM-SNAP-001 | na | — | DEM |
| WIDGET-CYCLE-001 | — | missing | T-075 | Space cycles widget in Eden; TBD Space = flyTo |
| WIDGET-TRANS-001 | — | missing | — | Axis widget |
| TOOLBAR-INTEL-001 | TOP-SETTINGS-001 | partial | — | Scenario attrs partial |
| TOOLBAR-MAP-001 | MAP-VIEW-001 | partial | — | TBD 2D-only always |
| TOOLBAR-GRID-MOVE-001 | — | missing | — | Snap grid |
| TOOLBAR-UNDO-001 | TOP-UNDO-001 | match | T-052 | ✅ T-052 — Cmd/Ctrl+Z/Shift+Z/Ctrl+Y keyboard + toolbar buttons |

---

## Compositions

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| COMP-SAVE-001 | — | missing | T-078 | Custom compositions |
| COMP-PLACE-001 | — | missing | T-078 | Place composition |
| COMP-WORKSHOP-001 | — | deferred | — | Steam Workshop |

---

## Connections & groups

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| CONN-GROUP-001 | — | missing | T-071 | ORBAT squad authoring in ORBAT Manager modal |
| CONN-SYNC-001 | XFORM-SYNC-001 | missing | — | Entity sync |
| CONN-TRG-OWNER-001 | — | missing | — | Triggers |
| CONN-WP-ACT-001 | — | missing | — | Waypoints |
| CONN-WP-ATTACH-001 | — | missing | — | WP attach/detach |
| CREW-PANEL-001 | — | missing | T-076 | Hover crew list |
| CREW-BOARD-001 | — | missing | T-076 | Drag into vehicle |
| CREW-SEAT-001 | — | missing | — | Change seat RMB |
| SEL-GROUP-ICON-001 | LEFT-ORBAT-001 | partial | T-071 | Left ORBAT read-only until T-071 removes duplicate; full authoring in modal |

---

## Layers & selection

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| LAYER-CREATE-001 | LEFT-LAYER-005 | tbd_only | — | Editor Layers ≠ Eden layers |
| LAYER-DEL-001 | LEFT-LAYER-007 | partial | — | TBD deletes slots in folder |
| SEL-MOD-001 | SEL-MOD-001 | match | T-053 | ✅ T-053 — Ctrl/Cmd+LMB toggle additive select (Shift unbound) |
| SEL-ALL-001 | KEY-SELALL-001 | missing | — | Select all |
| ATTR-FIELD-OBJ-SKILL | ATTR-TAB-003 | missing | — | States stub |
| ATTR-FIELD-OBJ-FUEL | — | missing | — | Vehicles |
| ATTR-FIELD-OBJ-POSITION | ATTR-TAB-001 | match | T-049 | ✅ T-049 — editable X/Y/Z/rotation (NumberField → updateSlotPosition) |
| ATTR-MULTI-CHK-001 | — | missing | — | Multi-edit per-field checkbox |
| SEL-ORBAT-DBL-001 | SEL-ORBAT-DBL-001 | match | T-054 | ✅ T-054 — ORBAT slot row dbl-click → Attributes (map uses native dblclick + pickObject) |
| MAP-TERRAIN-001 | MAP-TERRAIN-001 | match | T-049 | ✅ T-049 — meta.terrain → viewport (key-remount, Everon/Arland bounds) |
| ENV-SETTINGS-002 | TOP-SETTINGS-001 | partial | — | Thermals + view dist in dialog |
| DATA-HYD-TITLE-001 | TOP-TITLE-001 | match | T-049 | ✅ T-049 — applyMissionRowMeta hydrates title/terrain/env on load (no PATCH-back) |

---

## Shell & data (unchanged core)

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| SEL-MAP-003 | SEL-MAP-003 | match | — | Marquee |
| XFORM-DEL-001 | XFORM-DEL-001 | match | — | Delete |
| TOP-SAVE-001 | TOP-SAVE-001 | partial | — | Semver vs Eden save |
| TOP-EXPORT-001 | TOP-EXPORT-001 | match | — |  |
| — | TBD-LAYER-001 | tbd_only | — | Workflow folders |
| — | TBD-CONFLICT-001 | tbd_only | — | IndexedDB conflict |

---

## Summary

| Category | Match | Partial | Missing | N/A | TBD-only |
|----------|-------|---------|---------|-----|----------|
| Core map edit | 5 | 2 | 0 | 0 | 0 |
| Asset browser | 0 | 2 | 8 | 0 | 0 |
| Transform/widget | 1 | 1 | 4 | 3 | 0 |
| Compositions | 0 | 0 | 2 | 0 | 0 |
| Attributes | 0 | 2 | 10+ | 0 | 0 |
| TBD extras | — | — | — | — | 3 |

**Phase 2 doc coverage:** 85+ Eden interaction IDs in `07`; 60+ attribute fields in `07b`; full UI anatomy in `07a`; wiki scrape 28/28 pages. **Second pass (2026-06-20):** Comments, crew, clipboard actions, WP attach, multi-edit attrs, TRG/WP field tables; TBD code audit supplement in `06`.
