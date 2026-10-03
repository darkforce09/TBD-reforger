# Ballistics solver

The `ballistics_solver` crate: the inverse of the shell flight model. From a gun, a target and a
weapon and shell of a catalog it answers the geometric azimuth and, for every charge, the aim
azimuth and high-angle elevation that land the shell on the target through the wind, or a typed
refusal; it also measures a solved charge's clearance over a terrain profile and its impact
dispersion.

## Contents

```text
crates/ballistics/ballistics_solver/
├── Cargo.toml  the package: `ballistics_model`, `serde`, `thiserror`, `libm`; layout tier 2
└── src/        the per-charge solve and its searches, the wind aim, crest clearance, dispersion, the error and the prelude
```

## How it works

`solve_fire_solution` answers the gun-to-target distance, height difference and azimuth, then for
every charge the high-angle elevation with its time of flight and apex, or a `SolutionRefusal`
(`too_close`, `out_of_range`, `unreachable`, `did_not_converge`, `time_to_live_exceeded`,
`invalid_input`), and recommends the charge with the fewest rings that solves. With wind the aim
point is iterated until the impact lies within tolerance of the target (`wind_corrected_aim`).
`crest_clearance` re-flies a solved charge over a terrain profile sampled along the line of fire:
the smallest clearance, where it lies, and the first sample that blocks. `dispersion` derives the
probable errors and the 50 % ellipse of a solved charge, a documented interpretation of the
game's dispersion fields that no engine call verifies (`verified_in_engine` is always `false`):
central finite differences of the impact, flown in double precision, carry the launcher's
dispersion disc and the shell's speed variation to range and deflection.

## Getting started

Run from the repository root:

```bash
cargo test -p ballistics_solver   # the solver, wind aim, clearance, dispersion and the two sweeps
```

## Configuration

None: no features and no environment variables.

## Public surface

- `solve_fire_solution`, `FireSolutionRequest` (borrowing a `WeaponId` and a `ShellId`),
  `FireSolution`, `FireSolutionError`, `MapPosition`.
- `charge_selection`: `solve_charge_elevation`, `ChargeProblem`, `ChargeElevation`,
  `ChargeSolution`, `SolutionRefusal`, `recommended_rings`.
- `elevation_search`: `GOLDEN_SECTION_ITERATIONS`, `RootSearchLimits`, `find_bracketed_root`,
  `maximise_by_golden_section`, the searches the calibration reuses.
- `wind_corrected_aim`, `crest_clearance` (`TerrainProfile`, `TerrainSample`, `CrestClearance`),
  `dispersion` (`charge_dispersion`, `ImpactDispersion`); `Error` and `Result`; `prelude`.

## Boundaries

- Depends on: `ballistics_model`; `serde`, `thiserror` and `libm`. Its bounded-failure sweep draws
  with `deterministic_random` (dev-dependency).
- Used by: `fire_mission_planning` (battery, fuze, assembler), `ballistics_calibration` (the
  elevation searches) and `ballistics_agreement_cases`; the mortar calculator's solution rows and
  terrain profile.
- Rules: the per-module rules are in [the source README](src/README.md); rotating the gun, the
  target and the wind rotates the aim and keeps every charge row, a mirrored crosswind mirrors the
  deflection, and a higher target lowers the elevation (`src/tests/symmetry.rs`); 10,000 seeded
  clean and corrupted requests never panic and answer typed refusals
  (`src/tests/bounded_failure.rs`).

## Related documentation

- [Ballistics crates](/crates/ballistics/README.md) — the category and its tiers.
- [Game ballistics engine](/documentation/crates/ballistics/game_ballistics_engine.md) — the
  solver's brackets, refusals and wind aim in the whole walk.
