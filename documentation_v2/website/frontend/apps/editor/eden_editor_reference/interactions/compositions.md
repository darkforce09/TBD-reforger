**Status:** live

# Eden custom composition interactions

How the Arma 3 Eden editor saves a selection as a custom composition, edits its metadata, places
it, and publishes or subscribes to compositions on the Steam Workshop. Each entry follows the Eden
reference format of the [feature entry schema](/documentation_v2/website/frontend/apps/editor/feature_inventory/feds_schema.md#eden-reference-entries);
the composition metadata fields are in the [attribute catalog](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/attributes.md#composition-metadata-not-entity-attributes).

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
