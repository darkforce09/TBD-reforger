# Ballistics catalogs

The game ballistics catalogs the fire-mission solver reads: every weapon and shell whose flight
parameters the mortar calculator and the [API](/documentation/glossary/a_to_f.md#api)'s
fire-mission check solve with, pinned to the game build and equipment export they came from. This
is production content an administrator uploads, not test data.

## Contents

```text
contracts/catalogs/ballistics/
└── vanilla_mortars.v1.catalog.json  the vanilla M252 and 2B14 with their seven shells, every charge
```

## How it works

`cargo xtask ballistics trim-export --generation <id>` writes the catalog from one gameplay
equipment export generation and the ballistics oracle's output for it, together with its
calibration bundle in `contracts/fixtures/ballistics/`. Each weapon comes from its mortar
prefab and range card, each shell from its `ShellMoveComponent` (initial speed, mass, air drag,
wind and dispersion factors, time to live) and its charge ring configuration; gravity is the
magnitude the oracle read from the game's physics world. Every value is the engine's 32-bit float
written as its shortest decimal.

An administrator uploads the catalog with its bundle through `POST /api/v1/ballistics-catalogs`;
the API runs the flight model over the bundle and refuses the pair if any case is out of
tolerance. Catalogs are immutable once uploaded: a changed catalog is a new version.

## Format

- Encoding: UTF-8 JSON, pretty-printed with two-space indentation and a final newline, named
  `<catalog_id>.v<catalog_version>.catalog.json`.
- Schema: `contracts/definitions/ballistics-catalog.schema.json` (`BallisticsCatalog`): catalog
  id and version, title, game build, export generation, gravity from the oracle, the source
  resources with their SHA-256, `weapons` and `shells`.
- Adding a file: never by hand. Rerun the trim after a new export or oracle run; the bundle pins
  the catalog's exact bytes by SHA-256, so an edited catalog no longer matches its calibration.

## Producers and consumers

- Producer: `cargo xtask ballistics trim-export` (`tools/xtask/src/commands/ballistics/`).
- Consumers:
  - `cargo xtask schema validate`, whose ballistics section
    (`tools/xtask/src/verifications/schemas/checks/ballistics_validation.rs`) validates the
    catalog and cross-checks it with its calibration bundle;
  - the map engine's calibration tests, which load the catalog and bundle and run the flight model
    over every case;
  - the catalog upload of `POST /api/v1/ballistics-catalogs`, fed by an administrator.

## Boundaries

- Depends on: `contracts/definitions/ballistics-catalog.schema.json`; the gameplay export and
  the oracle output of the generation it names.
- Used by: the schema gate, the map engine's calibration tests and the catalog upload.
- Rules: a catalog and its calibration bundle change together, in one trim; the bundle's
  `catalog_sha256` equals the SHA-256 of the catalog file's bytes (`cargo xtask schema validate`).
