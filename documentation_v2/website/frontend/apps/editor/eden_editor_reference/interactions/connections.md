**Status:** live

# Eden connection interactions

How the Arma 3 Eden editor connects entities: the right-click Connect flow, grouping, syncing,
trigger owners, random start positions, waypoint activation and attachment, and deleting a
connection. Each entry follows the Eden reference format of the
[feature entry schema](/documentation_v2/website/frontend/apps/editor/feature_inventory/feds_schema.md#eden-reference-entries);
the connection types are summarised in the [UI anatomy](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/ui_anatomy.md#context-menu-contextmenu).

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
