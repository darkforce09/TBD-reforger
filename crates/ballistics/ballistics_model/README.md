# Ballistics model

The `ballistics_model` crate: the vocabulary every ballistics crate speaks. A catalog names each
weapon and shell with the game's own values; this crate decodes it, resolves one weapon, shell and
charge into flight parameters and a muzzle speed, and flies that shell from the muzzle the way the
Arma Reforger engine does until it descends through a target height.

## Contents

```text
crates/ballistics/ballistics_model/
├── Cargo.toml  the package: `newtype_ids`, `serde`, `serde_json`, `thiserror`, `libm`; layout tier 1
└── src/        the catalog, the flight model, wind, angular units, the typed ids, the error and the prelude
```

## How it works

`catalog` decodes one catalog document (`BallisticsCatalog::from_json_slice`, refusing any
`schema_version` but 1 and every unknown field) and answers typed lookups: `weapon`, `shell`,
`weapon_and_shell` and `resolve_firing`, each with a typed `CatalogLookupError`. `flight_model`
flies the shell with the engine's own step: every 1/30 s gravity first, then quadratic
air-relative drag, then the mean-velocity position, in `f32` like the engine; the crossing of the
target height is interpolated linearly along the last step (`fly_to_height`). A double-precision
twin of the same scheme (`fly_to_height_double_precision`) serves the finite differences of the
dispersion. `wind` turns a speed and a "from" direction into the air velocity the flight
subtracts; `angular_units` converts degrees, radians and the weapon's mils (`MilsConvention`) and
folds azimuths. `ids` declares `CatalogId`, `ExportGenerationId`, `WeaponId` and `ShellId`.

## Getting started

Run from the repository root:

```bash
cargo test -p ballistics_model   # decode and schema parity, lookups, the flight model, wind, angles
```

## Configuration

None: no features and no environment variables. The catalog bytes come from the caller.

## Public surface

- `catalog`: `BallisticsCatalog`, `WeaponSystem`, `Shell`, `ShellRole`, `Charge`, `TimeFuze`,
  `CatalogResource`, `GravitySource`, `CATALOG_SCHEMA_VERSION`, `CatalogDecodeError`, the
  lookups with `CatalogLookupError` and `ResolvedFiring`, `flight_parameters`, `muzzle_speed_m_s`.
- `flight_model`: `FlightParameters`, `Launch`, `FlightError`, `FlightOutcome`, `PathRecording`,
  `fly_to_height`, `fly_to_height_double_precision`, the step constants.
- `wind`: `Wind`, `WindError`; `angular_units`: `MilsConvention`, the degree, radian and mil
  conversions and azimuth normalisation.
- `CatalogId`, `ExportGenerationId`, `WeaponId`, `ShellId`; `Error` and `Result`; `prelude`.

## Boundaries

- Depends on: `newtype_ids`; `serde`, `serde_json`, `thiserror` and `libm`. Its tests also use
  `jsonschema`, which validates the sample catalog against
  `contracts/definitions/ballistics-catalog.schema.json`.
- Used by: every other ballistics crate; the API's catalog upload and store, the mortar
  calculator's catalog DTOs and the developer tools' gates, which decode catalogs through it.
- Rules: the sample catalog `contracts/fixtures/ballistics/minimal_catalog.json` and its
  re-encoding conform to the schema (`src/catalog/tests/catalog_decode.rs`); each lookup refusal
  is typed and ordered (`src/catalog/tests/lookup.rs`); the production flight reproduces the
  native row of the M821 at 45° and seven engine-oracle samples within 0.02 m
  (`src/flight_model/tests/flight_model.rs`); the angle units and the wind convention are pinned
  in `src/tests/angular_units.rs` and `src/tests/wind.rs`.

## Related documentation

- [Ballistics crates](/crates/ballistics/README.md) — the category and its tiers.
- [Game ballistics engine](/documentation/crates/ballistics/game_ballistics_engine.md) — the
  identified engine scheme and the catalog to solution walk.
