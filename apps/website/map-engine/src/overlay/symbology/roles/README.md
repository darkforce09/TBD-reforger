# Role, vehicle and side classes

The one table that turns a [slot](/documentation_v2/glossary.md#slot)'s role or kit string into
one of five unit glyph classes, a vehicle alias into one of three silhouettes, and a faction key
into its side tint. The symbology is bespoke: no MIL-STD-2525 or APP-6 affiliation frames, no
echelon modifiers and no civilian side.

## Contents

```text
apps/website/map-engine/src/overlay/symbology/roles/
├── classify.rs  side tints, `UnitRoleClass`, `VehicleKind` and the two lookup tables
└── mod.rs       the module tree
```

## How it works

`unit_role_class` normalises the string (lowercase; `-`, space and `/` become `_`; a `kit:`,
`veh:`, `vehicle:`, `preset:` or `comp:` prefix is dropped), then matches whole `_`-separated tokens
and a few phrases in a fixed order: leader, medic, anti-tank, machine gun, else rifleman. Each class
is a knockout in the unit disc (chevron, plus, down-triangle, twin bars, plain disc), and its
discriminant is the class's cell offset in the atlas (`UNIT_ROLE_CLASS_COUNT`, 5).
`vehicle_kind_for_alias` matches substrings: tracked and armoured names (`m113`, `btr`, `bmp`,
`tank`, …) are `Apc`, trucks (`m923`, `ural`, `cargo`, …) are `Truck`, and everything else is
`WheeledLight` (`VEHICLE_KIND_COUNT`, 3). `side_rgba` maps `BLUFOR`, `OPFOR` and `INDFOR` to
`SIDE_BLUFOR_RGBA`, `SIDE_OPFOR_RGBA` and `SIDE_INDFOR_RGBA`; any other key, the empty one
included, tints as BLUFOR.

## Boundaries

- Depends on: nothing outside the folder.
- Used by:
  - the symbology packers in `crate::overlay::symbology::instances` (re-exported by its `slots`
    module), `crate::overlay::symbology::links::squad_links` and
    `crate::editing::lanes::markers`;
  - the [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s document host and canvas
    mount
    (`apps/website/frontend/src/v2/apps/editor/bridge/document_host/history.rs`,
    `apps/website/frontend/src/v2/apps/editor/bridge/document_host/history/render_lanes.rs`,
    `apps/website/frontend/src/v2/apps/editor/mission_editor/canvas_mount/boot_tasks.rs`), which
    pack side tints.
- Rules: no other file re-derives the role or alias mapping
  (`every_seeded_kit_maps_and_unknown_defaults`, `authored_role_strings_and_token_boundaries` and
  `seeded_vehicles_map_to_silhouette_kinds` in
  `apps/website/map-engine/src/overlay/symbology/instances/slots/tests/cases_1.rs`); the three side
  tints stay distinct and are pinned to their RGBA literals by
  `cargo xtask verify editor-orbat-coherency`.
