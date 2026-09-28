# Ballistics calibration

The judge a ballistics catalog must pass before it is accepted: it flies the catalog's shells
through the flight model and compares them with the game's own ballistic and wind tables and the
engine oracle's samples, all carried in a calibration bundle pinned to the catalog. The catalog
upload of the API and the map engine's tests run the same `evaluate`.

## Contents

```text
apps/website/map-engine/src/data/scenario/ballistics/calibration/
├── mod.rs             `CalibrationBundle`, `OracleRun`, `PinnedCatalog`, `evaluate`, `evaluate_shell`
├── charge_flight.rs   one shell at one coefficient: range window, elevation pinned by range, the criterion
├── native_tables.rs   `NativeTable`, `NativeTableRow`, the row judge and the row-at-elevation match
├── wind_tables.rs     `WindTable`, `WindTableRow`, the value-list decode and the row judge
├── oracle_samples.rs  `OracleSample`, its typed forward-angle and simulation decode and judges
├── provenance.rs      identity, game build, generation, catalog SHA-256, gravity, digests, coverage
├── report.rs          `CalibrationReport` (the upload report's `cases`, `failures`, `forward_samples_not_judged`), `CalibrationFailure`, `FailureKind`
├── sha256_digest.rs   SHA-256 of the catalog bytes as lowercase hexadecimal
└── tests/             unit tests over the committed catalog, bundle and negative variants, and the report's wire keys
```

## How it works

`PinnedCatalog::from_json_slice` decodes a catalog and hashes the exact bytes it was given.
`CalibrationBundle::from_json_slice` decodes a bundle
(`contracts_v2/definitions/ballistics-calibration.schema.json`). `evaluate(pinned, bundle)`
answers a `CalibrationReport {cases, failures}`; the catalog is accepted when `failures` is empty.

Provenance comes first and adds failures, never cases: the bundle names the catalog's id and
version, game build and export generation; its `catalog_sha256` equals the SHA-256 of the catalog
bytes; the catalog's gravity equals the gravity the oracle run reported; no resource GUID carries
two digests across the two documents; every (shell, charge) of the catalog has a native table and
a simulation sample at its coefficient; every table and sample names a catalog shell.

Then every shell of the catalog, in catalog order (`evaluate_shell`). Each shell is flown at
muzzle speed `init_speed_m_s × coefficient` with the catalog's gravity, mass, drag and wind
multiplier, and each case is judged with fixed tolerances, 1 mil of the 6400 convention and 0.1 s:

