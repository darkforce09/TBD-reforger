**Status:** live

# Eden selection, layer and attribute interactions

How the Arma 3 Eden editor selects entities, creates and deletes layers, opens the Attributes
dialog for one or several entities, and sets a formation from the context menu. The index table
names each entry with its Eden wiki page, written as the page title after
`https://community.bistudio.com/wiki/Eden_Editor:_` with spaces for underscores and an optional
`#` section; the entries with a field table follow the Eden reference
format of the [feature entry schema](/documentation_v2/website/frontend/apps/editor/feature_inventory/feds_schema.md#eden-reference-entries).
The attribute fields themselves are in the [attribute catalog](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/attributes.md).

## SEL / LAYER / ATTR — Selection, layers and attributes

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
