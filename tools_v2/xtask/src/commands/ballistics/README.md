# Ballistics commands

The `cargo xtask ballistics` group: `trim-export` writes the vanilla mortar ballistics catalog and
its calibration fixtures from one gameplay equipment export generation and the ballistics
oracle's output for it. The catalog is what the fire-mission solver reads; the calibration bundle
is what the flight model must reproduce before the catalog is accepted.

## Contents

```text
tools_v2/xtask/src/commands/ballistics/
├── calibration_assembly.rs  one shell's native tables, wind tables and oracle samples at its charges
├── catalog_extraction.rs    weapons and shells from mortar prefabs, range cards and shell components
├── cli.rs                   the `BallisticsCmd` clap enum: `trim-export`
├── contract_documents.rs    serde shapes of the catalog and calibration schemas
├── dispatch.rs              runs a `BallisticsCmd` against the checkout and prints the summary
├── engine_numbers.rs        engine floats as the shortest decimal of their 32-bit value
├── game_tables.rs           the game's ballistic and wind tables from their configurations
├── gameplay_export.rs       the export generation: completeness, resource index, manifest hashes
├── mod.rs                   the module tree
├── negative_variants.rs     the four refused bundles, one defect each
├── oracle_output.rs         the oracle's forward-angle and simulation output, verified by sidecar
├── provenance_readme.rs     the fixture and negative READMEs with the full provenance
├── record_per_line_json.rs  indented JSON with one table row or oracle sample per line
├── row_elevations.rs        each native row's elevation, fixed by the oracle's forward samples
├── tests/                   the trim tests over a synthetic export
└── trim_export.rs           `trim-export`: assembles and writes every document
```

## How it works

```text
assets_v2/equipment/gameplay/generations/<id>/export/   (gitignored)
  │ manifest SHA-256 per file
  ├─ mortar prefab ─ SCR_MortarMuzzleComponent, SCR_TurretControllerComponent
  ├─ range card    ─ SCR_VisualisedBallisticConfig pages: shells, coefficients, dispersion, mils unit
  ├─ shell prefab  ─ ShellMoveComponent, SCR_MortarShellGadgetComponent, TimerTriggerComponent
  └─ BallisticTableArray and SCR_ProjectileWindTable configurations
assets_v2/scratch/ballistics_oracle/<id>/                (gitignored)
  └─ forward_angles.json, simulation.json + _meta.json sidecars (size and SHA-256)
        │
        ▼ trim-export
contracts_v2/catalogs/ballistics/vanilla_mortars.v1.catalog.json
contracts_v2/fixtures/ballistics/vanilla_mortars.v1/{calibration.json, README.md, negative/}
```

The mortars are named once, in `catalog_extraction::VANILLA_MORTARS`: the M252 with the US range
card and the 2B14 with the USSR range card. Everything else follows from the export: a weapon's
shells are the shells its range card prints, its mils per circle is the card's unit
(`MILS_NATO` 6400, `MILS_WP` 6000), its caliber is its default ammunition's diameter, and a shell's
standard dispersion is the card page at its default charge. A shell's role comes from its
`EAmmoType` flags (illumination, smoke, training as practice, high explosive).

Only the catalog's charges enter the bundle: the native table at each charge coefficient, the
wind tables at charge coefficients, and the oracle's samples at those coefficients. Forward
samples the engine answered with its time-of-flight sentinel −1 are left out. A native row stores
no elevation, so `row_elevations` fixes each from the forward samples, whose lattice is fine enough
to hold every row: exactly one sample with equal range and time of flight (0.01 m, 0.001 s), or a
lattice end, which the engine answers with the sentinel (the vertical first row at range 0 and the
last row at the lattice's last elevation). Every native row of every kept table enters the bundle;
a row neither rule fixes refuses the trim, naming the row. Gravity is the magnitude the oracle
read from the physics world.

## Commands

### trim-export

- Synopsis: `cargo xtask ballistics trim-export --generation <id> [--oracle <folder>]`; the oracle
  folder defaults to the generation's folder under assets_v2/scratch/ballistics_oracle.
- Does: verifies the export and the oracle output, then writes the catalog, the calibration
  bundle, the four refused bundles and both READMEs, byte for byte the same for the same inputs;
  prints both document hashes, the table, row and sample counts and the gravity.
- Exit codes: 0 written; 1 any refusal, with its cause: a missing or incomplete export or oracle
  output, a hash that differs from the manifest or a sidecar, a generation, build or plugin
  revision mismatch, a missing component or property, or an unmatched native row. Nothing is
  written on a refusal.
- Example: `cargo xtask ballistics trim-export --generation 6A6F008DC5395616`

## Boundaries

- Depends on: `crate::core::repository_root`, `developer_tools::repository_layout` for the
  contract folders; the gameplay export and the oracle output of the generation.
- Used by: `tools_v2/xtask/src/cli/dispatch.rs`; people, after a new equipment export or oracle
  run (the oracle plugin lives in `apps/mod/tbd-export/Scripts/WorkbenchGame/BallisticsOracle/`).
- Rules: the trim never reads an unverified byte and never writes over a refusal; two runs over
  the same inputs write identical bytes; the written documents validate against
  `ballistics-catalog.schema.json` and `ballistics-calibration.schema.json` and pin each other by
  hash (`tests/trim_export/mod.rs`, and `cargo xtask schema validate` over the committed pair).

## Related documentation

- [Ballistics catalogs](/contracts_v2/catalogs/ballistics/README.md) — the catalog this command
  writes.
- [Ballistics calibration fixtures](/contracts_v2/fixtures/ballistics/README.md) — the bundle and
  its refused variants.
