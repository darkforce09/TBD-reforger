**Status:** live

# Game ballistics: flight model, calibration, catalogs, fire missions and the offline page

Design for the game-ballistics requirements (`game_ballistics_flight_model`,
`game_ballistics_calibration`, `game_ballistics_elevation_wind_dispersion`,
`game_ballistics_battery`, `game_ballistics_wasm_agreement`, `game_ballistics_offline_page`) and the
API requirement `verification_game_ballistics`. It records the chosen semantics and the model the
engine oracle identified; acceptance evidence is the command output recorded in
progress_checkpoint.md. The wire shapes are three schemas under `contracts/definitions/`:
`contracts/definitions/ballistics-catalog.schema.json`,
`contracts/definitions/ballistics-calibration.schema.json` and
`contracts/definitions/fire-mission.schema.json` (version 2). The map-engine module is
`legacy/map_engine/src/data/scenario/ballistics/`, written `ballistics/` below; its feature
doc is the [game ballistics engine](/documentation/legacy/map_engine/data/scenario/ballistics/game_ballistics_engine.md).
The section B entry of remaining_milestones.md links here once the register lands.

## Operator decisions

These are binding; the design below implements them and nothing else.

| # | Subject | Decision |
|---|---|---|
| 1 | Model | Physics that matches the game engine. The game's own tables are calibration fixtures, never lookup data. |
| 2 | Scope | M252 and 2B14, all seven vanilla shells, every charge ring. Data-driven, so modded mortars and artillery arrive as catalogs. The M120 is dropped: legacy rows stay readable but cannot be re-solved. |
| 3 | Tolerance | At most 1 mil of elevation and 0.1 s of time of flight against every native and wind point and every oracle sample. Native and WASM solutions agree within 1 mil. |
| 4 | Fixtures | Trimmed from the equipment export plus a new tbd-export oracle plugin; pinned to game build, export generation and sha256; red on a build mismatch, a stale hash, or a shell or ring without a fixture. Gravity comes from the oracle. The operator runs Workbench (checkpoint W). |
| 5 | Heights | Sampled automatically from the full 2 m DEM raster, with a manual override. The solver takes heights as inputs; sampling belongs to the page. Arland is manual only, with its map disabled (it has no DEM and no tiles). |
| 6 | Wind | Manual speed plus a "from" direction in degrees, constant with height, scaled by the shell's WindInfluenceMultiplier. |
| 7 | Dispersion | A probable-error ellipse from game parameters only, labelled "documented interpretation, not verified in-engine". |
| 8 | Battery | Independent per-gun solutions onto one target. No sheaf, no time on target, no adjust-fire. |
| 9 | API | The flat solve route is retired. Saving stores the inputs, re-solves them and refuses beyond 1 mil (weapon convention) or 0.1 s. Migration 0060 preserves and detaches fire missions whose event is gone, then adds the event foreign key; the list checks viewer access. |
| 10 | Page | A map picker on the Mission Creator renderer through a shared seam; grid references in 6, 8 or 10 figures; shell choice with every valid charge; mils in the weapon's convention plus degrees; illumination fuze time for a burst height; a crest-clearance warning; one save DTO shared with the API, `token_type` always "Bearer"; stale comments in touched files corrected. |
| 11 | Offline | A full PWA through a Rust/WASM service worker; the JavaScript loader holds no logic. The first `/tools/mortar` visit caches the app shell, catalogs, the Everon manifest, DEM, tbd-sat and map tiles z0–6 (about 248 MB) with progress, a quota check and `persist()`; Range requests are answered 206 from the cache; Material Symbols are cached. |
| 12 | Catalogs | Every catalog, vanilla included, comes from the API, uploaded by an admin at `/admin/ballistics-catalogs`; there is no boot import. An upload carries its calibration fixtures; the API runs the model and refuses anything over tolerance. Catalogs are immutable and audited. Catalog reads and the calculator page are public; saving needs sign-in. |
| 13 | In-game | Only the tbd-export oracle. Nothing in tbd-framework. |
| 14 | Register | Several requirements, listed under [Register](#register). |
| 15 | Oracle lattice | The forward-angle oracle samples every 1.5625 mils (one sixteenth of 25 mils), so every native row's elevation is a lattice point; the trim matches every row exactly, with no interpolation and no omission. |
| 16 | Forward samples (R1) | A forward-angle sample is evidence for a native row only. Between rows the engine answers by linear interpolation of its own table, so such a sample is counted (`forward_samples_not_judged`) and never judged. |
| 17 | Precision | The flight model holds its state in `f32`, like the engine, with no tolerance floor added for it. |

Design constraints taken from the plan review:

- The new crate `offline_service_worker` (`apps/offline_service_worker/`) has its
  own crate-direction rule: it may not depend on api, frontend or
  graphics_engine. Its wasm-only code sits behind `cfg(target_arch = "wasm32")` with a
  native no-op `main`, so workspace builds and clippy compile it on the host, and it joins the
  `wasm-ci` lane (fmt, clippy host and wasm32, tests).
- Tile pyramids and tbd-sat are gitignored local build output. The offline gate fails closed
  ("assets missing", non-zero exit) when they are absent and never passes on a smaller set.
- Cached responses keep every header, so cross-origin isolation (COOP same-origin, COEP
  credentialless) survives offline; the gate asserts `crossOriginIsolated === true` offline.
- The catalog upload route sets its own body limit, because a calibration bundle reaches several
  MB; 413 is tested.
- `Cargo.lock` is resolved by cargo, never edited by hand; each manifest hunk has one owner.

## Game facts the model rests on

Read from the game pak and the tbd-export equipment export (game build 1.8.0.13, export generation
`6A6F008DC5395616`, stored gitignored at
`assets/equipment/gameplay/generations/6A6F008DC5395616/export/`).

- `ShellMoveComponent` integrates a = −g·ẑ − (AirDrag / Mass)·|v_rel|·v_rel. Mass is the
  ShellMoveComponent mass, not the RigidBody mass.
- Muzzle speed is InitSpeed × the ring coefficient (`SCR_MortarShellGadgetComponent`
  `m_aChargeRingConfig`: tuples of ring count, coefficient and default flag) × the weapon's muzzle
  coefficient.
- Gravity is `PhysicsWorld.GetGravity` = 9.8100004196167 m/s² (9.81 rounded to `f32`).
- The engine's integration scheme is identified in [Flight model](#flight-model); it reproduces
  every native row within 0.0057 m and 0.0008 s.

| Shell | InitSpeed (m/s) | Mass (kg) | AirDrag | Ring coefficients | Time fuze |
|---|---|---|---|---|---|
| M821 | 66 | 4.06 | 0.000462 | rings 0–4: 1, 1.531, 2.085, 2.541, 2.977 | none |
| M879 | 66 | 4.26 | 0.000469 | rings 0–4: 1, 1.573, 2.082, 2.532, 2.932 | none |
| M819 | 137 | 4.85 | 0.0009139 | rings 1–4: 0.666, 0.959, 1.184, 1.387 | none |
| M853A1 | 152 | 4.0 | 0.001488 | rings 1–4: 0.638, 1, 1.281, 1.596 | 10–40 s, default 24 |
| O-832DU | 76 | 3.10 | 0.000615 | rings 0–4: 1, 1.321, 1.736, 2.087, 2.455 | none |
| D-832DU | 71 | 3.48 | 0.000655 | rings 0–3: 1, 1.339, 1.748, 2.086 | none |
| S-832S | 127 | 3.51 | 0.001836 | rings 1–4: 0.698, 1.111, 1.512, 2.154 | 10–40 s, default 19 |

Every shell shares InitSpeedVariation 3, DispersionMultiplier 1, TimeToLive 60 s,
SideAirDragScale 10 and WindInfluenceMultiplier 1.

| Weapon | Mils per circle | Elevation | Muzzle dispersion | Range card |
|---|---|---|---|---|
| M252 | 6400 | 45–85° | DispersionDiameter 1 m at DispersionRange 48 m | `m_fStandardDispersion` 15 m |
| 2B14 | 6000 (`SCR_MortarInfo m_fMils`) | 45–85° | DispersionDiameter 1 m at DispersionRange 48 m | `m_fStandardDispersion` 15 m |

Vanilla has no 120 mm mortar and no artillery.

The game's native tables hold one table per init-speed coefficient (M821 has 15, from 0.1 to
5.0); only the tables at a charge coefficient enter the bundle, since only those are fired. A row
stores no elevation: rows follow a lattice from 1600 mils down in steps of 100, 50, 25 or 12.5
mils, and the apex row comes last. Column 0 is range, column 2 is time of flight, and column 1 is
range · tan(elevation) (see [Settled model questions](#settled-model-questions)). The wind tables
are 90 rows at 1° of [θ rad, range, apex] plus `m_aValues`, decoded below.

## Units and frames

- Map frame in metres: x east, y north, z up. Azimuth is clockwise from north.
- Angles are radians inside the model. A weapon's mils are angle × mils_per_circle / 2π; outputs
  use the weapon's convention, and every calibration tolerance is written in mil₆₄₀₀
  (1 mil₆₄₀₀ = 2π/6400 rad ≈ 0.982 mrad).
- Wind is given as a speed and the meteorological "from" direction in degrees. The air velocity is
  w = −speed·(sin from, cos from, 0). The oracle's `wind_vector_world` is the same vector in
  Enfusion's x-east, y-up, z-north frame: −speed·(sin from, 0, cos from).
- Heights are metres above the map datum; Δh is target height minus gun height.

## Flight model

`ballistics/flight_model/` with `ballistics/wind.rs`. It takes its own `FlightParameters` and knows
nothing of catalogs.

- Acceleration a = −g·ẑ − k·|v_rel|·v_rel with k = AirDrag / Mass, v_rel = v − m_w·w and
  m_w = WindInfluenceMultiplier. Drag is isotropic: SideAirDragScale has no term.
- Muzzle speed v0 = InitSpeed × ring coefficient × weapon muzzle coefficient.
- The engine's own step, at the fixed `DEFAULT_INTEGRATION_STEP_S` = 1/30 s: gravity first
  (u = v − g·Δt·ẑ), then linear drag on the air-relative velocity
  (v′ = u − k·|u − w|·(u − w)·Δt), then the position by the mean of the old and new velocities
  (x′ = x + (v + v′)/2·Δt).
- State and every step are computed in `f32` with the constants as the engine holds them (gravity,
  k, the step and the scaled air velocity each rounded to `f32`; the muzzle velocity from `libm`'s
  `sinf` and `cosf`). Results widen to `f64`.
- The impact is where the polyline through the step points falls through the target height,
  placed by linear interpolation along that step (the chord crossing the engine uses); the time
  of flight is the interpolated crossing time. The apex is the highest step point.
- Refusals are typed, never a panic: `InvalidInput`, `InvalidWind`, `TooManyIntegrationSteps`,
  `TargetAboveApex`, and `TimeToLiveExceeded` when the shell is still above the target when its
  TimeToLive ends.
- sin and cos come from the `libm` crate on every target; +, −, ×, / and sqrt are IEEE and never
  fused, so native and WASM results are bit-identical.
- The same scheme compiles in `f64`: `fly_to_height_double_precision` serves the dispersion's
  derivatives and the proofs of the scheme's own properties (vacuum step points on the exact
  parabola, the analytic range within the chord sag g·Δt²/8 ≈ 1.4 mm over the crossing slope,
  first-order convergence in the step).

### How the scheme was identified

The engine oracle's 4,185 simulation samples, with exact inputs, identify the scheme; a smooth
integrator such as classical RK4 misses them by metres. Their times of flight all end on a
multiple of 1/30 s, and of explicit, semi-implicit and mean-velocity Euler, velocity Verlet,
midpoint, Heun and RK4, over three drag forms, steps from 1/30 s to 1/1000 s, both precisions and
interpolated or step-end crossings, only this scheme reproduces them.

| Scheme | Largest downrange difference | Root mean square | Largest crossrange difference |
|---|---|---|---|
| this step, 1/30 s, single precision (the model) | 0.0078 m | 0.001 m | 0.0005 m |
| this step, 1/30 s, double precision | 0.016 m | 0.003 m | 0.0005 m |
| this step, 1/50 s | 1.15 m | 0.17 m | 0.09 m |
| drag before gravity, 1/60 s, step-end crossing | 1.20 m | 0.35 m | 0.34 m |
| exponential drag factor, 1/50 s | 1.29 m | 0.57 m | 0.38 m |
| classical Runge-Kutta, 1/30 s | 2.87 m | 0.42 m | 0.22 m |

Over all 476 native rows the model's range lies within 0.0057 m and its time of flight within
0.0008 s of the table. Because the state is `f32`, the range is a step function of the elevation at
the scale of one `f32` rounding of the angle; the solver's 1e-9 rad bracket still closes within its
iteration cap on every charge, and exact symmetries (rotating gun, target and wind together) hold
to 0.01 mil and 1e-3 s rather than 1e-9.

## Solver

`ballistics/solver/`, over `ballistics/catalog/` lookups and the flight model.

- Per charge, the top of the bracket is the maximum elevation, or, when that flight outlives the
  shell's TimeToLive, the highest elevation that still lands in time (found by bisection).
- θ* = argmax of range on [el_min, top] by golden-section search (40 iterations).
- Refusals, each typed: `too_close` when range(top) > D; `out_of_range` when range(θ*) < D;
  `unreachable` when the target is above the apex; `did_not_converge` when Brent's method with its
  bisection fallback has not reached |Δθ| < 1e-9 rad within 60 iterations;
  `time_to_live_exceeded`; `invalid_input` for NaN, infinity, a negative mass or a zero speed.
- Only the high-angle branch is solved, on [max(θ*, el_min), top].
- Wind-corrected aim (`solver/wind_corrected_aim.rs`): in calm air the aim is the gun-to-target
  line, bit-identical to the geometric solution. Under wind the aim point moves by the miss, at
  most 20 times until the miss is under 0.01 m. An aim line refused by a range bound does not end
  the search: the next aim point is the target minus the drift of the bracket-end flight, and the
  refusal stands only if the aim point stops moving or the last iteration is refused.
- Every charge is evaluated; the recommended charge is the one with the fewest rings that solves.
- Output per gun: the geometric azimuth; per charge the aim azimuth (degrees and weapon mils), the
  deflection and range corrections, the elevation in degrees and weapon mils, the time of flight,
  the apex, or the refusal; the recommended charge.

## Dispersion, fuze, crest and battery

### Dispersion: a documented interpretation, not verified in-engine

The game exposes dispersion parameters but no engine call returns a dispersion, so nothing here is
checked against the engine. The page labels the ellipse "Interpretation, not verified in-engine".

- Angular spread: a uniform disc of diameter DispersionDiameter × DispersionMultiplier at
  DispersionRange. With disc radius r, the per-axis angular σ is (r / DispersionRange) / 2. The two
  axes are perpendicular to the bore: pitch (an elevation change) and the transverse axis (a
  rotation of the launch direction about the bore normal, which equals an azimuth change of δ / cos
  θ).
- Speed spread: uniform ±InitSpeedVariation in m/s, σ = a / √3.
- Propagation: central finite differences of the impact point at the charge's wind-corrected aim
  (h = 1e-5 rad for angles, 1e-3·v0 for speed) give the Jacobian J; the impact covariance is J·Σ·Jᵀ
  with Σ the diagonal of the three variances. The differences fly the `f64` copy of the engine
  step, so `f32` rounding never enters a derivative; that flight lands within 0.02 m of the `f32`
  one.
- Reported: range and deflection probable errors (0.6745·σ), the 50 % ellipse (semi-axes
  √(2 ln 2) ≈ 1.1774 times the principal σ) and its orientation, and the range card's
  `standard_dispersion_m` beside them. On the high-angle branch only the speed share of the range
  probable error grows with range.

### Time fuze

The fuze solves onto the burst point, the target raised by the burst height, and reports the time
of flight to that descending point as the fuze time, with the burst-point aim the gun lays
(`FuzeSetting.burst_aim`). Without an operator charge, `solve_time_fuze_over_charges` searches the
charges from the lowest ring up and takes the first whose burst-point aim solves with a time inside
the shell's [min_s, max_s]; an operator charge pins the fuze to that charge. When no charge fuzes,
the refusal names the cause (above the apex, beyond range, inside the minimum range, outside the
fuze window, no descending crossing); `default_s` is reported with it.

### Crest clearance

The input is a terrain profile of (downrange m, height m) samples, which the page samples along the
lead gun's line. The trajectory height at each sample is linearly interpolated in downrange
distance. Reported: the minimum clearance, its distance, and the first blocking sample (clearance
below zero).

### Battery and the assembled solution

`guns[]` (one to twelve) each solve independently onto one target, each with its own azimuth,
elevation, charge, time of flight and dispersion. `ballistics/fire_mission.rs` is the one
assembler the API and the page call: `solve_fire_mission` takes the save's input fields and an
optional terrain profile and returns the whole `FireMissionSolution`, stamped with the catalog and
`SOLVER_REVISION` (the constant in `ballistics/fire_mission.rs`, now `game-ballistics-2`, bumped
by any change that can alter a solved number).
`ballistics/fire_mission_comparison.rs` holds the mismatch rule: equal provenance, gun count, ring
lists, recommended charges and refusals; aim azimuths and elevations within 1 weapon mil; times of
flight and the fuze time within 0.1 s.

## Calibration criterion

`ballistics/calibration/` is production code: `evaluate(catalog, bundle)` returns a
`CalibrationReport {cases, failures, forward_samples_not_judged}`. The API upload and the tests
run the same function. Each shell is flown at InitSpeed × coefficient with the catalog's gravity,
mass, drag and wind multiplier.

| Case | Passes when |
|---|---|
| Native-table row | The row's range lies within the model's range over [θ_row ± 1 mil₆₄₀₀], and \|T_model(θ_row) − T_row\| ≤ 0.1 s |
| Forward-angle sample at a native row | As a native-table row, at the sample's elevation; a sample between rows is counted in `forward_samples_not_judged`, never judged (decision 16) |
| Wind-table row | The calm range lies within the model's range over [θ_row ± 1 mil₆₄₀₀]; the crosswind deflection angle within 1 mil of the model's; the range change under a wind along the line of fire within (range + \|change\|)·tan(1 mil) of the model's |
| Simulation oracle sample | The impact error along range and along deflection is each ≤ D·tan(1 mil₆₄₀₀), and \|ΔT\| ≤ 0.1 s; a sample the engine never lands passes only if the model refuses it too |

Altitude-difference samples are carried and not judged. Provenance is judged before any case and
adds failures, never cases:

- the bundle names the catalog's id and version, game build and export generation;
- `bundle.catalog_sha256` equals the sha256 of the catalog bytes;
- the catalog's gravity equals the gravity the oracle run reported;
- no resource GUID carries two digests across the two documents;
- every (shell, charge) of the catalog has a native table and a simulation sample at its
  coefficient, and every table and sample names a catalog shell.

Any failure is listed in the report; a report with failures refuses the catalog. Tolerances are
never widened to make a case pass: a model that misses them is a finding. On the committed vanilla
pair the report holds 7,865 cases and no failure; 414 of the 15,841 forward samples lie at a native
row and are judged, the other 15,427 are interpolation.

## Fixtures and their provenance

The committed catalog is `contracts/catalogs/ballistics/vanilla_mortars.v1.catalog.json`; its
calibration bundle is `contracts/fixtures/ballistics/vanilla_mortars.v1/calibration.json` (31
native tables with 476 rows, 31 wind tables, 21,111 oracle samples: 15,841 forward, 4,185
simulation, 1,085 altitude) beside four `negative/` variants: a row skewed by +5 m, a wrong game
build, a missing shell, and a stale catalog sha256. The fixture folder's README holds every sha256.

Lifecycle:

1. The equipment export of a game build lands, gitignored, under
   `assets/equipment/gameplay/generations/<id>/export/`.
2. The operator runs the ballistics oracle in Workbench (checkpoint W, next section); its output
   folder is copied to `assets/scratch/ballistics_oracle/<id>/` (gitignored).
3. `cargo xtask ballistics trim-export --generation <id>` (the `--oracle <folder>` default is that
   scratch folder) verifies every export file against the export manifest and each oracle file
   against its sidecar, then writes the catalog, the calibration bundle, the negatives and both
   READMEs deterministically (three runs are byte-equal). Each native row's
   `elevation_mils_6400` is fixed by the one forward sample whose range and time of flight equal
   the row within 0.01 m and 0.001 s (414 rows), or by a lattice end, which the engine answers with
   the time-of-flight sentinel −1 (62 rows). An unmatched row refuses the trim. Gravity is the
   oracle's, and `gravity_source` is "oracle".
4. `cargo xtask schema validate` checks both documents, their provenance and coverage.
5. The calibration tests load the committed catalog and bundle through `include_str!` and evaluate
   every case of every shell and charge; each negative variant fails with its own failure.
6. A new game build repeats steps 1–5 into a new `catalog_version`. Earlier versions stay stored
   and servable, because saved fire missions pin (catalog_id, catalog_version).

## Oracle run (checkpoint W)

The oracle lives in tbd-export only: a Workbench plugin in
`apps/mod/tbd-export/Scripts/WorkbenchGame/BallisticsOracle/` (menu `Plugins > TBD > Ballistics
Oracle`) and a play-mode component in `apps/mod/tbd-export/Scripts/Game/TBD/Export/BallisticsOracle/`
on the export game mode. Nothing ships in tbd-framework. The steps are the runbook
[ballistics oracle run](/documentation/runbooks/ballistics_oracle_run.md).

1. Edit mode: the plugin writes
   `$profile:TBD_BallisticsOracle/<export_generation_id>/forward_angles.json`. For every vanilla
   mortar shell and every coefficient it calls the static
   `BallisticTable.GetDistanceOfProjectileSource` from 1600 to 800 mils in 1.5625-mil steps (513
   elevations per coefficient), and records the raw outputs of
   `GetAimHeightOfProjectileAltitudeFromSource` for Δh ∈ {−200, −100, −50, 0, 50, 100, 200} m at
   five distances per charge.
2. Play mode (server): the component writes `simulation.json`. It spawns each shell prefab and, for
   charges × elevations {45, 55, 65, 75, 85}° × wind {calm, 5 and 10 m/s from 0, 90, 180, 270°} ×
   target height {−100, 0, 100} m, calls `ProjectileMoveComponent.GetProjectileSimulationResult`
   (its compiled signature adds `mustFallDown`, a maximum time and the horizontal distance, and
   returns the end position; the time of flight comes from a 16-step bisection on the time limit).
   It records the raw vector, its decoding and the gravity from `PhysicsWorld.GetGravity`.
3. Every output carries the game build, the plugin revision (`tbd-ballistics-oracle/2`), the run
   time and a sidecar sha256.
4. Altitude cases come from `GetProjectileSimulationResult` with a target height; the
   `GetAimHeight…` outputs are carried raw and not judged.

The committed bundle comes from the second run of 2026-09-28: `forward_angles.json` 13,888,200
bytes and `simulation.json` with 4,185 samples and no shell error, both equal to their sidecars.
The simulation availability probe passed without the projectile-debugging diagnostic enabled.

## Catalog upload lifecycle

1. An admin opens `/admin/ballistics-catalogs` and uploads two files: the catalog and its
   calibration bundle.
2. `POST /api/v1/ballistics-catalogs` (admin; multipart parts `catalog`, capped at 1 MiB, and
   `calibration`, capped at 16 MiB; route body limit 17 MiB + 64 KiB) decodes both, runs `evaluate`
   on the blocking pool, and answers:
   - 201 with a `CatalogUploadReport {accepted, cases, failures[], forward_samples_not_judged}`
     when every case passes;
   - 422 with the report under `details` when any case or provenance check fails;
   - 409 when the (catalog_id, catalog_version) or the (catalog_id, catalog_sha256) exists
     (`catalog_version_exists`, `catalog_bytes_exist`);
   - 400, 413 or 415 in the standard error envelope for a malformed, oversized or wrongly typed
     body.
3. The row and its audit record (`append_actor_audit`, action `ballistics_catalog.uploaded`) are
   written in one transaction.
4. A stored catalog never changes: a trigger refuses every UPDATE and DELETE. A correction is a new
   `catalog_version`.
5. Reads are public: `GET /api/v1/ballistics-catalogs` lists summaries, and
   `GET /api/v1/ballistics-catalogs/{catalogId}/versions/{version}` returns the document with
   `ETag` = the catalog sha256 (304 on a match) and `Cache-Control: public, max-age=31536000,
   immutable`.

## Fire-mission API and migration 0060

Routes of the [operations](/documentation/glossary/n_to_z.md#operations) domain:

- `POST /api/v1/fire-missions/solve` is removed.
- `POST /api/v1/fire-missions` (signed-in user) takes a `FireMissionSave`: optional event, catalog
  id and version, weapon, shell, optional charge rings, target {x, y, height_m, height_source
  dem|manual}, guns [{label, x, y, height_m, height_source}] (one to twelve), optional wind
  {speed_m_s, from_deg}, optional burst height, the target grid and the client's
  `FireMissionSolution`. The server loads the pinned catalog (unknown catalog or version → 404),
  re-solves through `solve_fire_mission` on the blocking pool, and compares with
  `compare_solutions`: a mismatch answers 422 `details.code = solution_mismatch` with both values
  and the server's solution; a save the assembler refuses answers 422 `fire_mission_refused`; a
  lead gun whose fired charge does not solve answers 422 `no_firing_solution`. Otherwise it stores
  the server solution and the guns in one transaction and answers 201 `{solution, fire_mission}`. A
  foreign-key violation answers 404.
- `GET /api/v1/events/{id}/fire-missions` goes through `viewer_event_access`
  (`apps/api/src/operations/services/event_access/visibility.rs`), which decides 404 or
  403. Legacy rows, including `weapon_system` "M120 120mm", list with their original strings and are
  marked not re-solvable.

Migration `apps/api/migrations/0060_game_ballistics_catalogs_and_fire_mission_inputs.sql`:

- `ballistics_catalogs`: catalog_id, catalog_version, title, game_build, export_generation_id,
  catalog_sha256, calibration_sha256, catalog_document jsonb, calibration_document jsonb,
  validation_report jsonb, uploaded_by, uploaded_at (default now()). Primary key (catalog_id,
  catalog_version); unique (catalog_id, catalog_sha256); a BEFORE UPDATE OR DELETE trigger raises.
- `fire_missions` gains nullable columns: catalog_id, catalog_version, weapon_id, shell_id,
  charge_rings smallint, target_height_m, target_height_source (CHECK dem|manual), wind_speed_m_s,
  wind_from_deg, burst_height_m, fuze_time_s, mils_per_circle, dispersion jsonb, solver_revision,
  detached_event_id uuid. A foreign key (catalog_id, catalog_version) references the catalogs; a
  CHECK makes the new-model columns all present or all absent, which is what separates a re-solvable
  row from a legacy one.
- Dangling events are preserved and detached: where the event row does not exist, `event_id` moves
  to `detached_event_id` and `event_id` becomes NULL; then `event_id` gains its foreign key to
  `events(id)`, ON DELETE SET NULL.
- `fire_mission_guns`: fire_mission_id (foreign key, ON DELETE CASCADE), gun_index, label, x, y,
  height_m, height_source, azimuth_mils, elevation_mils, charge_rings, time_of_flight_s; primary key
  (fire_mission_id, gun_index).

## Offline design

- Crate `offline_service_worker` (`apps/offline_service_worker/`). Its lib is pure
  and native-tested:
  - only the shell cache name carries the build id, so a new build replaces the shell and keeps
    the map assets; activation deletes every stale `tbd-offline-*` cache;
  - request classes: navigations are network-first, stored under `/`; the other shell files and
    the icon font are cache-first; catalog documents (`/api/v1/ballistics-catalogs/*/versions/*`) are
    cache-first; the catalog list is network-first with a cache fallback; `/map-assets/*` is
    cache-first, and a `Range` request is answered 206 with a slice of the full cached body (416
    when unsatisfiable); everything else passes through;
  - the pack list comes from the served terrain manifest plus a tile index
    (`/map-assets/<terrain>/tiles/map/index.json`, written next to the pyramid by
    `cargo xtask map tile-index --terrain everon`, 5,461 tiles for Everon z0–6, and served by both
    the gate server and the API). Without an index the pack is "incomplete", never "ready".
- Its bin is the wasm32 service worker (install, activate, fetch through web-sys, delegating every
  decision to the lib). The loader `apps/frontend/service_worker.js` imports the bindgen
  output and registers the install, activate and fetch handlers synchronously; it holds no policy.
  It is served with `Cache-Control: no-cache` and registered as `/service_worker.js?build=<bundle
  hash>`.
- The page's offline core (`apps/frontend/src/foundation/offline/`) registers the worker at
  boot. On the first `/tools/mortar` visit it checks `navigator.storage.estimate()`, calls
  `persist()`, and downloads the pack with progress. The document element carries
  `data-offline-state` ∈ idle | downloading | ready | incomplete | quota-short | unsupported |
  failed, and `data-offline-progress`.
- Cached responses keep all their headers, COOP and COEP included. Satellite xyz tiles are never
  cached (the renderer reads `everon-sat.tbd-sat`, 152,713,114 bytes, by Range requests).
- Other browser gates bypass the service worker (`Network.setBypassServiceWorker`), so a cached
  build never answers a gate that tests the live one. The runbook is
  [offline mortar page](/documentation/runbooks/offline_mortar_page.md).

## Register

Minimums are the counts the orchestrator measures after implementation, never lowered. The
map-engine crate has two `#[ignore]` tests, so its checks use name filters and never `--quiet`.

Every map-engine check runs the one command
`cargo test -p map_engine --all-features --locked data::scenario::ballistics::` and counts
its own modules with `(?m)^test data::scenario::ballistics::(?:<modules>)::[A-Za-z0-9_:]+ \.\.\. ok$`;
each of the 18 modules belongs to exactly one check.

| Requirement | Check | Modules or command | Minimum |
|---|---|---|---|
| `game_ballistics_flight_model` | `game_ballistics_flight_model` | `flight_model`, `wind`, `angular_units`, `catalog` | 51 |
| `game_ballistics_calibration` | `game_ballistics_calibration` | `calibration` | 34 |
| `game_ballistics_elevation_wind_dispersion` | `game_ballistics_elevation_wind_dispersion` | `solver`, `dispersion`, `fuze`, `crest_clearance`, `tests_bounded_failure`, `tests_end_to_end`, `tests_oracle_elevation_and_wind`, `tests_symmetry` | 81 |
| `game_ballistics_battery` | `game_ballistics_battery` | `battery`, `fire_mission`, `fire_mission_comparison` | 29 |
| `game_ballistics_wasm_agreement` | `game_ballistics_shared_solution_cases` | `agreement_cases`, `solution_wording` | 18 |
| | `game_ballistics_wasm_agreement` | `cargo xtask mk ballistics-wasm-agreement` (trunk release build plus the developer_tools `gate ballistics-agreement`): `(?m)^case ballistics_wasm_agreement_[A-Za-z0-9_]+ \.\.\. ok$`, marker `ballistics-wasm-agreement: PASS 32/32` | 32 |
| `game_ballistics_offline_page` | `game_ballistics_offline_page`, plus `frontend_quality` and `browser_acceptance` | `cargo xtask mk mortar-offline-gate` (the developer_tools `gate mortar-offline`): `(?m)^case mortar_offline_[a-z0-9_]+ \.\.\. ok$`, marker `mortar-offline: PASS` | 15 |
| `verification_game_ballistics` (catalog API and saved fire missions) | `verification_game_ballistics`, plus `backend_regression`, route acceptance and contract parity | `cargo xtask db test-it`, `game_ballistics*` | 29 |

Recorded assumptions: dispersion is not verified in-engine; the offline gate needs the local
gitignored tile pyramid, tile index and tbd-sat and fails closed without them; the oracle fixtures
are pinned to game build 1.8.0.13.

The native/WASM agreement bench solves a deterministic lattice of cases (a splitmix64 seed over
catalog shells, targets, heights, wind and guns) in the browser at the URL-only debug route
`/debug/ballistics-agreement?seed=&count=` and natively in the gate, which first checks that the
served catalog's sha256 equals the committed catalog's. It prints one case line per solution and a
bit-identity count, and exits non-zero on any failure or on zero cases. Against the committed
catalog all 32 cases agree and all 32 are bit-identical.

## Settled model questions

Each question was settled against the committed bundle of game build 1.8.0.13; the calibration
module's [README](/legacy/map_engine/src/data/scenario/ballistics/calibration/README.md)
holds the evidence and `ballistics/calibration/tests/committed_bundle.rs` pins it.

| # | Question | Answer |
|---|---|---|
| 1 | What native-table column 1 holds | `column_1 = range_m · tan(elevation)`, the height of the line of departure above the point of fall: on every committed row `atan2(column_1, range_m)` equals the row's elevation within 0.01 mil. A row's elevation is therefore recoverable from the table alone. No criterion reads it. |
| 2 | What the wind tables' `m_aValues` holds, and how a wind table's wind is oriented | Each row's values are [crosswind deflection, range change, angle of fall] at the table's wind speed (10 m/s on every committed table): `values[0]` is 1000·atan(deflection / downrange) in milliradians at the point of fall under a full crosswind (within 0.009 mrad of the model); `values[1]` is the range change in metres under a wind along the line of fire, half the tail-wind minus head-wind difference; `values[2]` is the angle of fall in degrees (within 0.15°). Both effects are symmetric, so the table carries no orientation. A row's `elevation_rad` is stated to three decimals, up to half a mil off its one-degree lattice point, so the wind effects are flown at the elevation inside the row's ± 1 mil window where the model's calm range equals the row's. |
| 3 | Whether SideAirDragScale (10 on every shell) enters the flight | It does not. Over the oracle's 10 m/s crosswind samples at target height 0 the isotropic model's drift is 0.998 to 1.002 times the engine's (the worst crossrange difference is 0.54 of the 1 mil tolerance); a side-drag term of scale 10 drifts 7.95 to 9.93 times the engine's. The flight model's omission stands, pinned by `side_air_drag_scale_never_enters_the_flight_parameters`. |
| 4 | The unit of InitSpeedVariation | Metres per second (±3 m/s on a 66 m/s shell): a 7.62×39 mm cartridge in the same export has InitSpeed 732 with InitSpeedVariation 7, about ±1 % in m/s and ±51 m/s as a percent. No engine call returns a speed spread, so the reading stays part of the documented dispersion interpretation. |
