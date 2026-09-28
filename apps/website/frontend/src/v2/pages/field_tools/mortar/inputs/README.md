# Mortar calculator inputs

What the battery fires, from where, onto what and through which wind: the drafts the operator types
or the map writes, their resolution into the map engine's fire-mission inputs, and the fields that
edit them.

## Contents

```text
apps/website/frontend/src/v2/pages/field_tools/mortar/inputs/
├── battery.rs           one to twelve labelled guns, adding and removing them, each resolved into a gun position
├── illumination.rs      the burst height of a time-fuzed shell and the fuze window it must fit
├── mod.rs               the five input groups and the control class they share
├── positions.rs         the terrain choice and one position: grid text, height source, manual height, resolution
├── weapon_and_shell.rs  the catalog's weapons, the chosen weapon's shells and every charge, kept valid on change
└── wind.rs              the wind speed and "from" direction, parsed into the fire-mission wind or calm air
```

## How it works

Each group keeps a draft of what was typed and resolves it on Calculate: a position parses its 6-,
8- or 10-figure grid to the cell centre (`camera::grid_reference::parse_grid`) and takes its height
from the terrain heights the map fills or from the typed value, never a zero; the battery resolves
every gun the same way; the weapon and shell selection is reconciled whenever the catalog or the
weapon changes. Each resolution answers every problem it finds, so the page lists them all at once.

## Boundaries

- Depends on: the ballistics catalog DTOs of `crate::v2::core::api::dto::ballistics_catalogs`;
  `website_map_engine` (`camera::grid_reference`, the fire-mission input types of
  `data::scenario::ballistics::fire_mission`); the terrain heights of `crate::v2::core::map_view`.
- Used by: the page, the solve bridge, the map picker and the save area of the mortar page.
- Rules: a height that has not loaded is an error, never zero; Arland resolves only manual heights.

## Related documentation

- [Mortar calculator page](/documentation_v2/website/frontend/pages/field_tools/mortar/mortar_calculator_page.md)
  — the page's behaviour, its data and its decisions.
