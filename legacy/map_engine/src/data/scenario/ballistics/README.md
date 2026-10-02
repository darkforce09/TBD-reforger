# Game ballistics

The mortar ballistics behind the field tools, driven by a catalog of the game's own weapon and
shell values: the shell flight model, its inverse (the firing solver, which answers the azimuth and
an elevation per charge from a gun to a target), and the angle and wind conventions they share.
The [API](/documentation/glossary/a_to_f.md#api) serves solutions to the mortar calculator and
records solved fire missions against an [event](/documentation/glossary/a_to_f.md#event).

## Contents

```text
legacy/map_engine/src/data/scenario/ballistics/
├── agreement_cases.rs       `agreement_cases`: a seeded SplitMix64 lattice of battery cases
├── angular_units.rs         `MilsConvention`: degrees, radians and weapon mils; azimuth folding
├── battery.rs               `solve_battery`: every gun solved on its own, with its dispersion
├── calibration/             `evaluate`: a catalog judged against the game's tables and the oracle
├── catalog/                 the game ballistics catalog types, their decode and typed lookups
├── crest_clearance.rs       `crest_clearance`: trajectory clearance over a terrain profile
├── dispersion.rs            `charge_dispersion`: probable errors and the 50 % impact ellipse
├── fire_mission.rs          `solve_fire_mission`: the one assembler of a whole fire-mission solution
├── fire_mission_comparison.rs  `compare_solutions`: the client/server mismatch rule
├── flight_model/            point-mass shell flight: the engine's single-precision step, crossing, apex
├── fuze.rs                  `solve_time_fuze_over_charges`: the lowest charge that fuzes, or the precise refusal
├── mod.rs                   the module tree; re-exports the solver, its result and its error
├── solution_wording.rs      `charge_row_words`, `battery_line_words`: a solution in the words an operator reads
├── solver/                  the firing solver: elevation per charge, refusals, recommended charge
├── tests/                   unit tests of the modules at this level and the cross-module sweeps
└── wind.rs                  `Wind`: speed and "from" direction to the air-velocity vector
```

## How it works

A catalog names each weapon and shell with the game's values; `catalog/` decodes it and resolves
one weapon, shell and charge into flight parameters and a muzzle speed. `flight_model/` flies that
shell from the muzzle until it descends through a target height. `solver/` inverts the flight:
`solve_fire_solution` answers the gun-to-target distance, height difference and azimuth, then for
every charge the high-angle elevation with its time of flight and apex, or a typed refusal
(`too_close`, `out_of_range`, `unreachable`, `did_not_converge`, `time_to_live_exceeded`,
`invalid_input`), and recommends the charge with the fewest rings that solves. Angles are reported
in degrees and in the weapon's mils through `angular_units.rs`; `wind.rs` turns a speed and a
"from" direction into the air velocity the flight subtracts.

`battery.rs` solves several guns onto one target, each on its own through `solve_fire_solution`,
and adds the impact dispersion of each gun's recommended charge. `dispersion.rs` computes that
spread, a documented interpretation of the game's parameters that no engine call verifies: the
launcher's dispersion disc gives a pitch and a transverse angular σ (the transverse one lays the
azimuth off by δ / cos θ), `init_speed_variation` is read as a uniform ± spread in metres per
second, and central finite differences of the impact at the charge's wind-corrected aim carry
them to range and deflection probable errors and the 50 % ellipse. Those differences fly the
engine's scheme in double precision (`fly_to_height_double_precision`) with a 1e-5 rad angle
step, so single-precision rounding never enters a derivative; that flight lands within 0.02 m
of the engine's single-precision one (`tests/dispersion.rs`). Its `verified_in_engine` is always
`false`, and a read of a dispersion claiming `true` fails. `fuze.rs` solves every charge
onto the burst point, the target raised by the burst height, and reports the time of flight as
the fuze time: `solve_time_fuze` for one given charge, `solve_time_fuze_over_charges` for the
charge with the fewest rings whose time lies inside the shell's fuze window, so a burst the
ground-impact charge cannot reach is fuzed on a stronger one. A refused setting carries no time
and names its cause: `outside_fuze_window` when a charge reaches the burst point but none inside
the window, else the solver's cause at the lowest charge (`above_apex`, `beyond_range`,
`inside_minimum_range`, `did_not_converge`, `time_to_live_exceeded`, `invalid_input`).
`crest_clearance.rs` re-flies a
solved charge and measures how far it passes above a terrain profile sampled along the
gun-to-target line: the smallest clearance, where it lies, and the first sample that blocks.
`fire_mission.rs` is the one assembler both the API and the mortar calculator call, so they
produce the same bytes from the same inputs: `solve_fire_mission` takes the input fields of a
fire-mission save (catalog id and version, weapon, shell, an optional operator charge, target,
guns, wind, burst height) and an optional terrain profile under the lead gun's line, solves the
battery, then adds the lead gun's dispersion, time-fuze setting with the burst-point aim the gun
lays, and crest clearance at the fired charge: the operator's charge when given (which also
pins the fuze), else the fuze's burst charge when the fuze sets, else the lead gun's recommended
one; the solution is stamped with the catalog and `SOLVER_REVISION`. That revision is bumped with any change
that can alter a solved number. `fire_mission_comparison.rs` holds the mismatch rule the API
applies to a client solution: equal provenance, gun count, ring lists, recommended charges and
refusals; aim azimuths (the short way round) and elevations within 1 weapon mil; times of flight
and the fuze time within 0.1 s; the fuze's burst aim compared like a charge row.
`agreement_cases.rs` draws a deterministic lattice of battery cases (every catalog shell, targets
across the shell's whole reach, heights, wind, one to three guns) from a seed, so the native and
wasm32 builds can solve the same cases and compare the bits. It also holds the one mapping both
halves use: `fire_mission_inputs` (a case as fire-mission inputs, every height `manual`, the wind
always present), `lead_summary` (the lead gun's recommended rings and time of flight) and
`case_bit_patterns` / `f64_bit_patterns` (every `f64` of `{"inputs", "solution"}` by JSON
pointer, as the 16 hexadecimal digits of its IEEE 754 bits).

## Boundaries

- Depends on: `serde`, `serde_json` and `thiserror`; the flight model and `wind.rs` also on `libm`, whose
  transcendentals give native and wasm32 builds the same bits.
- Used by: `apps/api/src/operations/services/fire_mission_resolve.rs`, which
  re-solves every `POST /api/v1/fire-missions` save through `fire_mission::solve_fire_mission` and
  checks the client's solution with `fire_mission_comparison::compare_solutions` (pinned by
  `apps/api/tests/game_ballistics_fire_missions.rs`); the mortar calculator in
  `apps/frontend/src/v2/pages/field_tools/mortar/`; the agreement bench
  `apps/frontend/src/v2/apps/debug/ballistics_agreement/` and the agreement gate
  `tools/developer_tools/src/browser_testing/ballistics_agreement/`, which both draw, map and
  walk the cases through `agreement_cases.rs`.
- Rules: each module's rules and the tests that pin them are listed in its own README
  ([`solver/`](solver/README.md), [`flight_model/`](flight_model/README.md),
  [`catalog/`](catalog/README.md), [`calibration/`](calibration/README.md)); the angle units and the wind convention are pinned in
  `tests/angular_units.rs` and `tests/wind.rs`; the battery, the crest clearance and the
  agreement lattice with its case-to-inputs mapping, lead summary and bit walk in
  `tests/battery.rs`, `tests/crest_clearance.rs` and `tests/agreement_cases.rs`; the dispersion's symmetry, finite-difference stability and the
  double-precision flight it uses within 0.02 m of the engine flight in `tests/dispersion.rs`;
  the assembled solution's schema parity, determinism and fired
  charge in `tests/fire_mission.rs`, the mismatch rule in `tests/fire_mission_comparison.rs`.
- Cross-module sweeps over the committed vanilla catalog, registered in `mod.rs` as the
  `tests_*` modules: rotating the gun, the target and the wind rotates the aim and keeps every
  charge row, a mirrored crosswind mirrors the deflection correction, and a higher target
  lowers the elevation and shortens the flight (`tests/symmetry.rs`; the exact quantities to
  1e-9, the engine's `f32` flight to a hundredth of a mil and the wind aim to its accepted
  miss); 10,000 seeded clean and corrupted requests never panic and answer typed refusals
  (`tests/bounded_failure.rs`); every oracle simulation sample with a target height or a wind
  inverts to the oracle's launch elevation, aim azimuth and time of flight within 1 mil and
  0.1 s on the high-angle branch, and to the steeper twin on the low-angle branch
  (`tests/oracle_elevation_and_wind.rs`); every weapon and shell solves a whole fire mission
  whose recommended charges land on the target when flown again (`tests/end_to_end.rs`).

## Related documentation

- [Game ballistics documentation](/documentation/legacy/map_engine/data/scenario/ballistics/README.md) —
  the feature documentation of the flight model, the solver, the calibration and the assembled
  fire-mission solution.
