# Ballistics calibration fixtures

The calibration bundles of the ballistics catalogs in `contracts/catalogs/ballistics/`: the
game's own ballistic and wind tables and the engine oracle's answers that the flight model must
reproduce within tolerance before a catalog is accepted, and the bundles that must be refused.

## Contents

```text
contracts/fixtures/ballistics/
├── minimal_catalog.json  the hand-written test catalog: three shells, two weapons, read by every ballistics crate's tests
└── vanilla_mortars.v1/  the calibration bundle of the vanilla mortar catalog, and its refused variants
```

## How it works

`cargo xtask ballistics trim-export --generation <id>` writes each folder whole, together with its
catalog: `calibration.json` (the bundle), `negative/` (copies with exactly one defect each: a
native row skewed by 5 m, a wrong game build, a missing shell, a stale catalog hash) and a README
with the full provenance: the game build, the export generation, every source resource's GUID and
SHA-256, the oracle run and the hashes of its output, and how many native rows each kind of
elevation evidence fixed. Every native row of every kept table is in the bundle: the trim refuses
a row that no forward sample (or the lattice-end rule) matches.

A bundle passes when every native table row, wind table row and oracle sample agrees with the
flight model within 1 mil (6400 convention) and 0.1 s, its game build equals the catalog's, its
`catalog_sha256` is the SHA-256 of the catalog's bytes, and every charge of every shell has a native
table and a simulation sample. The `ballistics_calibration` evaluator applies these rules; the
catalog upload runs the same evaluator.

## Format

- Encoding: UTF-8 JSON, pretty-printed with two-space indentation and a final newline; one folder
  per catalog version, named `<catalog_id>.v<catalog_version>`.
- Schema: `contracts/definitions/ballistics-calibration.schema.json` (`CalibrationBundle`).
- Adding a file: never by hand; rerun the trim. A fixture edited to pass hides a flight-model or
  export regression.

## Producers and consumers

- Producer: `cargo xtask ballistics trim-export` (`tools/xtask/src/commands/ballistics/`).
- Consumers: `cargo xtask schema validate`, which validates each bundle and cross-checks its
  provenance and coverage against its catalog; the `ballistics_calibration` tests, which run every
  case and require each negative variant to fail for its own reason; the catalog upload of
  `POST /api/v1/ballistics-catalogs`.

## Boundaries

- Depends on: `contracts/definitions/ballistics-calibration.schema.json` and the catalog each
  bundle pins.
- Used by: the schema gate, the `ballistics_calibration` tests and the catalog upload.
- Rules: a bundle and its catalog change together; a negative variant carries one defect only, so
  its refusal names that defect.
