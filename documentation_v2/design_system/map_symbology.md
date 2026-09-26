**Status:** live

# Map symbology

The symbols that stand for units, vehicles and briefing markers: how the
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s map draws a
[slot](/documentation_v2/glossary.md#slot), a vehicle and a marker, which colour marks each side,
and how the game [mod](/documentation_v2/glossary.md#mod) draws the same markers in game. The set
is bespoke: five unit roles, three vehicle kinds and three side tints, with no MIL-STD-2525 or
APP-6 frames, no echelon modifiers and no civilian side.

## Where it lives

- Map: [`apps/website/map-engine/src/overlay/symbology/`](/apps/website/map-engine/src/overlay/symbology/README.md):
  the role, vehicle and side tables in `roles/classify.rs`, the glyph atlas in `atlas/`, the
  marker glyphs in `markers.rs`, and the marker lane's parse in
  `apps/website/map-engine/src/editing/lanes/markers.rs`.
- Game: [`apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Markers/`](/apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Markers/README.md):
  `TBD_MarkerData.c` (which markers a player may see, and the wire), `TBD_MarkerIcons.c` (authored
  icon to engine icon) and `TBD_MarkerClient.c` (drawing).
- Contract: the `marker` definition in `contracts_v2/definitions/mission.schema.json`.
- Related: the [design tokens](/documentation_v2/design_system/design_tokens.md), whose palette the
  side tints come from, and the
  [tactical marker palette](/documentation_v2/mod/tbd-framework/UI/tactical_marker_palette/tactical_marker_palette_specification.md),
  the in-game marker behaviour and the drawing toolbar it designs.

## Behaviour

### Unit symbols

A slot draws as a white disc glyph tinted with its side's colour. `unit_role_class` reads the
slot's role or kit string (lowercased; `-`, space and `/` become `_`; a `kit:`, `veh:`,
`vehicle:`, `preset:` or `comp:` prefix dropped) and picks the first class whose tokens match:

| Class | Knockout in the disc | Matches, for example |
|---|---|---|
| `Leader` | chevron | `sl`, `tl`, `pl`, `co`, `xo`, leader, commander, officer |
| `Medic` | plus | medic, corpsman, `cls`, `doc`, aid, casevac |
| `AntiTank` | down-triangle | `at`, `atgm`, `rpg`, `law`, launcher, `at_gunner` |
| `MachineGun` | twin bars | `ar`, `mg`, `lmg`, `saw`, gunner, `automatic_rifleman` |
| `Rifleman` | none, a plain disc | every other role, the default |

Each glyph carries a facing point, and the atlas holds each class a second time with a selection
ring. The atlas cells are white on alpha, so the instance tint supplies both the side colour and
the selection colour. Squad members draw hairline tethers to their leader, and at extreme zoom
the slots merge into count clusters.

### Vehicle silhouettes

`vehicle_kind_for_alias` matches substrings of the vehicle's alias to one of three top-down
silhouettes: `Apc` for tracked and armoured names (`m113`, `btr`, `bmp`, `tank`, …), `Truck` for
trucks (`m923`, `ural`, `cargo`, …) and `WheeledLight`, the default, for everything else.

### Side tints

`side_rgba` maps a faction key to its tint; any other key, the empty one included, tints as
BLUFOR.

| Key | Constant | Colour | Palette match |
|---|---|---|---|
| `BLUFOR` | `SIDE_BLUFOR_RGBA` | `#adc6ff` | `primary` |
| `OPFOR` | `SIDE_OPFOR_RGBA` | `#f87171` | `error-alert` |
| `INDFOR` | `SIDE_INDFOR_RGBA` | `#22c55e` | `success` |

### Markers in the Mission Creator

A briefing marker draws one of eleven glyphs (`MARKER_GLYPH_COUNT`), chosen by
`marker_glyph_for_alias` from its `icon` alias after folding case and separators:

| Glyph | Family, for example |
|---|---|
| ring | `circle`, `area`, `zone`, `ao` |
| disc | `dot`, `point`, `marker`, and every unknown alias |
| square | objective: `objective`, `obj`, `target`, `task` |
| diamond | point of interest: `poi`, `intel`, `contact` |
| up-triangle | attack: `attack`, `assault`, `capture`, `seize` |
| down-triangle | defend: `defend`, `hold`, `garrison` |
| cross | medical: `medical`, `aid`, `casevac` |
| X | destroy: `destroy`, `demolish`, `sabotage` |
| flag | `flag`, `rally`, `base`, `hq`, `spawn` |
| chevron | waypoint: `waypoint`, `move`, `route`, `phase_line` |
| target | observation: `observation_post`, `op`, `overwatch`, `recon` |

The marker glyph atlas shares its ring and disc cells with the slot atlas. The marker is tinted
by its side: the lane takes the row's `factionId`, drops a `faction-` prefix and asks `side_rgba`.
Its `label` draws as a caption through the map's text pipeline.

### Markers in game

1. The server sends each player only their own side's markers, at most 64 (`MAX_MARKERS`), with
   labels cut to 64 characters; the
   [tactical marker palette](/documentation_v2/mod/tbd-framework/UI/tactical_marker_palette/tactical_marker_palette_specification.md)
   documents the flow.
2. Each client inserts them as the engine's own `PLACED_CUSTOM` map markers.
   `TBD_MarkerIcons.Resolve` tries the running game's icon names first, then the table of the 64
   authored aliases over `SCR_EScenarioFrameworkMarkerCustom`, and falls back to `DOT`, the same
   downgrade the map's disc makes.
3. An authored `color` maps to the nearest entry of the engine's placed-marker palette; a marker
   without one takes `REFORGER_ORANGE` (`MARKER_COLOR`). Rotation applies through the marker's
   `SetRotation`, size and alpha through `TBD_StyledMapMarker`. An area fill is not drawn,
   because the engine's marker has no brush API.

### Known discrepancies

- The same marker changes colour between the surfaces: the Mission Creator tints it by side and
  ignores `color` (`marker_lane_fields` in `apps/website/map-engine/src/editing/lanes/markers.rs`),
  while the game draws `color` or orange and ignores the side.
- The game's own interface paints BLUFOR and OPFOR from Tailwind's blue and red families and has
  no INDFOR tint (`ChipFill`, `FactionRowFill` in `TBD_UITheme.c`), while the map uses the three
  side tints above.
- `TBD_MarkerIcons.c:51-53` says the schema has no colour field, but `marker.color` exists and
  `TBD_MarkerData.c` carries it on the wire.

## Data

- `briefing.markers` rows of the mission document: `x`, `z`, `icon` (one of 64 aliases), `label`,
  and the optional `size`, `rotationDeg`, `shape`, `brush`, `color`, `alpha` and `area`
  (`#/$defs/marker` in `contracts_v2/definitions/mission.schema.json`).
- Slot `role` and kit strings, vehicle aliases and faction keys of the mission document feed the
  unit, vehicle and side tables.
- `cargo xtask verify editor-orbat-coherency` pins the three side tints to their RGBA literals;
  `every_schema_alias_maps` in `apps/website/map-engine/src/overlay/symbology/tests/markers_tests.rs`
  checks that every schema alias maps to a glyph.

## Design

The map follows no military symbol standard. Each unit is a tinted disc whose one knockout names
the role a mission maker reads the map by, and each marker is a single solid shape. Frames,
affiliation shapes, echelon marks and the civilian and unknown affiliations do not exist.
The in-game marker drawing toolbar with channel-scoped markers, which the tactical marker palette
specification designs, is not built.

## Open work

- [T-831 — Per-side marker authoring audit then explicit UI](/documentation_v2/tickets/specs/t831_per_side_markers_audit.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-831_plan.md)): the Mission Creator authors
  markers per side explicitly, which changes the rows and tints each side's markers carry.
- [T-1083 — Keep the marker client poll off dedicated servers](/.ai/tickets/T-1083.toml) (idea, no
  plan): the in-game marker client starts only where a player sees the map.

## Decisions

- One role table: `unit_role_class` and `vehicle_kind_for_alias` are the only mapping, and no other
  file re-derives it, so the map and every test agree on what a role draws as.
- An unknown alias draws the disc in the Mission Creator and `DOT` in game, never nothing: a marker
  the renderer did not understand still shows where it is.
- A side's markers never leave the server for another side's players: two sides may hold opposite
  orders at the same place.
