**Status:** live

# Eden entity placement interactions

How the Arma 3 Eden editor places entities from the asset browser into the view: click, drag,
repeated and area placement, comments, and vehicles without crew. Each entry follows the Eden
reference format of the [feature entry schema](/documentation_v2/website/frontend/apps/editor/feature_inventory/feds_schema.md#eden-reference-entries).

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
