# Shell flight model

The physics under the game-ballistics solver: a point-mass shell with quadratic air drag and a
constant wind, flown from the muzzle until it falls through a target height with the game
engine's own simulation step. It answers the time
of flight, the impact point, the apex and, on request, the whole sampled path, and knows nothing
of catalogs, weapons or shells: callers hand it plain numbers.

## Contents

```text
crates/ballistics/ballistics_model/src/flight_model/
├── mod.rs         `FlightParameters`, `Launch`, `FlightError`, input validation, the step constants
├── integrator.rs  the engine's simulation step: gravity, then drag, then the mean-velocity position
├── crossing.rs    linear interpolation along a step; the target-height crossing
├── precision.rs   `FlightFloat`: the arithmetic of `f32` (the engine's) and `f64` (the proofs')
├── trajectory.rs  `fly_to_height`, `fly_to_height_double_precision`, `FlightOutcome`, `FlightSample`, `PathRecording`
└── tests/         unit tests: vacuum bounds, drag and step convergence, energy, crossing, wind, oracle
```

## How it works

Positions are map-frame metres with the muzzle at the origin, x east, y north, z up; the azimuth
runs clockwise from north and the elevation up from the horizontal, both in radians. The
acceleration is `a = -g·ẑ - k·|v_rel|·v_rel`, with `k = air_drag / mass_kg` and
`v_rel = v - wind_influence_multiplier · w`, where `w` is the air velocity of
[`../wind.rs`](../wind.rs). Drag is isotropic: a shell's side air-drag scale has no term.

`fly_to_height(parameters, launch, wind, target_height_m, recording)` validates every input,
then advances the shell the way the game engine does, in the engine's single precision, at the
engine's fixed step
`DEFAULT_INTEGRATION_STEP_S` (1/30 s): gravity first (`u = v - g·Δt·ẑ`), then drag on the
air-relative velocity (`v' = u - k·|u - w|·(u - w)·Δt`), then the position by the mean of the old
and new velocities (`x' = x + (v + v')/2·Δt`). The flight ends where the polyline through the step
points falls through the target height, placed by linear interpolation along that step as the
engine places it; the apex is the highest step point. The time of flight is the interpolated
crossing time.

The scheme was identified against the engine oracle of game build 1.8.0.13 (generation
6A6F008DC5395616): 4,185 simulation samples with exact inputs, over explicit, semi-implicit and
mean-velocity Euler, velocity Verlet, midpoint, Heun and classical Runge-Kutta, three drag forms,
steps from 1/30 s to 1/1000 s, single and double precision, and interpolated or step-end
crossings. The oracle's times of flight are all the end of a step at a multiple of 1/30 s.

| Scheme | Largest downrange difference | Root mean square | Largest crossrange difference |
|---|---|---|---|
| this step, 1/30 s, single precision (the model) | 0.0078 m | 0.001 m | 0.0005 m |
| this step, 1/30 s, double precision | 0.016 m | 0.003 m | 0.0005 m |
| this step, 1/50 s | 1.15 m | 0.17 m | 0.09 m |
| drag before gravity, 1/60 s, step-end crossing | 1.20 m | 0.35 m | 0.34 m |
| exponential drag factor, 1/50 s | 1.29 m | 0.57 m | 0.38 m |
| classical Runge-Kutta, 1/30 s | 2.87 m | 0.42 m | 0.22 m |

The model computes the state and every step in `f32`, as the engine does, with the constants as
the engine holds them: gravity rounded to `f32` (9.81 becomes 9.8100004196167), `k` divided in
`f32`, the step `1/30` in `f32`, the air velocity scaled in `f32`, and the muzzle velocity from
`libm`'s `sinf` and `cosf`. It widens to `f64` only in its results. Over all 4,185 simulation
samples the point of fall lies within 0.0078 m downrange and 0.0005 m crossrange of the
engine's; over all 476 native ballistic-table rows the range lies within 0.0057 m and the time
of flight within 0.0008 s (pinned by `../../../ballistics_calibration/src/tests/committed_bundle.rs`).

The same scheme also compiles in `f64` (`precision.rs`): the scheme's own properties are proven
on that instantiation, free of single-precision rounding. In vacuum its step points lie on the
exact parabola, so the analytic range, time of flight and apex hold within the chord sag of one
step, `g·Δt²/8` (1.4 mm), divided by the slope of the step at the crossing. Single-precision
rounding makes the production range a step function of the elevation at the scale of one `f32`
rounding of the angle; the solver's root search still closes its 1e-9 rad bracket within its
iteration cap. Derivatives of the point of fall do not go through that step function:
`fly_to_height_double_precision` flies the same step, step size and constants in `f64`, and the
dispersion's finite differences (1e-5 rad) use it.

It refuses with a typed `FlightError`, never a panic:

- `InvalidInput` or `InvalidWind` for NaN, infinities, a non-positive mass, speed, gravity, step
  or lifetime, or a negative drag, multiplier or wind speed;
- `TooManyIntegrationSteps` when `time_to_live_s / integration_step_s` exceeds
  `MAX_INTEGRATION_STEPS`;
- `TargetAboveApex` as soon as the shell descends below a target it never reached;
- `TimeToLiveExceeded` when the shell is still above the target when its lifetime ends.

Sine and cosine go through `libm` (`sinf` and `cosf` in the flight), `sqrt` and the four
operations are IEEE and never fused, so a native build and the browser's wasm32 build produce
the same bits.

## Boundaries

- Depends on: `libm`, `thiserror` and the sibling `wind` module.
- Used by: the solver, fuze and crest computations of `ballistics_solver` and
  `fire_mission_planning` (`fly_to_height`) and the dispersion (`fly_to_height_double_precision`).
- Rules: on the double-precision scheme, vacuum flight matches the analytic range, time of
  flight and apex at five angles within the chord-sag bound, and its step points lie on the
  parabola; the step is first order (halving it halves the error) on every case of the vanilla
  shell lattice; the crossing lies on its step's chord and the apex is the highest step point.
  The production flight holds the engine's `f32` constants, differs from the double-precision
  scheme by at most 0.02 m and is not bit-identical to it; the M821 at 45° and coefficient 1
  lands at the native row, 427.308 m and 9.417 s, and seven engine-oracle samples are reproduced
  within 0.02 m; all in `tests/flight_model.rs`. The residual over every committed oracle sample
  and native row is pinned in `../../../ballistics_calibration/src/tests/committed_bundle.rs`.
