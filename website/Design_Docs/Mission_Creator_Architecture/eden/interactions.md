# Arma 3 Eden Editor — Feature Reference (Interactions)

**Document:** `eden/interactions.md`  
**Schema:** [reference/feds_schema.md](../reference/feds_schema.md) (FEDS v2)  
**UI layout:** [ui_anatomy.md](./ui_anatomy.md)  
**Attribute fields:** [attributes.md](./attributes.md)  
**Scrape cache:** `artifacts/eden-wiki/` (28 pages, 2026-06-20)  
**Gap analysis:** [gap_analysis.md](./gap_analysis.md)

Scope: **Arma 3 Eden Editor** (3D). TBD is 2D flat — 3D-only items marked `N/A (3D)`.

---

## Glossary

See [ui_anatomy.md](../eden/ui_anatomy.md) and [Terminology](https://community.bistudio.com/wiki/Eden_Editor:_Terminology).

---

## RIGHT — Asset Browser

#### RIGHT-MODE-001 — Object mode (F1)

| Field | Value |
|-------|-------|
| **Domain** | RIGHT |
| **UI Surface** | AssetBrowser |
| **Feature kind** | browser_mode |
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Asset_Browser#Mode |
| **Shortcut** | F1 |
| **Goal** | Browse/place objects (units, vehicles, props) |
| **Trigger** | Click Object mode tab or press F1 |
| **Preconditions** | Asset Browser visible |
| **Procedure** | 1. Mode filter = Object. 2. Submode shows faction/side filters. 3. List shows categorized assets. |
| **Postconditions** | Object assets available for placement |
| **Inputs** | F1, tab click |
| **Outputs** | Browser list content |
| **Edge cases** | Addon assets appear when mods loaded |
| **Acceptance** | `- [ ] F1 shows unit/vehicle/prop tree` |

#### RIGHT-MODE-002 — Composition mode (F2)

| Field | Value |
|-------|-------|
| **Domain** | RIGHT |
| **UI Surface** | AssetBrowser |
| **Feature kind** | browser_mode |
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Asset_Browser#Mode |
| **Shortcut** | F2 |
| **Goal** | Place predefined/custom group compositions |
| **Trigger** | F2 or Composition tab |
| **Procedure** | Mode = Composition; tree includes Compositions > Custom; composition toolbar active. |
| **Acceptance** | `- [ ] Custom compositions under Compositions > Custom` |

#### RIGHT-MODE-003 — Trigger mode (F3)

| Field | Value |
|-------|-------|
| **Domain** | RIGHT |
| **UI Surface** | AssetBrowser |
| **Feature kind** | browser_mode |
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Asset_Browser#Mode |
| **Shortcut** | F3 |
| **Goal** | Place trigger entities |
| **Procedure** | Mode = Trigger; no submode. |
| **Acceptance** | `- [ ] Trigger list; no submode` |

#### RIGHT-MODE-004 — Waypoint mode (F4)

| Field | Value |
|-------|-------|
| **Domain** | RIGHT |
| **UI Surface** | AssetBrowser |
| **Feature kind** | browser_mode |
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Asset_Browser#Mode |
| **Shortcut** | F4 |
| **Acceptance** | `- [ ] Waypoint assets listed` |

#### RIGHT-MODE-005 — System mode (F5)

| Field | Value |
|-------|-------|
| **Domain** | RIGHT |
| **UI Surface** | AssetBrowser |
| **Feature kind** | browser_mode |
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Asset_Browser#Mode |
| **Shortcut** | F5 |
| **Acceptance** | `- [ ] Systems modules listed` |

#### RIGHT-MODE-006 — Marker mode (F6)

| Field | Value |
|-------|-------|
| **Domain** | RIGHT |
| **UI Surface** | AssetBrowser |
| **Feature kind** | browser_mode |
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Asset_Browser#Mode |
| **Shortcut** | F6 |
| **Acceptance** | `- [ ] Marker icons in list` |

#### RIGHT-SUBMODE-001 — Asset submode (faction filter)

| Field | Value |
|-------|-------|
| **Domain** | RIGHT |
| **UI Surface** | AssetBrowser |
| **Feature kind** | browser_mode |
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Asset_Browser#Submode |
| **Shortcut** | Edit → Toggle Asset Sub-type; `SubmodeToggle` |
| **Goal** | Filter objects by side (BLUFOR, OPFOR, …) |
| **Procedure** | Unavailable for Trigger/Waypoint modes. |
| **Acceptance** | `- [ ] Faction tabs on Object mode` |

#### RIGHT-SEARCH-001 — Search by asset name

| Field | Value |
|-------|-------|
| **Domain** | RIGHT |
| **UI Surface** | AssetBrowser |
| **Feature kind** | interaction |
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Entity_Placing#Asset_Search |
| **Trigger** | Type in search field |
| **Procedure** | Persists across mode/submode switches. |
| **Acceptance** | `- [ ] Search survives mode change` |

#### RIGHT-SEARCH-002 — class: prefix search

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Asset_Browser#Search |
| **Trigger** | `class B_Soldier` |
| **Acceptance** | `- [ ] class prefix filters` |

#### RIGHT-SEARCH-003 — mod: prefix search

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Asset_Browser#Search |
| **Trigger** | `mod kart` or dropdown |
| **Acceptance** | `- [ ] mod prefix filters` |

#### RIGHT-SEARCH-004 — Wildcard search

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Asset_Browser#Search |
| **Trigger** | `house*ruin` (2.22+) |
| **Acceptance** | `- [ ] Wildcards work` |

#### RIGHT-SEARCH-005 — Regex search

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Asset_Browser#Search |
| **Trigger** | `/` prefix e.g. `/(Brick` |
| **Acceptance** | `- [ ] Regex filter works` |

#### RIGHT-CREW-001 — Vehicle crew toggle

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Entity_Placing#Placing_vehicles_with_crew |
| **Shortcut** | Alt inverts while placing |
| **Trigger** | Switch below browser |
| **Acceptance** | `- [ ] Manned/unmanned toggle` |

---

## PLACE — Entity placement

#### PLACE-001 — Click-then-click place

| Field | Value |
|-------|-------|
| **UI Surface** | View |
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Entity_Placing#Placing_Entities |
| **Trigger** | LMB asset in browser → LMB view |
| **Procedure** | Entity created + selected. |
| **Acceptance** | `- [ ] Click place works` |

#### PLACE-002 — Drag browser to view

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Entity_Placing#Placing_Entities |
| **Trigger** | Drag leaf to view |
| **Acceptance** | `- [ ] Drag place works` |

#### PLACE-003 — Dbl-click empty type picker

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Entity_Placing#Placing_Entities |
| **Trigger** | Dbl-click empty scene |
| **Acceptance** | `- [ ] Type picker opens` |

#### PLACE-004 — Ctrl multi-place

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Entity_Placing#Placing_multiple_entities |
| **Shortcut** | Ctrl |
| **Trigger** | Hold Ctrl while placing repeatedly |
| **Acceptance** | `- [ ] Repeated place without re-select` |

#### PLACE-005 — Area draw (triggers/markers)

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Entity_Placing#Placing_area_entities |
| **Trigger** | Select area asset → LMB hold drag on map |
| **Acceptance** | `- [ ] Area drawn on map` |

#### PLACE-COMMENT-001 — Place comment (RMB empty)

| Field | Value |
|-------|-------|
| **Domain** | PLACE |
| **UI Surface** | View |
| **Feature kind** | annotation |
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Comment#Placing_Comments |
| **Trigger** | RMB empty space → **Place Comment** |
| **Procedure** | 1. Create virtual comment entity (editor-only). 2. Open attributes for Title + Tooltip. 3. Draggable, copy/paste, layerable, composable. |
| **Postconditions** | Comment icon at position; no gameplay effect |
| **Edge cases** | Saved in custom compositions as documentation |
| **Acceptance** | `- [ ] RMB → Place Comment` `- [ ] Title/tooltip editable` |

#### PLACE-CREW-001 — Empty vehicle (Alt while placing)

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Switching_from_2D_Editor#Empty_Vehicles |
| **Shortcut** | Alt (invert crew toggle) |
| **Trigger** | Place vehicle with crew toggle off / hold Alt |
| **Acceptance** | `- [ ] Vehicle spawns without default crew` |

---

## XFORM — Basic transform

#### XFORM-MOVE-001 — LMB drag move

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Entity_Transforming#Position |
| **Shortcut** | LMB drag |
| **Acceptance** | `- [ ] Drag moves entity` |

#### XFORM-ALT-001 — Alt drag altitude

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Entity_Transforming#Altitude |
| **Shortcut** | Alt + drag |
| **Eden parity** | N/A (3D) for TBD |

#### XFORM-SHIFT-001 — Shift drag rotate

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Entity_Transforming#Direction |
| **Shortcut** | Shift + drag |
| **Acceptance** | `- [ ] Rotates to cursor` |

#### XFORM-VERT-001 — Vertical mode

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Entity_Transforming#Vertical_Mode |
| **UI Surface** | Toolbar |
| **Acceptance** | `- [ ] Sea level / underground` |

#### XFORM-SNAP-001 — Surface snap

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Entity_Transforming#Surface_Snapping |
| **UI Surface** | Toolbar |
| **Acceptance** | `- [ ] Snaps to terraces` |

---

## CREW — Vehicle crew

#### CREW-PANEL-001 — Hover crew panel

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Transforming_Crew#Crew_in_the_editor_interface |
| **Trigger** | Hover vehicle icon |
| **Procedure** | Panel lists all crew roles (Driver/Pilot, Commander, Turret, Passenger); visible crew get extra scene icon |
| **Acceptance** | `- [ ] Crew list beside vehicle icon` |

#### CREW-BOARD-001 — Drag character into vehicle

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Transforming_Crew#Moving_crew_into_a_vehicle |
| **Trigger** | Drag character onto vehicle icon/model |
| **Edge cases** | Full vehicle → character moves to vehicle position only; enemy vehicle warns |
| **Acceptance** | `- [ ] Character becomes crew` |

#### CREW-UNBOARD-001 — Drag crew out

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Transforming_Crew#Removing_crew_from_a_vehicle |
| **Trigger** | Drag crew icon away from vehicle |
| **Acceptance** | `- [ ] Crew detached` |

#### CREW-SEAT-001 — Change seat (RMB)

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Transforming_Crew#Changing_Seats |
| **Trigger** | RMB crew → Change Seat → pick role |
| **Edge cases** | Occupied seat → swap |
| **Acceptance** | `- [ ] Seat reassigned` |

---

## WIDGET — Transformation widget

#### WIDGET-CYCLE-001 — Cycle widget (Space)

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Transformation_Widget#Variants |
| **Shortcut** | Space |
| **Acceptance** | `- [ ] Cycles translation/rotation/area widgets` |

#### WIDGET-TRANS-001 — Translation widget

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Transformation_Widget#Translation_Widget |
| **Acceptance** | `- [ ] Axis arrows move entity` |

#### WIDGET-ROT-001 — Rotation widget

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Transformation_Widget#Rotation_Widget |
| **Acceptance** | `- [ ] Rotates on axis` |

#### WIDGET-AREA-SCALE-001 — Area scaling widget

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Transformation_Widget#Area_Scaling_Widget |
| **Acceptance** | `- [ ] Resizes trigger area` |

#### WIDGET-AREA-001 — Area widget

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Transformation_Widget#Area_Widget |
| **Acceptance** | `- [ ] Orients area marker` |

#### WIDGET-COORD-001 — Global vs local axes

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Transformation_Widget#Changing_the_Reference_System |
| **Acceptance** | `- [ ] Local rotates with entity` |

---

## TOOLBAR — Index

See [ui_anatomy § Toolbar](../eden/ui_anatomy.md). IDs: `TOOLBAR-NEW-001` … `TOOLBAR-TUTORIAL-001` (New, Open, Save, Workshop, Undo, Redo, widgets, snap, grids, intel, map, flashlight, vision, phase, tutorials).

Wiki: https://community.bistudio.com/wiki/Eden_Editor:_Toolbar

---

## COMP — Custom compositions

#### COMP-SAVE-001 — Save custom composition

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Custom_Composition#Saving_Compositions |
| **Shortcut** | RMB → Save; browser button; `CreateCustomComposition` |
| **Procedure** | Saves attrs, layers, visibility, connections. |
| **Acceptance** | `- [ ] Appears in Compositions > Custom` |

#### COMP-EDIT-001 — Edit metadata

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Custom_Composition#Saving_Compositions |
| **Trigger** | Dbl-click / Edit button |
| **Acceptance** | `- [ ] Title/author/category editable` |

#### COMP-PLACE-001 — Place composition

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Custom_Composition#Placing_Compositions |
| **Edge cases** | Terrain vs Sea vertical mode |
| **Acceptance** | `- [ ] All entities spawn` |

#### COMP-WORKSHOP-001 — Publish Workshop

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Custom_Composition#Publishing_Compositions |
| **Acceptance** | `- [ ] Publish dialog completes` |

#### COMP-SUBSCRIBE-001 — Subscribe Workshop

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Custom_Composition#Subscribing_to_Compositions |
| **Acceptance** | `- [ ] Subscribed comps in browser` |

---

## CONN — Connections

#### CONN-START-001 — RMB Connect flow

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Connecting#Connecting_Entities |
| **Trigger** | RMB → Connect → type → LMB target |
| **Acceptance** | `- [ ] Line drawn` |

#### CONN-GROUP-001 — Grouping

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Connecting#Grouping |
| **Shortcut** | Ctrl + drag char→char |
| **Acceptance** | `- [ ] Squad formed` |

#### CONN-SYNC-001 — Syncing

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Connecting#Syncing |
| **Acceptance** | `- [ ] Char-object sync` |

#### CONN-TRG-OWNER-001 — Trigger owner

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Connecting#Setting_Trigger_Owner |

#### CONN-RAND-START-001 — Random start

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Connecting#Setting_Random_Start |

#### CONN-WP-ACT-001 — Waypoint activation

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Connecting#Setting_Waypoint_Activation |

#### CONN-WP-ATTACH-001 — Attach waypoint to object

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Waypoint#Attaching_Waypoints |
| **Trigger** | Drag waypoint onto object; drag away to detach |
| **Edge cases** | DESTROY / GET IN require attachment; moving object moves attached WPs |
| **Acceptance** | `- [ ] Attached icon outline` `- [ ] Detach by drag away` |

#### CONN-DEL-001 — Delete connection

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Connecting#Disconnecting_Entities |
| **Shortcut** | Del on line |

---

## SEL / LAYER / ATTR

| ID | Summary | Wiki |
|----|---------|------|
| SEL-001 | Click select | Actions#SelectUnit |
| SEL-MOD-001 | Ctrl+LMB add | Actions#AddUnitToSel |
| SEL-ALL-001 | Select all on screen | Menu Bar#Edit |
| SEL-GROUP-ICON-001 | Click group icon | Group |
| SEL-LAYER-CHILDREN-001 | Select layer children | Actions |
| SEL-LAYER-DESC-001 | Select all descendants | Actions |
| LAYER-CREATE-001 | New layer button | Layer#Creating_a_layer |
| LAYER-DEL-001 | Del deletes subtree | Layer#Deleting_a_layer |
| ATTR-OPEN-001 | Dbl-click attributes | Setting Attributes |
| ATTR-MULTI-001 | Multi-select attributes | Switching from 2D Editor |
| ATTR-MULTI-CHK-001 | Per-field enable checkbox when values differ | Switching from 2D Editor#Editing_Multiple_Entities |

#### ATTR-MULTI-001 — Multi-edit attributes (detail)

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Switching_from_2D_Editor#Editing_Multiple_Entities |
| **Trigger** | Multi-select → RMB → Attributes (dbl-click opens single only) |
| **Procedure** | Shared values shown; differing fields **disabled** until right-side checkbox enabled |
| **Acceptance** | `- [ ] Bulk edit with per-field opt-in` |

#### CTX-FORMATION-001 — Formation in context menu

| Field | Value |
|-------|-------|
| **Wiki anchor** | https://community.bistudio.com/wiki/Eden_Editor:_Switching_from_2D_Editor |
| **Trigger** | RMB group/selection → Formation |
| **Acceptance** | `- [ ] Formation submenu` |

---

## KEY shortcuts

| ID | Keys | Effect |
|----|------|--------|
| KEY-WP-001 | Shift+RMB | Quick waypoint |
| KEY-WIDGET-001 | Space | Cycle widget |
| KEY-GRID-001 | `;` | Translation grid toggle |
| KEY-HIDE-UI-001 | Backspace | Hide UI (screenshot) |

---

## ACTION appendix

90+ `do3DENAction` names in `artifacts/eden-wiki/Eden_Editor__Actions.md`. Key clipboard/transform actions:

| ID | Action | Effect |
|----|--------|--------|
| ACTION-COPY-001 | `CopyUnit` | Copy selection to clipboard |
| ACTION-CUT-001 | `CutUnit` | Cut selection |
| ACTION-PASTE-001 | `PasteUnit` | Paste at cursor |
| ACTION-PASTE-ORIG-001 | `PasteUnitOrig` | Paste at original position |
| ACTION-LEVEL-001 | `LevelWithSurface` | Align to terrain |
| ACTION-SNAP-001 | `SnapToSurface` | Snap to ground |
| ACTION-SEAT-001 | `ChangeSeat` | Crew seat change |
| ACTION-FORM-001 | `ForceToFormation` | Snap group to formation positions |
| ACTION-TOGGLE-SEL-001 | `ToggleUnitSel` | Toggle unit in selection |
| ACTION-WP-QUICK-001 | Shift+RMB | Quick MOVE waypoint |

Also: `MissionSave`, `SelectObjectMode`, `SyncWith`, `GroupWith`, `OpenAttributes`, `SearchEdit`, …

**Note:** `Eden_Editor__Entity_Context_Menu.md` scrape is primarily **modding/config** (`class Cfg3DENContextMenu`) — end-user menu items are better sourced from Switching from 2D Editor + Connecting + per-type pages above.

## STATUS bar

| ID | Entry |
|----|-------|
| STATUS-X-001 | Cursor X |
| STATUS-Y-001 | Cursor Y |
| STATUS-Z-001 | Elevation |
| STATUS-ZOOM-001 | Zoom/resolution |
| STATUS-VER-001 | Game version |
| STATUS-MOD-001 | Mods loaded |
| STATUS-SRV-001 | MP server |

Wiki: https://community.bistudio.com/wiki/Eden_Editor:_Status_Bar
