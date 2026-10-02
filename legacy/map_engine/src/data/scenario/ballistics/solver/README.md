# Firing solver

The inverse of the shell flight model: from a gun, a target and a weapon and shell of a
ballistics catalog, the geometric azimuth and, for every charge the shell accepts, the aim azimuth
and high-angle elevation that land the shell on the target through the wind, with the deflection
and range corrections, the time of flight and the apex. A charge that cannot land
there gets a typed refusal instead, and the charge with the fewest rings that solves is the
recommended one.

## Contents

```text
legacy/map_engine/src/data/scenario/ballistics/solver/
├── mod.rs                   the module tree and its re-exports
├── elevation_search.rs      golden-section maximum and Brent root search with an iteration cap
├── charge_selection.rs      one charge's elevation, `ChargeSolution`, `SolutionRefusal`, the recommendation
├── wind_corrected_aim.rs    the aim-point iteration that lets the wind carry the shell onto the target
├── fire_solution.rs         `solve_fire_solution`: request validation, bearing, every charge
└── tests/                   unit tests: vacuum limit, refusals, iteration bound, conventions, wind aim, the wind aim at the elevation limits
```

## How it works

`solve_fire_solution(catalog, request)` looks the weapon and shell up, validates the gun and
target positions, the wind, the weapon's elevation limits and the shell's flight constants, and
computes the horizontal distance `hypot(Δx, Δy)`, the height difference and the azimuth
`atan2(Δx, Δy)` clockwise from north. Each charge is then solved on its own:

1. Fly at the maximum elevation. When that flight outlives the shell's lifetime, bisect for the
   highest elevation that still lands in time and use it as the top of the bracket; when even the
   lowest elevation lands late, or the capped top still overshoots, the charge is
   `time_to_live_exceeded`. Any other failed flight refuses the charge: `unreachable` when the
   target is above its apex, `invalid_input` for a non-physical value. A range at the maximum
   elevation beyond the distance is `too_close`.
2. Find the maximum-range elevation `θ*` by 40 golden-section iterations between the lowest
   elevation and the top of the bracket. A maximum range short of the distance is `out_of_range`.
3. Find the root of `range(θ) - distance` on `[max(θ*, el_min), top]`, the high-angle branch,
   by Brent's method with a bisection fallback: at most 60 iterations, a bracket under 1e-9 rad,
   else `did_not_converge`.

Range is the downrange distance along the aim azimuth where the shell descends through the target
height, flown in the wind. In calm air the aim is the gun-to-target line and the charge's row is
the geometric solution bit for bit. When the wind blows, `wind_corrected_aim` iterates the aim
point: steps 1–3 run along the line to the aim point, the impact is compared with the target, and
the aim point moves by the miss (aim = target minus the wind's drift), at most 20 times until the
miss is under 0.01 m, else `did_not_converge`. An aim line refused by a range bound (`too_close`,
`out_of_range`) does not end the search: a crosswind target at an elevation limit is often reachable
only from an aim point upwind of it, so the next aim point is the target minus the drift of the
bracket-end flight nearest the distance (the flight at the top of the bracket, or at the
maximum-range elevation). The charge keeps that range bound only when this aim point stays within
0.01 m of the refused one, or when the last iteration is refused. Each solved row reports the aim azimuth (degrees and
weapon mils), the deflection correction (aim minus geometric azimuth, weapon mils, positive to the
right) and the range correction (aim-point distance minus target distance, metres); the head- or
tailwind share of the drift is already absorbed by the in-wind elevation. Angles are reported in
degrees and in the weapon's mils (`mils_per_circle`, 6,400 or 6,000). Request-wide faults are a
`FireSolutionError`; charge-specific ones are that charge's refusal; nothing panics.

## Boundaries

- Depends on: the sibling `catalog`, `flight_model`, `wind` and `angular_units` modules; `libm`,
  `serde` and `thiserror`.
- Used by: the battery, dispersion, fuze and calibration computations, and through them the
  fire-mission assembler that the API's fire-mission save and the mortar calculator call; both
  build on `solve_fire_solution` and `solve_charge_elevation`.
- Rules: in vacuum the solved high-angle elevation lands on the analytic range, and its time of
  flight and apex match the analytic flight, within the engine step's chord-sag bound (`g·Δt²/8`
  over the descent slope) plus one `f32` unit roundoff per step, a tenth of one step's ground
  travel at most; the bracket ends refuse as `too_close`, `out_of_range` and `unreachable`; a
  step function over a huge bracket stops at the 60-iteration cap as `did_not_converge`, and the
  root search closes its bracket inside that cap on the single-precision flight; the recommended ring is the lowest that solves; NaN,
  infinite and degenerate inputs are refused; cardinal azimuths and elevations follow the 6,400
  and 6,000 conventions; a crosswind aims upwind and the forward-flown aim lands within 0.05 m of
  the target under cross, head, tail and quartering winds with height differences; calm air keeps
  the geometric aim bit for bit; a crosswind target whose geometric aim line is refused at the
  lowest or highest elevation solves onto the target through the corrected aim
  (`tests/wind_corrected_aim.rs`), while one no aim point reaches keeps `out_of_range` or
  `too_close`; a maximum elevation that outlives the lifetime leaves the lower
  bracket searchable; the rest in `tests/solver.rs`.
