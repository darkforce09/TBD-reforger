**Status:** live

# Game ballistics engine

The mortar ballistics of the map engine: from a catalog of the game's own weapon and shell values,
it flies a shell the way the Arma Reforger engine does, inverts that flight into firing solutions
for a battery of guns, and judges whether a catalog reproduces the game before the
[API](/documentation/glossary/a_to_f.md#api) accepts it. The mortar calculator page and the
API's fire-mission save run the same code, natively and in the browser, with the same bits.

## Where it lives

- Code: [`legacy/map_engine/src/data/scenario/ballistics/`](/legacy/map_engine/src/data/scenario/ballistics/README.md),
  with [`flight_model/`](/legacy/map_engine/src/data/scenario/ballistics/flight_model/README.md),
  [`solver/`](/legacy/map_engine/src/data/scenario/ballistics/solver/README.md),
  [`catalog/`](/legacy/map_engine/src/data/scenario/ballistics/catalog/README.md) and
  [`calibration/`](/legacy/map_engine/src/data/scenario/ballistics/calibration/README.md);
  `fire_mission.rs` is the assembler every consumer calls.
- Entry: `solve_fire_mission` in `fire_mission.rs` for a whole fire mission; `evaluate` in
  `calibration/mod.rs` for a catalog upload. Both compile under the crate's default `scenario`
  feature, which the API links and the frontend builds for wasm32.
- Related:
  - the [mortar calculator page](/documentation/apps/frontend/pages/field_tools/mortar/mortar_calculator_page.md),
    which solves on the device;
  - the [ballistics catalogs page](/documentation/apps/frontend/pages/administration/ballistics_catalogs/ballistics_catalogs_page.md),
    where catalogs are uploaded and judged;
  - the [ballistics agreement bench](/documentation/apps/frontend/apps/debug/ballistics_agreement_page.md),
    which proves the browser build solves bit for bit like the native one;
  - the [ballistics oracle](/documentation/mod/tbd-export/Scripts/WorkbenchGame/BallisticsOracle/ballistics_oracle.md),
    the engine measurements behind the calibration.

## Behaviour

### From catalog to solution

1. A catalog (`contracts/definitions/ballistics-catalog.schema.json`) names each weapon (mils
   convention, elevation limits, muzzle factor, dispersion disc, the shells it fires) and each
   shell (initial speed and its variation, mass, air drag, wind influence, lifetime, charges,
   optional time fuze). `catalog/` decodes it and resolves a weapon, shell and charge into flight
   parameters and a muzzle speed, initial speed × charge coefficient × muzzle factor.
2. `flight_model/` flies the shell from the muzzle with the engine's own step: every 1/30 s,
   gravity first, then quadratic drag on the air-relative velocity, then the position by the mean
   of the old and new velocities, all in `f32` like the engine. The flight ends where the step
   polyline falls through the target height. The side air-drag scale the game declares has no
   effect in the engine and none here.
3. `solver/` inverts the flight per charge: it brackets the elevations that land within the
   shell's lifetime, finds the maximum-range elevation, and solves the high-angle elevation that
   reaches the target's distance, or refuses with a typed reason (`too_close`, `out_of_range`,
   `unreachable`, `did_not_converge`, `time_to_live_exceeded`, `invalid_input`). Under wind it
   iterates an aim point until the drift carries the shell onto the target, reporting the aim
   azimuth and the deflection and range corrections. The recommended charge is the one with the
   fewest rings that solves.
4. `battery.rs` solves each gun on its own onto the one target. `dispersion.rs` derives the
   probable-error ellipse from the game's dispersion parameters, a documented interpretation no
   engine call verifies. `fuze.rs` solves a time-fuzed shell onto the burst point above the
   target at every charge and sets the fuze on the charge with the fewest rings whose time of
   flight lies inside the fuze window, so a burst the ground-impact charge cannot reach is fuzed
   on a stronger one; an operator's charge pins the fuze to that charge. A refused setting names
   its cause: `outside_fuze_window`, or, when no charge reaches the burst point, the solver's
   cause at the lowest charge (`above_apex`, `beyond_range`, `inside_minimum_range`,
   `did_not_converge`, `time_to_live_exceeded`, `invalid_input`). `crest_clearance.rs` re-flies
   the fired charge over a terrain profile and reports the smallest clearance and the first
   blocking sample.