| Case | Passes when |
|---|---|
| Native-table row | its range lies within the model's range over its elevation ± 1 mil, and the model's time of flight at its elevation is within 0.1 s |
| Forward-angle sample at a native row | as a native row, at the sample's `elevation_rad`; only a sample fired at a native row of its shell and coefficient (within 1e-6 rad) is a case |
| Wind-table row | its range lies within the model's calm range over its elevation ± 1 mil; its crosswind deflection angle lies within 1 mil of the model's, and its range change under wind along the line of fire within `(range + absolute change) · tan(1 mil)` of the model's, at the table's wind speed |
| Simulation sample | the downrange and crossrange differences of the point of fall are each within `D · tan(1 mil)` (`D` the oracle's distance to it), and the time of flight within 0.1 s; a sample the engine never lands passes only if the model refuses it too |

The engine answers a forward-angle call between two native rows by linear interpolation of its
own table, so such a sample says nothing about the flight: it is counted in the report's
`interpolated_forward_samples`, serialised as the upload report's `forward_samples_not_judged`,
not judged, and never counted as a case or a pass. Of the 15,841 committed forward-angle samples, 414 lie at a native row and are
judged; the other 15,427 are interpolation.

The range window uses the fact that the calm range is unimodal in elevation: its minimum is at an
end of the window and its maximum at an end or at the model's maximum-range elevation, found once
per coefficient by golden section. Altitude-difference samples are carried and not judged. Every
failure names its case (`native/<shell>/<coefficient>/<lattice index>`,
`forward/<shell>/<coefficient>/<sample index>`, `wind/<shell>/<coefficient>/<wind speed>/<row>`,
`simulation/…`, `provenance/…`, `coverage/…`) and states the expected and modelled values.

## Model questions settled from the oracle

Measured on the committed bundle of game build 1.8.0.13 (generation 6A6F008DC5395616).

- **Native column 1.** `column_1 = range_m · tan(elevation)`: the height of the line of departure
  above the point of fall. On every committed row `atan2(column_1, range_m)` equals the row's
  `elevation_mils_6400` within 0.01 mil; at the vertical row, where the range is zero, it is the
  limit of that product. A row's elevation is therefore recoverable from the table
  alone. No criterion reads the column; `tests/committed_bundle.rs` pins the identity.
- **Wind-table values.** Each row's `values` is `[crosswind deflection, range change, angle of
  fall]` at the table's wind speed (10 m/s on every committed table), against the model:
  `values[0]` is `1000 · atan(deflection / downrange)`, milliradians, both measured at the point
  of fall under a full crosswind (the downrange under that crosswind, not the calm range; 1570.8,
  that is `π/2`, at the vertical row), within 0.009 mrad of the model on every committed row; `values[1]` is the range change in metres under a wind
  along the line of fire, half the difference between the tail-wind and head-wind ranges;
  `values[2]` is the angle of fall in degrees below the horizontal (within 0.15° of the model on
  every row). The wind is symmetric in both effects, so the table carries no orientation: a head
  wind shortens and a tail wind lengthens by the same `values[1]` to first order.
- **Wind-table elevations.** A row's `elevation_rad` is stated to three decimals, up to half a
  mil off the one-degree lattice the export fires (row `i` at `i + 1` degrees), while its calm
  `range_m` is stated to the millimetre. Near the vertical the crosswind angle changes by several
  milliradians over that half mil, so the wind effects are flown at the elevation inside the
  row's ± 1 mil window where the model's calm range equals `range_m`, found by the solver's
  bracketed root search; it lands on the lattice within 0.011 m over the local range slope.
- **Oracle wind orientation.** The oracle's `wind_vector_world` is
  `-speed · (sin from, 0, cos from)` in Enfusion's x-east, y-up, z-north frame, the flight
  model's convention for a wind blowing from `from` degrees.
- **SideAirDragScale.** It does not enter the flight. Over the oracle's 10 m/s crosswind samples
  at target height 0, the isotropic model's drift is 0.998 to 1.002 times the engine's (the worst
  crossrange difference is 0.54 of the 1 mil tolerance); a side-drag term of scale 10 on the
  air-relative velocity across the shell axis drifts 7.95 to 9.93 times the engine's. The flight
  model's omission stands.

## Boundaries

- Depends on: `serde`, `serde_json`, `thiserror` and `libm`; the sibling `catalog`,
  `flight_model`, `wind` and `solver::elevation_search` modules.
- Used by: this module's tests; `evaluate` is the judge of the API's catalog upload
  (`POST /api/v1/ballistics-catalogs`).
- Rules: the committed catalog, bundle and the four `negative/` variants of
  `contracts_v2/fixtures/ballistics/vanilla_mortars.v1/` are read through `include_str!`; one
  test per shell requires every case of that shell to pass; each negative variant fails with its
  own failure (the provenance variants with nothing else); a SHA-256 mismatch, a game-build mismatch, missing coverage, a
  resource with two digests and an unknown shell are red; SHA-256 matches the FIPS 180-2 vectors;
  a forward sample between native rows is counted and not judged while one at a row is judged; a
  crosswind value 1.01 mil off the model's angle is red and 0.99 mil off is not; every rounded
  wind-row elevation is pinned back to its lattice degree; the single-precision flight meets
  every simulation sample within 0.010 m downrange and 0.001 m crossrange and every native row
  within 0.011 m and 0.001 s; the report serialises `cases`, `failures` and
  `forward_samples_not_judged` and keeps the failure kind off the wire (`tests/report.rs`). All in
  `tests/`.
