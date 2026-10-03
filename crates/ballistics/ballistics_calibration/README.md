# Ballistics calibration

The `ballistics_calibration` crate: the judge a ballistics catalog must pass before the API
accepts it. It flies the catalog's shells through the flight model and compares them with the
game's own ballistic and wind tables and the engine oracle's samples, all carried in a
calibration bundle pinned to the catalog by the SHA-256 of its exact bytes.

## Contents

```text
crates/ballistics/ballistics_calibration/
├── Cargo.toml  the package: `ballistics_model`, `ballistics_solver`, `content_digest`, `newtype_ids`; layout tier 3
└── src/        the bundle and the judges, the report, the case id, the error, the prelude and the tests
```

## How it works

`CalibrationBundle::from_json_slice` decodes a bundle; `PinnedCatalog::from_json_slice` decodes a
catalog and hashes its bytes with `content_digest::sha256_hex`. `evaluate` judges provenance
first (identity, game build, export generation, the catalog digest, gravity, coverage), then every
shell in catalog order: native-table rows, forward-angle and simulation samples, and wind-table
rows, each case within 1 mil of the 6400 convention and 0.1 s. Every miss is one
`CalibrationFailure` named by its `CalibrationCaseId`; a report accepts its catalog exactly when
it lists no failure. The details of each judge are in [the source README](src/README.md).

## Getting started

Run from the repository root:

```bash
cargo test -p ballistics_calibration   # the committed bundle, negatives, report, digest, oracle inversion
```

## Configuration

None: no features and no environment variables. The catalog and bundle bytes come from the
caller.

## Public surface

- `CalibrationBundle`, `OracleRun`, `CalibrationDecodeError`, `CALIBRATION_SCHEMA_VERSION`,
  `PinnedCatalog`, `evaluate`, `evaluate_shell`.
- `CalibrationReport`, `CalibrationFailure`, `FailureKind`, `CalibrationCaseId`.
- The judge modules `charge_flight`, `native_tables`, `wind_tables`, `oracle_samples` and
  `provenance` with their table and sample types; `Error` and `Result`; `prelude`.

## Boundaries

- Depends on: `ballistics_model`, `ballistics_solver` (the elevation searches), `content_digest`
  and `newtype_ids`; `serde`, `serde_json`, `thiserror` and `libm`.
- Used by: the API's catalog upload
  (`crates/api/api_operations/src/services/ballistics_catalogs/upload_validation.rs`).
- Rules: the committed catalog and bundle of `contracts/fixtures/ballistics/vanilla_mortars.v1/`
  pass shell by shell and each negative variant fails with its own failure
  (`src/tests/committed_bundle.rs`); the catalog digest matches the FIPS 180-2 vectors
  (`src/tests/catalog_digest.rs`); every oracle simulation sample with a target height or a wind
  inverts to the oracle's launch within 1 mil and 0.1 s (`src/tests/oracle_elevation_and_wind.rs`).

## Related documentation

- [Ballistics crates](/crates/ballistics/README.md) — the category and its tiers.
- [Ballistics oracle run](/documentation/runbooks/ballistics_oracle_run.md) — how a new bundle is
  measured and proved.