5. `solve_fire_mission` assembles the whole `FireMissionSolution` from the fields of a
   fire-mission save and an optional terrain profile, stamped with the catalog id and version and
   `SOLVER_REVISION`. `compare_solutions` is the rule the API applies to the page's copy: 1 weapon
   mil on angles, 0.1 s on times, equality on everything discrete.

Heights are inputs: the solver never reads terrain. The page samples the elevation model; the API
takes the heights the save carries.

### Calibration of a catalog

`evaluate(catalog, bundle)` flies every charge of every shell against the calibration bundle
(`contracts/definitions/ballistics-calibration.schema.json`): the game's native ballistic
tables and wind tables at the charge coefficients, and the engine oracle's forward-angle and
simulation samples. Provenance is judged first (catalog digest, game build, generation, gravity,
coverage of every shell and charge); then every case against fixed tolerances of 1 mil (6400
convention) and 0.1 s. A report with any failure refuses the catalog. The committed vanilla pair
passes all 7,865 cases; forward samples between two native rows are counted as the engine's table
interpolation and not judged. The criterion table and the evidence are in the
[design note](/documentation/apps/api/verification_evidence/game_ballistics.md#calibration-criterion).

### Same bits everywhere

Sine and cosine come from `libm`; the other operations are IEEE and never fused. The native API
and the browser's wasm32 build therefore produce identical bits, which the agreement bench checks
over a seeded lattice of cases (`agreement_cases.rs`) with `cargo xtask mk ballistics-wasm-agreement`.

## Data

- Catalogs: stored immutable by the API (`POST /api/v1/ballistics-catalogs`, administrator,
  multipart `catalog` and `calibration` parts), read publicly with
  `GET /api/v1/ballistics-catalogs` and `GET /api/v1/ballistics-catalogs/{catalogId}/versions/{version}`.
  The upload runs `evaluate` on the API's blocking pool and stores the version and its audit line
  in one transaction, or answers 422 with the report.
- Fire missions: `POST /api/v1/fire-missions` re-solves the save with `solve_fire_mission` and
  refuses a client solution `compare_solutions` does not tolerate (422 `solution_mismatch`); the
  stored solution is the server's.
- Fixtures: the committed vanilla catalog
  `contracts/catalogs/ballistics/vanilla_mortars.v1.catalog.json` and its bundle in
  `contracts/fixtures/ballistics/vanilla_mortars.v1/`, written by
  `cargo xtask ballistics trim-export` from the equipment export and the oracle output, and read by
  the module's tests through `include_str!`.

## Design

The model is engine-faithful rather than textbook: the integration scheme, the precision and the
constants are the engine's, identified from 4,185 oracle simulations (largest point-of-fall
difference 0.0078 m), and the game's tables are calibration fixtures, never lookup data. The
[design note](/documentation/apps/api/verification_evidence/game_ballistics.md#flight-model)
holds the identification table and the settled model questions (native column 1, the wind-table
values, the side air-drag scale, the unit of the speed variation).

## Open work

- [T-940.10 — Mortar ballistics crate for API and offline frontend](/documentation/tickets/specs/t940_website_platform.md)
  (ready, [plan](/documentation/tickets/plans/t-940_10_plan.md)): built by milestone B as this
  module; the registry closes it with the milestone.

## Decisions

- Physics that matches the game engine: a catalog of any mortar or artillery piece solves without
  code changes, and the game's tables only judge the model.
- The engine's `f32` step, not a more accurate integrator: the answer must be where the game's
  shell lands, and the engine's own rounding is part of that.
- One assembler for the page and the API: a saved fire mission is the server's solution, and a
  client that differs is refused rather than trusted.
- Dispersion is flown on the `f64` copy of the engine step: a finite difference through `f32`
  rounding would measure the rounding, not the spread.
- Tolerances are fixed: a case that misses 1 mil or 0.1 s is a finding about the model, never a
  reason to widen the criterion.
