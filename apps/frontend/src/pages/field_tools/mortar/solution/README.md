# Mortar calculator solution panel

What the last Calculate produced: the input problems, or the solved fire mission worded for the
gun line.

## Contents

```text
apps/frontend/src/pages/field_tools/mortar/solution/
├── battery_rows.rs     one line per gun: the laid charge, elevation, aim azimuth and time of flight
├── charges_table.rs    one gun's table of every charge, the laid one highlighted
├── crest_warning.rs    whether the lead gun's flight clears the terrain, or why there is no check
├── dispersion_card.rs  the lead gun's probable errors and 50 % ellipse, labelled an interpretation
├── fuze_card.rs        the time-fuze setting for a burst above the target, its window and burst-point lay
└── mod.rs              the panel: problems, or battery rows, crest, fuze, dispersion and the charge tables
```

## How it works

`mod.rs` renders the `SolveOutcome`: either the list of input problems or a solved mission, whose
`FireMissionSolution` comes from the map engine's `solve_fire_mission` unchanged. Every number is
worded here, in the weapon's mils and in degrees through the solve bridge's `mils_and_degrees`; the
dispersion card carries "Interpretation, not verified in-engine".

## Boundaries

- Depends on: `map_engine::data::scenario::ballistics` (`battery`, `crest_clearance`,
  `dispersion`, `fire_mission`, `fuze`); the ballistics catalog DTOs; the solve bridge.
- Used by: the mortar page (`page.rs`).
- Rules: the panel only words the engine's solution and never computes a firing number.

## Related documentation

- [Mortar calculator page](/documentation/apps/frontend/pages/field_tools/mortar/mortar_calculator_page.md)
  — the page's behaviour, its data and its decisions.
