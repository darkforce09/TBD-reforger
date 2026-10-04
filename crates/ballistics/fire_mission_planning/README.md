# Fire mission planning

The `fire_mission_planning` crate: the one fire-mission assembler. A fire mission's inputs
(catalog id and version, weapon, shell, an optional operator charge, target, guns, wind, burst
height and an optional terrain profile) against the pinned catalog become the whole solution:
every gun of the battery solved onto the target, the lead gun's dispersion, time-fuze setting and
crest clearance, stamped with the catalog and the solver revision. The API and the mortar
calculator call the same function, so they produce the same bytes.

## Contents

```text
crates/ballistics/fire_mission_planning/
├── Cargo.toml  the package: `ballistics_model`, `ballistics_solver`, `serde`, `thiserror`, `libm`; layout tier 3
└── src/        battery, fuze, the assembler, the comparison rule, the solution wording, the error, the prelude and the tests
```

## How it works

`battery::solve_battery` solves each gun on its own through `solve_fire_solution` and adds the
dispersion of each gun's recommended charge. `fuze` solves every charge onto the burst point, the
target raised by the burst height, and reports the time of flight as the fuze time;
`solve_time_fuze_over_charges` picks the charge with the fewest rings whose time lies inside the
shell's fuze window, and a refused setting names its cause (`outside_fuze_window`, or the solver's
cause at the lowest charge). `fire_mission::solve_fire_mission` assembles the battery, then the
lead gun's dispersion, fuze and crest clearance at the fired charge: the operator's charge when
given, else the fuze's burst charge when the fuze sets, else the recommended one.
`SOLVER_REVISION` is bumped with any change that can alter a solved number.
`fire_mission_comparison::compare_solutions` is the mismatch rule the API applies to a client
solution: equal provenance, gun count, ring lists, recommended charges and refusals; aim azimuths
and elevations within 1 weapon mil; times within 0.1 s. `solution_wording` holds the words every
surface shows a solution in.

## Getting started

Run from the repository root:

```bash
cargo test -p fire_mission_planning   # battery, fuze, assembler, comparison, wording, end to end
```

## Configuration

None: no features and no environment variables.

## Public surface

- `solve_fire_mission`, `FireMissionInputs`, `FireMissionSolution`, `FireMissionRefusal`,
  `SOLVER_REVISION`; `fire_mission` also holds `FireMissionPoint`, `FireMissionGunPosition`,
  `FireMissionWind`, `HeightSource`, `FireMissionFuze`, `FuzeBurstAim`.
- `battery`: `solve_battery`, `BatteryRequest`, `BatteryGun`, `GunFireSolution`, `BatteryError`.
- `fuze`: `solve_time_fuze`, `solve_time_fuze_over_charges`, `fuze_setting`, `FuzeSetting`,
  `FuzeRefusal`, `FuzeError`.
- `compare_solutions` with `SolutionComparison` and `SolutionMismatch`; `solution_wording`;
  `Error` and `Result`; `prelude`.

## Boundaries

- Depends on: `ballistics_model` and `ballistics_solver`; `serde`, `thiserror` and `libm`. Its
  tests also use `jsonschema` against `contracts/definitions/fire-mission.schema.json`.
- Used by: `crates/api/api_operations/src/services/fire_mission_resolve.rs`, which re-solves every
  `POST /api/v1/fire-missions` save and checks the client's solution (pinned by
  `apps/api/tests/game_ballistics_fire_missions.rs`); the mortar calculator in
  `crates/frontend/pages/field_tools_pages/src/mortar/`; `ballistics_agreement_cases`; the developer
  tools' agreement and offline mortar gates.
- Rules: the assembled solution conforms to the fire-mission schema, is deterministic and fires
  the documented charge (`src/tests/fire_mission.rs`); the mismatch rule is pinned in
  `src/tests/fire_mission_comparison.rs`, the battery in `src/tests/battery.rs`, the fuze in
  `src/tests/fuze.rs`; every weapon and shell of the vanilla catalog solves a whole fire mission
  whose recommended charges land on the target when flown again (`src/tests/end_to_end.rs`).

## Related documentation

- [Ballistics crates](/crates/ballistics/README.md) — the category and its tiers.
- [Game ballistics engine](/documentation/crates/ballistics/game_ballistics_engine.md) — the
  assembled solution and its consumers.
