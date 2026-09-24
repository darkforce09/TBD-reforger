# Mortar firing solutions

The mortar solver behind the field tools: from a firing position and a target on the flat game
map, the distance, azimuth, elevation, charge and time of flight for one of the mortars whose
charge tables it holds. The [API](/documentation_v2/glossary.md#api) serves it to the mortar
calculator and records solved fire missions against an
[event](/documentation_v2/glossary.md#event).

## Contents

```text
apps/website/map-engine/src/data/scenario/ballistics/
├── mod.rs                   the module tree; re-exports the solver, its result and its error
├── mortar_fire_solution.rs  `solve_fire_mission`, the per-weapon charge tables and `FireSolution`
└── tests/                   unit tests for the geometry, the per-weapon pins and the two refusals
```

## How it works

`solve_fire_mission(weapon, fp_x, fp_y, tgt_x, tgt_y)` takes game-world metres, x east and y
north. It looks the weapon up by exact name (`M252 81mm` and `M821 81mm` share one table,
`2B14 82mm`, `M120 120mm`), then computes the distance and the grid azimuth, clockwise from north,
in degrees to 0.1 and in mils (6,400 to the circle). It picks the lowest charge whose muzzle
velocity reaches, `range × g / v² ≤ 1`, and answers the high-angle root: elevation in mils, the
charge's index from 0, and the time of flight to 0.1 s. `FireSolution` serialises in snake_case.

It refuses twice. An unknown name is `SolveError::UnknownWeapon` before any range work, so a
misspelled weapon never gets a range verdict. When no charge reaches, `SolveError::OutOfRange`
carries the partial solution: weapon, distance and both azimuths set; charge, elevation and time of
flight zero.

## Boundaries

- Depends on: `serde` and `thiserror`.
- Used by: `apps/website/api_v2/src/operations/handlers/fire_missions.rs`, whose
  `POST /api/v1/fire-missions/solve` and `POST /api/v1/fire-missions` answer an unknown weapon with
  400 and an unreachable target with 422 carrying the partial solution, and the integration suite
  `apps/website/api_v2/tests/fire_mission_solution.rs`; over HTTP, the mortar calculator in
  `apps/website/frontend/src/v2/pages/field_tools/mortar/`.
- Rules: an unknown or padded weapon name is refused, never replaced by another tube, and wins over
  out of range (`unknown_weapon_is_refused_not_substituted`, `unknown_weapon_beats_out_of_range`
  in `tests/mortar_fire_solution.rs`); each weapon's numbers are pinned
  (`per_weapon_solutions_are_pinned`); out of range keeps the partial solution
  (`out_of_range_reports_out_of_range_with_the_partial_solution`); `M821 81mm` answers under its
  own name (`aliases_keep_their_own_name`).
