# Eden Gap Analysis — TBD vs Arma 3 Eden (Phase 2)

**Document:** `08_eden_gap_analysis.md`  
**Inputs:** [06](./06_tbd_mission_creator_feature_inventory.md) + [07](./07_arma3_eden_editor_feature_reference.md) + [07a](./07a_eden_ui_anatomy.md) + [07b](./07b_eden_attribute_catalog.md)  
**Schema:** [reference/feds_schema.md](../reference/feds_schema.md)

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

| eden_id | tbd_id | parity | priority | gap_notes |
|---------|--------|--------|----------|-----------|
| RIGHT-MODE-001 | RIGHT-CAT-001 | partial | P0 | Factions mock only; not F1 Object taxonomy |
| RIGHT-MODE-002 | — | missing | P2 | Compositions / custom save |
| RIGHT-MODE-003 | RIGHT-STUB-003 | missing | P2 | Triggers |
| RIGHT-MODE-004 | — | missing | P2 | Waypoints |
| RIGHT-MODE-005 | — | missing | P2 | Systems/modules |
| RIGHT-MODE-006 | RIGHT-STUB-002 | missing | P0 | Markers |
| RIGHT-SUBMODE-001 | — | missing | P1 | BLUFOR/OPFOR submode |
| RIGHT-SEARCH-001 | — | missing | P1 | Asset search field |
| RIGHT-SEARCH-002 | — | missing | P2 | class: prefix |
| RIGHT-CREW-001 | — | missing | P1 | Vehicle crew toggle |
| PLACE-001 | PLACE-DROP-001 | partial | — | TBD drag-only; no click-then-click |
| PLACE-002 | PLACE-DROP-001 | match | — | Drag place works (mock) |
| PLACE-003 | — | missing | P2 | Dbl-click empty picker |
| PLACE-004 | — | missing | P1 | Ctrl multi-place |
| PLACE-005 | — | missing | P0 | Area draw triggers/markers |
| PLACE-COMMENT-001 | — | missing | P3 | Editor annotations |
| PLACE-CREW-001 | — | missing | P1 | Alt empty vehicle |

---

## Transform, widget, toolbar

| eden_id | tbd_id | parity | priority | gap_notes |
|---------|--------|--------|----------|-----------|
| XFORM-MOVE-001 | XFORM-MOVE-001 | match | — | 2D drag |
| XFORM-ALT-001 | — | na | deferred | DEM Phase 2 |
| XFORM-SHIFT-001 | XFORM-ROT-001 | missing | P1 | No Shift rotate |
| XFORM-VERT-001 | — | na | deferred | 3D vertical mode |
| XFORM-SNAP-001 | XFORM-SNAP-001 | na | deferred | DEM |
| WIDGET-CYCLE-001 | — | missing | P1 | Space cycles widget in Eden; TBD Space = flyTo |
| WIDGET-TRANS-001 | — | missing | P2 | Axis widget |
| TOOLBAR-INTEL-001 | TOP-SETTINGS-001 | partial | P1 | Scenario attrs partial |
| TOOLBAR-MAP-001 | MAP-VIEW-001 | partial | — | TBD 2D-only always |
| TOOLBAR-GRID-MOVE-001 | — | missing | P2 | Snap grid |
| TOOLBAR-UNDO-001 | TOP-UNDO-001 | partial | P1 | No Ctrl+Z |

---

## Compositions

| eden_id | tbd_id | parity | priority | gap_notes |
|---------|--------|--------|----------|-----------|
| COMP-SAVE-001 | — | missing | P2 | Custom compositions |
| COMP-PLACE-001 | — | missing | P2 | Place composition |
| COMP-WORKSHOP-001 | — | deferred | P3 | Steam Workshop |

---

## Connections & groups

| eden_id | tbd_id | parity | priority | gap_notes |
|---------|--------|--------|----------|-----------|
| CONN-GROUP-001 | — | missing | P1 | ORBAT squad authoring related |
| CONN-SYNC-001 | XFORM-SYNC-001 | missing | P2 | Entity sync |
| CONN-TRG-OWNER-001 | — | missing | P2 | Triggers |
| CONN-WP-ACT-001 | — | missing | P2 | Waypoints |
| CONN-WP-ATTACH-001 | — | missing | P2 | WP attach/detach |
| CREW-PANEL-001 | — | missing | P1 | Hover crew list |
| CREW-BOARD-001 | — | missing | P1 | Drag into vehicle |
| CREW-SEAT-001 | — | missing | P2 | Change seat RMB |
| SEL-GROUP-ICON-001 | LEFT-ORBAT-001 | partial | P1 | ORBAT read-only |

---

## Layers & selection

