**Status:** live

# Eden editor UI anatomy

The Arma 3 Eden editor's workspace, panel by panel, as the Bohemia wiki describes it: the reference
design the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) is measured against.
Each panel cites its wiki pages, scraped into `.ai/artifacts/eden-wiki/`; the panel names in
backticks are the UI surfaces of the
[feature entry schema](/documentation_v2/website/frontend/apps/editor/feature_inventory/feds_schema.md#eden-reference-entries).
The [interactions catalog](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/README.md)
holds what each panel does, the [attribute catalog](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/attributes.md)
the fields of its dialogs, and the last section the Mission Creator's counterpart of each panel.

## Workspace layout (top → bottom, left → right)

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ Menu Bar: Scenario | Edit | View | Settings | Attributes | Tools | Play | Help │
├─────────────────────────────────────────────────────────────────────────────┤
│ Toolbar: New Open Save Workshop Undo Redo Widgets Snap Grids Intel Map NVG …   │
├──────────────┬──────────────────────────────────────────────┬───────────────┤
│ Entity List  │              View (3D scene or Map)           │ Asset Browser │
│ (left)       │         + transformation widget overlay       │ (right)       │
│              │         + entity icons / connection lines      │               │
├──────────────┴──────────────────────────────────────────────┴───────────────┤
│ Status Bar: X | Y | elevation | zoom/resolution | version | mods | server    │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Evidence:** [Menu Bar](https://community.bistudio.com/wiki/Eden_Editor:_Menu_Bar), [Toolbar](https://community.bistudio.com/wiki/Eden_Editor:_Toolbar), [Status Bar](https://community.bistudio.com/wiki/Eden_Editor:_Status_Bar), [Terminology](https://community.bistudio.com/wiki/Eden_Editor:_Terminology).

## Menu Bar (`MenuBar`)

Displayed at top of workspace; contains almost all actions sorted into categories.

| Menu | Contents (summary) | Wiki |
|------|-------------------|------|
| **Scenario** | New, Open, Save, Save As, Publish Workshop, Export, Exit | [Menu Bar#Scenario](https://community.bistudio.com/wiki/Eden_Editor:_Menu_Bar#Scenario) |
| **Edit** | Undo, Redo, Select All on Screen, Transformation Widget, Grid (trans/rot/area), Vertical Mode, Surface Snap, Waypoint Snap, Phase, Asset Type, Toggle Asset Sub-type | [Menu Bar#Edit](https://community.bistudio.com/wiki/Eden_Editor:_Menu_Bar#Edit) |
| **View** | Center random/player, Toggle Map, Map Textures, Vision Mode, Flashlight, Location Labels, Search (browser/list), Interface toggles | [Menu Bar#View](https://community.bistudio.com/wiki/Eden_Editor:_Menu_Bar#View) |
| **Settings** | Preferences, Video, Audio, Game, Controls | [Menu Bar#Settings](https://community.bistudio.com/wiki/Eden_Editor:_Menu_Bar#Settings) |
| **Attributes** | General, Environment, Multiplayer, Garbage Collection scenario dialogs | [Menu Bar#Attributes](https://community.bistudio.com/wiki/Eden_Editor:_Menu_Bar#Attributes) |
| **Tools** | Debug Console, Functions Viewer, Config Viewer | [Menu Bar#Tools](https://community.bistudio.com/wiki/Eden_Editor:_Menu_Bar#Tools) |
| **Play** | SP, SP+Briefing, SP at camera, MP preview | [Menu Bar#Play](https://community.bistudio.com/wiki/Eden_Editor:_Menu_Bar#Play) |
| **Help** | Documentation, Scripting, Wiki, Forums, Tracker, Dev Hub, Tutorials | [Menu Bar#Help](https://community.bistudio.com/wiki/Eden_Editor:_Menu_Bar#Help) |

**View → Interface** can show/hide: Entity List, Asset Browser, Controls Hint, Navigation Widget.

## Toolbar (`Toolbar`)

Quick-access buttons at top of workspace (below or integrated with menu area).

| Button / control | What it does |
|------------------|--------------|
| New scenario | Dialog to create new scenario |
| Open | Dialog to open scenario |
| Save | Save current; Save As if never saved |
| Workshop | Publish saved scenario to Steam Workshop |
| Undo / Redo | Last operation / redo undone |
| Widget off | Toggle transformation widget off |
| Translation widget | Axis drag-move widget |
| Rotation widget | Axis rotation widget |
| Area scaling widget | Resize trigger/marker areas |
| Area widget | Size + orient trigger/marker areas |
| Global / Local coords | Widget aligned to world vs entity axes |
| Vertical mode | Surface level vs sea level altitude preservation |
| Ground snap | Snap to ground during translation |
| Translation grid | Toggle/set move grid (`;` shortcut per forums) |
| Rotation grid | Toggle/set rotation grid |
| Area scaling grid | Toggle/set area grid |
| Scenario attributes (intel) | Open scenario attributes dialog |
| Scene / Map view | Toggle 3D vs 2D map |
| Flashlight | Camera-attached light (night editing) |
| Vision mode | Normal / NVG / Thermal |
| Phase | Select scenario phase |
| Tutorials | In-editor tutorial picker |

**Evidence:** [Toolbar](https://community.bistudio.com/wiki/Eden_Editor:_Toolbar) — scraped `.ai/artifacts/eden-wiki/Eden_Editor__Toolbar.md`.

**Note:** `Space` cycles transformation widget variants when widget active ([Transformation Widget](https://community.bistudio.com/wiki/Eden_Editor:_Transformation_Widget#Variants)).

## Entity List — left panel (`EntityList`)

| Element | Description |
|---------|-------------|
| **Tree** | All scenario entities; groups as folders with members + waypoints inside |
| **Layers** | Folder nodes; can nest sub-layers |
| **New Layer** | Button — creates layer from selection, or empty layer if nothing selected |
| **Delete** | Del key or Delete button on selected layer |
| **Search** | Focus via View → Search → Entity List (`SearchEdit` action) |
| **Layer attrs** | Name, Enable Transformation, Enable Visibility (editor-only hide) |

**Group display:** Icon above leader; click icon selects all members. In list, whole group moves between layers — dragging one member moves entire group.

**Evidence:** [Layer](https://community.bistudio.com/wiki/Eden_Editor:_Layer), [Group](https://community.bistudio.com/wiki/Eden_Editor:_Group), [Actions#SearchEdit](https://community.bistudio.com/wiki/Eden_Editor:_Actions).

**Mission Creator counterpart:** Eden's Entity List and layers correspond to the Mission Creator's
[ORBAT](/documentation_v2/glossary/n_to_z.md#orbat) tree (the export hierarchy) and **editor layers** (workflow folders). They are **not** the
same data model: the [feature entry schema's terms](/documentation_v2/website/frontend/apps/editor/feature_inventory/feds_schema.md#terms)
keep them apart.

### Comments in Entity List

Comments are virtual editor-only entities. They appear in the tree like other entities, support drag/copy/layers/compositions. Placed via RMB empty → Place Comment.

**Evidence:** [Comment](https://community.bistudio.com/wiki/Eden_Editor:_Comment).

## Asset Browser — right panel (`AssetBrowser`)

The primary placement UI. Shows all placeable **assets** (not yet entities).

### Structure (top → bottom)

| Region | Name | What you see |
|--------|------|--------------|
| 1 | **Mode** | Tabs: Object, Composition, Trigger, Waypoint, System, Marker — **F1–F6** |
| 2 | **Submode** | Faction/side filters (e.g. BLUFOR) — not on Trigger/Waypoint modes |
| 3 | **Search** | Filter tree (see search syntax below) |
| 4 | **List** | Category → subcategory → leaf assets (icon = placeable) |
| 5 | **Vehicle toggle** | Place with crew on/off; Alt inverts while placing |
| 6 | **Composition tools** | New / Edit / Delete / Publish / Workshop buttons (Composition mode) |

### Mode → F-key mapping

| Mode | Asset types | F-key |
|------|-------------|-------|
| Object | Units, vehicles, props | F1 |
| Composition | Predefined + custom groups | F2 |
| Trigger | Trigger modules | F3 |
| Waypoint | Waypoint types | F4 |
| System | Logic/modules (intel, respawn, …) | F5 |
| Marker | Map markers | F6 |

### Search syntax

| Syntax | Example | Purpose |
|--------|---------|---------|
| `class ` prefix | `class B_Soldier` | Search by class name |
| `mod ` prefix | `mod kart` | Filter by mod (also dropdown) |
| `*` `?` wildcards | `house*ruin` | Pattern match (2.22+) |
| `/` prefix | `/(Brick` | Regular expression (2.22+) |

### List hierarchy

1. **Category** — e.g. NATO faction or Furniture (props)
2. **Subcategory** — e.g. Men → Rifleman
3. **Entity (leaf)** — icon, draggable / click-to-place

Category folders have **no icon**; only leaves are placeable.

**Evidence:** [Asset Browser](https://community.bistudio.com/wiki/Eden_Editor:_Asset_Browser), [Object Categorization](https://community.bistudio.com/wiki/Eden_Editor:_Object_Categorization), [Entity Placing](https://community.bistudio.com/wiki/Eden_Editor:_Entity_Placing).

### Compositions subtree

- **Compositions → Custom** — user-saved compositions
- **Predefined compositions** — infantry/tank formations **and prop camps** (F2; the 2D Editor called them "Groups")
- **Steam subscribed content** — Workshop compositions
- Buttons: save, edit metadata, publish, subscribe browse, unsubscribe

**Evidence:** [Custom Composition](https://community.bistudio.com/wiki/Eden_Editor:_Custom_Composition), [Switching from 2D Editor#Groups Mode](https://community.bistudio.com/wiki/Eden_Editor:_Switching_from_2D_Editor).

### Vehicle crew UI

When hovering a vehicle, a **crew panel** lists Driver/Pilot, Commander, Turret, Passenger roles. Visible crew (e.g. car passengers) also show icons on the model. Drag character onto vehicle to board; drag out to unboard; RMB → Change Seat.

**Evidence:** [Transforming Crew](https://community.bistudio.com/wiki/Eden_Editor:_Transforming_Crew).

## View — center (`View`)

| Mode | Content |
|------|---------|
| **Scene (3D)** | Terrain mesh, entity models/icons, transformation widget, connection lines |
| **Map (2D)** | Top-down map; area draw for triggers/markers; zoom = m/px in status bar |

**Overlays when editing:**

- Selected entity highlight
- Transformation widget (translation / rotation / area widgets)
- Connection lines (color per type; terrain-occluded unless selected)
- Group icon stacked when zoomed out

**Basic manipulation in view (no widget):**

| Gesture | Effect |
|---------|--------|
| LMB drag entity | Move position |
| Alt + drag | Change altitude |
| Shift + drag | Rotate to face cursor |
| LMB drag empty | (context-dependent; marquee in some modes) |

**Evidence:** [Entity Transforming](https://community.bistudio.com/wiki/Eden_Editor:_Entity_Transforming).

## Attributes dialog (`AttributesDialog`)

Opens on **double-click** entity, or RMB → Attributes (multi-select supported).

- Tabbed categories per entity type (Object, Group, Marker, …)
- Fields map to `property` names in mission SQM (see the [attribute catalog](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/attributes.md))
- Transform fields mirror widget/drag edits (position, rotation, size)
- **Multi-select:** RMB → Attributes (not dbl-click). Fields with differing values are disabled until per-field checkbox enabled ([Switching from 2D Editor](https://community.bistudio.com/wiki/Eden_Editor:_Switching_from_2D_Editor#Editing_Multiple_Entities))

**Evidence:** [Entity Attributes](https://community.bistudio.com/wiki/Eden_Editor:_Entity_Attributes), [Setting Attributes](https://community.bistudio.com/wiki/Eden_Editor:_Setting_Attributes).

## Scenario attributes (`ScenarioAttributes`)

Opened via Toolbar intel button or Menu → Attributes.

| Section | Class | Examples |
|---------|-------|----------|
| General | Presentation | Title, Author, Picture, Overview text, DLC |
| Environment | Intel | Time, weather, fog, wind, view distance |
| Multiplayer | Multiplayer | Respawn, slots, difficulty |
| Garbage Collection | Performance | Cleanup distances |

**Evidence:** [Scenario Attributes](https://community.bistudio.com/wiki/Eden_Editor:_Scenario_Attributes).

## Context menu (`ContextMenu`)

**RMB** on entity → hierarchical menu (Connect, Log, Transform, Attributes, …).

**Connect** submenu starts connection mode:

| Type | Ends | Purpose |
|------|------|---------|
| Grouping | Character ↔ Character | Form squad; leader = target |
| Syncing | Character ↔ Object | Generic script sync |
| Trigger owner | Trigger ↔ Character | Object-based activation |
| Random start | Object ↔ Marker | Random spawn position |
| Waypoint activation | Waypoint ↔ Trigger | Complete WP when triggers fire |

**Ctrl + drag** line between characters = quick group ([Connecting](https://community.bistudio.com/wiki/Eden_Editor:_Connecting)).

## Status Bar (`StatusBar`)

| Entry | Shows |
|-------|-------|
| X | Cursor West→East (meters) |
| Y | Cursor South→North (meters) |
| Z | Elevation at cursor |
| Zoom | 3D: camera-to-cursor distance; Map: m/pixel |
| Version | Game version (copyable) |
| Mods | White when unofficial mods loaded |
| Server | White when background MP server running |

**Evidence:** [Status Bar](https://community.bistudio.com/wiki/Eden_Editor:_Status_Bar).

## Mission Creator counterpart

The Mission Creator's counterpart of each Eden panel, read from the code; the in-code README of
each folder holds the detail. Parity per ID is the
[Eden gap analysis](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/eden_gap_analysis.md)'s,
and the feature inventory area named in each row lists the features.

| Eden surface | Mission Creator counterpart | Code and inventory area |
|---|---|---|
| Asset Browser, modes F1–F6 | the right dock's seven tabs: "Factions", "Vehicles", "Zones", "Compositions", "Triggers", "Favourites" and "Markers", with no F-key bindings; the Factions tab's side chips "BLUFOR", "OPFOR", "INDFOR" and "Objects", and a disabled "Custom" chip; a search grammar of `class:`, `mod:`, wildcards and `/…/` expressions | [right dock](/apps/website/frontend/src/v2/apps/editor/ui/docks/dock_right/README.md); [right asset palette](/documentation_v2/website/frontend/apps/editor/feature_inventory/right_asset_palette.md) |
| Entity List and layers | the left dock's Layers tab (the editor layers tree, with search) and Locations tab, and the ORBAT tree of factions, squads and [slots](/documentation_v2/glossary/n_to_z.md#slot); editor layers are not Eden layers | [left dock](/apps/website/frontend/src/v2/apps/editor/ui/docks/dock_left/README.md), [outliner](/apps/website/frontend/src/v2/apps/editor/ui/outliner/README.md); [left sidebar](/documentation_v2/website/frontend/apps/editor/feature_inventory/left_sidebar.md) |
| Menu Bar, eight menus | the top strip's six menus: "File", "Edit", "Arrange", "Mission", "Environment" and "Help", each row running an action | [top strip](/apps/website/frontend/src/v2/apps/editor/ui/docks/top_strip/README.md); [top command strip](/documentation_v2/website/frontend/apps/editor/feature_inventory/top_command_strip.md) |
| Toolbar | the top strip's undo and redo, the transform widget variants (none, translation, rotation), the snap grid toggle and step, the time of day and weather, "Save Version" and "Export"; no area widgets, vertical mode, surface snap, flashlight, vision mode, phase or tutorials | [top strip](/apps/website/frontend/src/v2/apps/editor/ui/docks/top_strip/README.md); [top command strip](/documentation_v2/website/frontend/apps/editor/feature_inventory/top_command_strip.md) |
| Status Bar | the bottom status bar: X, Y and Z of the cursor ("CUR", Z sampled from the terrain's elevation grid once it loads) or of the one selected entity ("SEL"), the object and selection counts, the save size, the map scale and the ruler; no version, mods or server entries | [bottom toolbelt](/apps/website/frontend/src/v2/apps/editor/ui/docks/toolbelt/README.md); [bottom toolbelt area](/documentation_v2/website/frontend/apps/editor/feature_inventory/bottom_toolbelt.md) |
| View | a top-down map through the map engine's orthographic camera; no 3D scene | [map viewport and camera](/documentation_v2/website/frontend/apps/editor/feature_inventory/map_viewport_and_camera.md) |
| Attributes dialog | the Attributes dialog: "Transform", "Identity", "States" and "Arsenal" tabs, or the vehicle view, with per-field "Apply to all" over a multi-selection | [Attributes dialog parts](/apps/website/frontend/src/v2/apps/editor/ui/inspector/attributes_modal/README.md); [attributes and settings](/documentation_v2/website/frontend/apps/editor/feature_inventory/attributes_and_settings.md) |
| Scenario attributes | the Mission Settings dialog: presentation, mission shape, time, weather, mission flow and map display | [Mission Settings dialog parts](/apps/website/frontend/src/v2/apps/editor/ui/modals/settings_modal/README.md); [attributes and settings](/documentation_v2/website/frontend/apps/editor/feature_inventory/attributes_and_settings.md) |
| Context menu | the map context menu: "Connect" ("Sync to", "Group to", "Set Trigger Owner"), "Transform" (the formations), "Arrange", "Go Here", "Place Comment", "Connections...", "Edit Loadout..." and "Attributes..."; its other rows are disabled with a reason | [map context menu](/apps/website/frontend/src/v2/apps/editor/ui/docks/context_menu/README.md) |
| Compositions (F2) | the Compositions tab: save the selection with a title and category, then list, arm, edit and delete saved compositions; no Workshop publishing | [compositions](/apps/website/frontend/src/v2/apps/editor/ui/docks/dock_right/compositions/README.md); [right asset palette](/documentation_v2/website/frontend/apps/editor/feature_inventory/right_asset_palette.md) |
| Comments | "Place Comment" in the context menu places a comment in the active layer; its title and tooltip are edited in the comment editor overlay | [map context menu](/apps/website/frontend/src/v2/apps/editor/ui/docks/context_menu/README.md) |
| Vehicle crew panel | no hover crew panel; the Vehicles tab's "Place with crew" checkbox, with Alt held while placing forcing an empty vehicle rather than inverting the checkbox (`vehicle_places_its_crew` in `apps/website/map-engine/src/data/store/operations/entity/armed_placement.rs`), and the vehicle view's "Crew" seats in the Attributes dialog | [right dock](/apps/website/frontend/src/v2/apps/editor/ui/docks/dock_right/README.md), [Attributes dialog parts](/apps/website/frontend/src/v2/apps/editor/ui/inspector/attributes_modal/README.md) |
