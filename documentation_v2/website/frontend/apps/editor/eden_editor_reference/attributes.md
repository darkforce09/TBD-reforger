**Status:** live

# Eden editor attribute fields

The attribute fields of the Arma 3 Eden editor, the reference design the
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) is measured against: every field
of every entity type as an ID with its label, dialog section, SQM property and value type, read
from the Bohemia wiki. The last section says which of them the Mission Creator's Attributes dialog
holds.

Sources: [Setting Attributes](https://community.bistudio.com/wiki/Eden_Editor:_Setting_Attributes)
and the per-type wiki pages each section cites, scraped into `.ai/artifacts/eden-wiki/`. The IDs
follow the `ATTR-FIELD-{TYPE}-{NAME}` pattern of the
[feature entry schema](/documentation_v2/website/frontend/apps/editor/feature_inventory/feds_schema.md#feature-ids).
The [interactions catalog](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/README.md)
covers how the dialog opens, and the [UI anatomy](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/ui_anatomy.md#attributes-dialog-attributesdialog)
its layout.

Attributes are edited via **double-click** → Attributes dialog, or indirectly via drag/widget. Scripting: `set3DENAttribute` / `set3DENAttributes` (editor workspace only).

## Catalog format

| Column | Meaning |
|--------|---------|
| **ID** | `ATTR-FIELD-{TYPE}-{SLUG}` |
| **Name** | UI label in Eden |
| **Category** | Dialog tab section |
| **Property** | SQM/config property (`Development` column) |
| **Type** | Boolean, String, Number, Array, Position3D |
| **Wiki** | Anchor, given once per section as its Source line |

## Object (`OBJ`)

Source: [Setting Attributes#Object](https://community.bistudio.com/wiki/Eden_Editor:_Setting_Attributes#Object)

| ID | Name | Category | Property | Type |
|----|------|----------|----------|------|
| ATTR-FIELD-OBJ-TYPE | Type | Type | ItemClass | String |
| ATTR-FIELD-OBJ-VARNAME | Variable Name | Init | Name | String |
| ATTR-FIELD-OBJ-INIT | Init | Init | Init | String |
| ATTR-FIELD-OBJ-POSITION | Position | Transformation | position | Position3D |
| ATTR-FIELD-OBJ-ROTATION | Rotation | Transformation | rotation | Number |
| ATTR-FIELD-OBJ-SIZE | Size | Transformation | size3 | Array |
| ATTR-FIELD-OBJ-SHAPE | Shape | Transformation | IsRectangle | Boolean |
| ATTR-FIELD-OBJ-PLACEMENT-RADIUS | Placement Radius | Transformation | placementRadius | Number |
| ATTR-FIELD-OBJ-PLAYER-SP | Player | Control | ControlSP | Boolean |
| ATTR-FIELD-OBJ-PLAYABLE-MP | Playable | Control | ControlMP | Boolean |
| ATTR-FIELD-OBJ-ROLE-DESC | Role Description | Control | description | String |
| ATTR-FIELD-OBJ-LOCK | Lock | States | lock | Number |
| ATTR-FIELD-OBJ-SKILL | Skill | States | skill | Number |
| ATTR-FIELD-OBJ-HEALTH | Health / Armor | States | Health | Number |
| ATTR-FIELD-OBJ-FUEL | Fuel | States | fuel | Number |
| ATTR-FIELD-OBJ-AMMO | Ammunition | States | ammo | Number |
| ATTR-FIELD-OBJ-RANK | Rank | States | rank | String |
| ATTR-FIELD-OBJ-STANCE | Stance | States | unitPos | String |
| ATTR-FIELD-OBJ-DYN-SIM | Enable Dynamic Simulation | Special States | dynamicSimulation | Boolean |
| ATTR-FIELD-OBJ-WAKE-DYN-SIM | Wake-Up Dynamic Simulation | Special States | addToDynSimGrid | Number |
| ATTR-FIELD-OBJ-ENABLE-SIM | Enable Simulation | Special States | enableSimulation | Boolean |
| ATTR-FIELD-OBJ-SIMPLE-OBJ | Simple Object | Special States | objectIsSimple | Boolean |
| ATTR-FIELD-OBJ-SHOW-MODEL | Show Model | Special States | hideObject | Boolean |
| ATTR-FIELD-OBJ-ALLOW-DAMAGE | Enable Damage | Special States | allowDamage | Boolean |
| ATTR-FIELD-OBJ-STAMINA | Enable Stamina | Special States | enableStamina | Boolean |
| ATTR-FIELD-OBJ-REVIVE | Revive Enabled | Special States | EnableRevive | Boolean |
| ATTR-FIELD-OBJ-DOORS | Doors States | Special States | DoorStates | — |
| ATTR-FIELD-OBJ-LOCAL-ONLY | Local Only | Special States | isLocalOnly | Boolean |
| ATTR-FIELD-OBJ-UNIT-NAME | Name | Identity | unitName | String |
| ATTR-FIELD-OBJ-FACE | Face | Identity | face | String |
| ATTR-FIELD-OBJ-CALLSIGN | Call Sign | Identity | — | String |

**DoorStates gestures (wiki):** LMB cycle; RMB close; LMB+Alt open; LMB+Shift lock; LMB+Ctrl close.

The Mission Creator's counterpart of the object fields is in [Mission Creator counterpart](#mission-creator-counterpart).

## Comment (`CMT`)

Source: [Comment#Attributes](https://community.bistudio.com/wiki/Eden_Editor:_Comment)

| ID | Name | Category | Property | Type |
|----|------|----------|----------|------|
| ATTR-FIELD-CMT-TITLE | Title | Init | Name | String |
| ATTR-FIELD-CMT-TOOLTIP | Tooltip | Init | Description | String |
| ATTR-FIELD-CMT-POSITION | Position | Init | position | Position3D |

Editor-only; no gameplay effect. Can be saved in custom compositions.

## Group (`GRP`)

Source: [Group#Attributes](https://community.bistudio.com/wiki/Eden_Editor:_Group)

| ID | Name | Category | Property | Type |
|----|------|----------|----------|------|
| ATTR-FIELD-GRP-VARNAME | Variable Name | Init | Name | String |
| ATTR-FIELD-GRP-INIT | Init | Init | Init | String |
| ATTR-FIELD-GRP-CALLSIGN | Callsign | Init | groupID | String |
| ATTR-FIELD-GRP-PLACEMENT-RADIUS | Placement Radius | Init | placementRadius | Number |
| ATTR-FIELD-GRP-COMBAT-MODE | Combat Mode | State | combatMode | String |
| ATTR-FIELD-GRP-BEHAVIOUR | Behavior | State | behaviour | String |
| ATTR-FIELD-GRP-FORMATION | Formation | State | formation | Number |
| ATTR-FIELD-GRP-SPEED-MODE | SpeedMode | State | speedMode | String |
| ATTR-FIELD-GRP-DYN-SIM | Enable Dynamic Simulation | State | dynamicSimulation | Boolean |
| ATTR-FIELD-GRP-DELETE-EMPTY | Delete when Empty | State | — | Boolean |

**Combat mode options:** Forced Hold Fire, Do Not Fire Unless Fired Upon (± Keep Formation), Open Fire (± Keep Formation).

**Behaviour options:** Careless, Safe, Aware, Combat, Stealth.

## Marker (`MRK`)

Source: [Marker#Attributes](https://community.bistudio.com/wiki/Eden_Editor:_Marker)

| ID | Name | Category | Property | Type |
|----|------|----------|----------|------|
| ATTR-FIELD-MRK-TYPE | Type | Type | itemClass | String |
| ATTR-FIELD-MRK-VARNAME | Variable Name | Init | markerName | String |
| ATTR-FIELD-MRK-TEXT | Text | Init | text | String |
| ATTR-FIELD-MRK-POSITION | Position | Transformation | position | Position3D |
| ATTR-FIELD-MRK-SIZE | Size | Transformation | size2 | Array |
| ATTR-FIELD-MRK-ROTATION | Rotation | Transformation | rotation | Number |
| ATTR-FIELD-MRK-SHAPE | Shape | Style | markerType | Number |
| ATTR-FIELD-MRK-BRUSH | Brush | Style | brush | String |
| ATTR-FIELD-MRK-COLOR | Color | Style | baseColor | Array |
| ATTR-FIELD-MRK-ALPHA | Alpha | Style | alpha | Number |

**Marker kinds:** Icon (fixed screen size) vs Area (meters in world space).

## Layer (`LYR`)

Source: [Layer#Attributes](https://community.bistudio.com/wiki/Eden_Editor:_Layer)

| ID | Name | Category | Property | Type |
|----|------|----------|----------|------|
| ATTR-FIELD-LYR-NAME | Name | Init | Name | String |
| ATTR-FIELD-LYR-ENABLE-XFORM | Enable Transformation | Init | Transformation | Boolean |
| ATTR-FIELD-LYR-ENABLE-VIS | Enable Visibility | Init | Visibility | Boolean |

**Not scenario visibility** — `Enable Visibility` hides in editor only.

## Scenario (`SCN`)

Source: [Scenario Attributes](https://community.bistudio.com/wiki/Eden_Editor:_Scenario_Attributes)

Eden calls the document it edits a scenario; the Mission Creator's equivalent is the
[mission](/documentation_v2/glossary/g_to_m.md#mission); this section keeps Eden's word for it.

### General (Presentation)

| ID | Name | Property | Type |
|----|------|----------|------|
| ATTR-FIELD-SCN-TITLE | Title | IntelBriefingName | String |
| ATTR-FIELD-SCN-AUTHOR | Author | Author | String |
| ATTR-FIELD-SCN-PICTURE | Picture | OverviewPicture | String |
| ATTR-FIELD-SCN-OVERVIEW-TEXT | Text | OverviewText | String |
| ATTR-FIELD-SCN-DLC | DLC | AppId | Number |
| ATTR-FIELD-SCN-REQUIRE-DLC | Require DLC | — | — |

### Environment (Intel)

See full wiki table — includes time of day, weather, fog, wind, view distance, etc. (`Intel` section).

| ID | Name | Property | Type |
|----|------|----------|------|
| ATTR-FIELD-SCN-TIME | Time of day | — | — |
| ATTR-FIELD-SCN-WEATHER | Weather / overcast | — | — |
| ATTR-FIELD-SCN-FOG | Fog | — | — |
| ATTR-FIELD-SCN-WIND | Wind | — | — |
| ATTR-FIELD-SCN-VIEW-DIST | View distance | — | — |

The Mission Creator's Mission Settings dialog writes the time of day and the weather preset; it has no fog, wind or view distance field (see [Mission Creator counterpart](#mission-creator-counterpart)).

### Multiplayer / Garbage Collection

| Section | Wiki anchor |
|---------|-------------|
| Multiplayer | [Scenario Attributes#Multiplayer](https://community.bistudio.com/wiki/Eden_Editor:_Scenario_Attributes) |
| Garbage Collection | `GarbageCollection` section — performance cleanup |

## Trigger (`TRG`)

Source: [Trigger#Attributes](https://community.bistudio.com/wiki/Eden_Editor:_Trigger) — scrape: `.ai/artifacts/eden-wiki/Eden_Editor__Trigger.md`

| ID | Name | Category | Property | Type |
|----|------|----------|----------|------|
| ATTR-FIELD-TRG-VARNAME | Variable Name | Init | name | String |
| ATTR-FIELD-TRG-TEXT | Text | Init | text | String |
| ATTR-FIELD-TRG-POSITION | Position | Transformation | position | Position3D |
| ATTR-FIELD-TRG-ROTATION | Rotation | Transformation | rotation | Number |
| ATTR-FIELD-TRG-SIZE | Size | Transformation | size3 | Array |
| ATTR-FIELD-TRG-SHAPE | Shape | Transformation | IsRectangle | Boolean |
| ATTR-FIELD-TRG-TYPE | Type | Activation | TriggerType | String |
| ATTR-FIELD-TRG-ACTIVATION | Activation | Activation | ActivationBy | String |
| ATTR-FIELD-TRG-ACTIVATION-TYPE | Activation Type | Activation | activationType | String |
| ATTR-FIELD-TRG-CONDITION | Condition | Activation | condition | String |
| ATTR-FIELD-TRG-ON-ACTIVATION | On Activation | Effects | onActivation | String |
| ATTR-FIELD-TRG-REPEATABLE | Repeatable | Effects | repeatable | Boolean |
| ATTR-FIELD-TRG-TIMER | Timer | Effects | timeout | Number |

**Connections:** Set Trigger Owner (group-specific activation options). Area scalable on map when selected.

## Waypoint (`WP`)

Source: [Waypoint#Attributes](https://community.bistudio.com/wiki/Eden_Editor:_Waypoint) — scrape: `.ai/artifacts/eden-wiki/Eden_Editor__Waypoint.md`

| ID | Name | Category | Property | Type |
|----|------|----------|----------|------|
| ATTR-FIELD-WP-TYPE | Type | Type | itemClass | String |
| ATTR-FIELD-WP-DESCRIPTION | Description | Init | description | String |
| ATTR-FIELD-WP-ORDER | Order | Init | order | Number |
| ATTR-FIELD-WP-POSITION | Position | Transformation | position | Position3D |
| ATTR-FIELD-WP-COMBAT-MODE | Combat Mode | State | combatMode | String |
| ATTR-FIELD-WP-BEHAVIOUR | Behaviour | State | behaviour | String |
| ATTR-FIELD-WP-FORMATION | Formation | State | formation | Number |
| ATTR-FIELD-WP-SPEED | Speed | State | speed | String |
| ATTR-FIELD-WP-CONDITION | Condition | Completion | condition | String |

**Placement:** F4 browser or Shift+RMB quick MOVE. Attach by drag onto object; detach by drag away. Completion can require connected triggers (`CONN-WP-ACT-001`).

## System (`SYS`)

Full field tables vary per module — see [System](https://community.bistudio.com/wiki/Eden_Editor:_System) and scrape `.ai/artifacts/eden-wiki/Eden_Editor__System.md`.

## Composition metadata (not entity attributes)

Custom compositions have **title, author, category** in save/edit dialog — not standard entity attributes.

| ID | Field | Source |
|----|-------|--------|
| ATTR-FIELD-COMP-TITLE | Title | [Custom Composition#Saving](https://community.bistudio.com/wiki/Eden_Editor:_Custom_Composition) |
| ATTR-FIELD-COMP-AUTHOR | Author | same |
| ATTR-FIELD-COMP-CATEGORY | Category | same |

Saved payload includes: attribute values, layer structure, visibility, connections ([Custom Composition](https://community.bistudio.com/wiki/Eden_Editor:_Custom_Composition)).


## Mission Creator counterpart

What the Mission Creator holds for each Eden attribute group, read from the code. Parity per ID is
the [Eden gap analysis](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/eden_gap_analysis.md)'s,
and the dialog's entries are in the feature inventory's
[attributes and settings](/documentation_v2/website/frontend/apps/editor/feature_inventory/attributes_and_settings.md)
area.

The Mission Creator's Attributes dialog edits one or several placed
[slots](/documentation_v2/glossary/n_to_z.md#slot), or one placed vehicle, in four tabs: "Transform",
"Identity", "States" and "Arsenal" (`TABS` in
`apps/website/frontend/src/v2/apps/editor/ui/inspector/attributes_modal/field_gates_and_labels.rs`).
Over a multi-selection, a field whose values differ stays locked until its "Apply to all" box is
ticked, as Eden's per-field checkbox does.

| Eden attribute group | Mission Creator counterpart | Code |
|---|---|---|
| Object Transformation (position, rotation) | the "Transform" tab: X, Y and Z, "Rotation", and "Stance" ("Standing", "Crouched", "Prone") | `apps/website/frontend/src/v2/apps/editor/ui/inspector/attributes_modal/spatial_transform_tab.rs` |
| Object Type, Control and Identity (type, role description, name, call sign) | the "Identity" tab: "Type", "Role", "Role Description", "Tag", "Faction" and "Squad"; no name, face or call sign field | `apps/website/frontend/src/v2/apps/editor/ui/inspector/attributes_modal/identity_tab.rs`, `apps/website/frontend/src/v2/apps/editor/ui/inspector/attributes_modal/faction_and_squad_reassignment.rs` |
| Object States and Special States (skill, health, fuel, simulation, …) | the "States" tab, a placeholder: "Medic (soon)" and "Engineer (soon)" with no control | `states_tab` in `apps/website/frontend/src/v2/apps/editor/ui/inspector/attributes_modal.rs` |
| Loadout (Eden's Arsenal, outside the Attributes dialog) | the "Arsenal" tab: the slot's loadout rows, a 3D paper doll, and loadout export and import | `apps/website/frontend/src/v2/apps/editor/arsenal/` |
| Vehicle crew and cargo | the vehicle view: "Heading", "Cargo" rows and "Crew" seats (driver, gunner, commander and cargo seats) | `apps/website/frontend/src/v2/apps/editor/ui/inspector/attributes_modal/vehicle_attributes.rs` |
| Comment (title, tooltip, position) | a comment's title and tooltip, edited in the comment editor overlay | `apps/website/frontend/src/v2/apps/editor/bridge/overlays/comment_editor.rs` |
| Layer (name, Enable Transformation, Enable Visibility) | an editor layer's name, "locked" and "hidden" flags, both editor-only as in Eden | `apps/website/map-engine/src/data/store/rows/layers.rs` |
| Scenario General and Environment | the Mission Settings dialog: "Presentation" (briefing, thumbnail link), "Mission shape", "Time", "Weather", "Mission flow" and the map display settings | `apps/website/frontend/src/v2/apps/editor/ui/modals/settings_modal/` |
| Composition metadata (title, author, category) | a saved composition's title, category and author, set on save and edited in the Compositions tab | `apps/website/frontend/src/v2/apps/editor/ui/docks/dock_right/compositions/` |

The environment keys the editor writes into the document are listed in `CARRIED_ENV_KEYS`
(`apps/website/frontend/src/v2/apps/editor/ui/inspector/env.rs`): `time` and `weather`, which the
compiler carries into the mission, and the map display keys; `author_env` refuses any other key,
so fog, wind and view distance have no field.