| eden_id | tbd_id | parity | priority | gap_notes |
|---------|--------|--------|----------|-----------|
| LAYER-CREATE-001 | LEFT-LAYER-005 | tbd_only | — | Editor Layers ≠ Eden layers |
| LAYER-DEL-001 | LEFT-LAYER-007 | partial | — | TBD deletes slots in folder |
| SEL-MOD-001 | SEL-MOD-001 | missing | P1 | Ctrl+LMB |
| SEL-ALL-001 | KEY-SELALL-001 | missing | P2 | Select all |
| ATTR-FIELD-OBJ-SKILL | ATTR-TAB-003 | missing | P2 | States stub |
| ATTR-FIELD-OBJ-FUEL | — | missing | P2 | Vehicles |
| ATTR-FIELD-OBJ-POSITION | ATTR-TAB-001 | partial | P0 | Read-only position |
| ATTR-MULTI-CHK-001 | — | missing | P2 | Multi-edit per-field checkbox |
| SEL-ORBAT-DBL-001 | SEL-ORBAT-DBL-001 | missing | P1 | ORBAT dbl-click attrs |
| MAP-TERRAIN-001 | MAP-TERRAIN-001 | partial | P1 | meta.terrain not wired to map |
| ENV-SETTINGS-002 | TOP-SETTINGS-001 | partial | — | Thermals + view dist in dialog |
| DATA-HYD-TITLE-001 | TOP-TITLE-001 | partial | P0 | Title not from API row |

---

## Shell & data (unchanged core)

| eden_id | tbd_id | parity | priority | gap_notes |
|---------|--------|--------|----------|-----------|
| SEL-MAP-003 | SEL-MAP-003 | match | — | Marquee |
| XFORM-DEL-001 | XFORM-DEL-001 | match | — | Delete |
| TOP-SAVE-001 | TOP-SAVE-001 | partial | — | Semver vs Eden save |
| TOP-EXPORT-001 | TOP-EXPORT-001 | match | — | |
| — | TBD-LAYER-001 | tbd_only | — | Workflow folders |
| — | TBD-CONFLICT-001 | tbd_only | — | IndexedDB conflict |

---

## Numbered backlog (Phase 2)

### P0 — Ship-blocking

1. **P0-01** Real asset registry + RIGHT-MODE-001 parity (`RIGHT-CAT-001`)
2. **P0-02** Markers on map (`RIGHT-MODE-006`, `PLACE-005`)
3. **P0-03** Vehicles placeable (`RIGHT-MODE-001` vehicles)
4. **P0-04** Numeric position edit (`ATTR-FIELD-OBJ-POSITION`)
5. **P0-05** ORBAT authoring (`CONN-GROUP-001`, `LEFT-ORBAT-001`)
6. **P0-06** Title hydrate from mission row (`TOP-TITLE-001`, `DATA-HYD-TITLE-001`)
7. **P0-07** Wire `meta.terrain` to map viewport (`MAP-TERRAIN-001`)

### P1 — Eden feel

7. **P1-01** Ctrl+LMB additive select (`SEL-MOD-001`)
8. **P1-02** Ctrl+C/V copy paste (`ACTION-CopyUnit` / `PasteUnit`)
9. **P1-03** Ctrl+Z/Y (`TOP-UNDO-001`)
10. **P1-04** Asset browser search (`RIGHT-SEARCH-001`)
11. **P1-05** Ctrl multi-place (`PLACE-004`)
12. **P1-06** Map rotation / Shift rotate (`XFORM-SHIFT-001`)
13. **P1-07** Faction submode or catalog structure (`RIGHT-SUBMODE-001`)
14. **P1-08** Resolve Space conflict: flyTo vs widget cycle (`WIDGET-CYCLE-001` vs `MAP-FLY-001`)
15. **P1-09** ORBAT dbl-click opens attributes (`SEL-ORBAT-DBL-001`)
16. **P1-10** Vehicle crew UI (`CREW-PANEL-001`, `CREW-BOARD-001`)
17. **P1-11** Empty vehicle place Alt (`PLACE-CREW-001`)

### P2 — Power user

15. **P2-01** Custom compositions (`COMP-*`)
16. **P2-02** Triggers/waypoints/systems modes
17. **P2-03** Connection/sync UI (`CONN-*`)
18. **P2-04** Transformation widget + grids (`WIDGET-*`, `TOOLBAR-GRID-*`)
19. **P2-05** Full attribute fields per `07b`
20. **P2-06** Menu bar implementation (`TOP-MENU-001`)
21. **P2-07** class:/mod: search (`RIGHT-SEARCH-002/003`)

### P3 — Deferred

22. **P3-01** Workshop compositions (`COMP-WORKSHOP-001`)
23. **P3-02** DEM / 3D snap (`XFORM-SNAP-001`, `XFORM-ALT-001`)
24. **P3-03** Arsenal (`ATTR-FIELD-*` loadout)
25. **P3-04** y-websocket collab

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
