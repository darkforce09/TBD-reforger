# Ballistics oracle tooling

The `ballistics_oracle_tooling` crate: `cargo xtask ballistics trim-export`, which writes the
vanilla mortar ballistics catalog and its calibration fixtures from one gameplay equipment export
generation and the ballistics oracle's output for it. The catalog is what the fire-mission solver
reads; the calibration bundle is what the flight model must reproduce before the catalog is
accepted.

## Contents

```text
tools/commands/ballistics_oracle_tooling/
├── Cargo.toml  the `ballistics_oracle_tooling` library package: `repository_layout`, `content_digest`, serde, layout tier 1
└── src/        the export and oracle readers, the catalog and calibration assembly, the writers and the errors
```

## How it works

```text
assets/equipment/gameplay/generations/<id>/export/   (gitignored; manifest SHA-256 per file)
assets/scratch/ballistics_oracle/<id>/                (gitignored; sidecars with size and SHA-256)
        │  every byte verified before use
        ▼  run(BallisticsCmd::TrimExport { generation, oracle })
contracts/catalogs/ballistics/vanilla_mortars.v1.catalog.json
contracts/fixtures/ballistics/vanilla_mortars.v1/{calibration.json, README.md, negative/}
```

The xtask binary parses `BallisticsCmd` and calls `run`, which finds the checkout root, trims and
prints both document hashes, the table, row and sample counts and the gravity. Nothing is written
on a refusal, and the same inputs always write the same bytes. `src/README.md` describes each
module and the command's synopsis and exit codes.

## Getting started

Run from the repository root:

```bash
cargo test -p ballistics_oracle_tooling   # the trim over a synthetic export and oracle output
cargo xtask ballistics trim-export --generation 6A6F008DC5395616
```

## Configuration

No feature and no environment variable. `--oracle <folder>` replaces the default oracle folder
under `assets/scratch/ballistics_oracle/`.

## Public surface

- At the crate root: `BallisticsCmd` (the `ballistics` group's arguments), `run`, `Error` and
  `Result`.
- `prelude`: `BallisticsCmd` and `run`.

## Boundaries

- Depends on: `repository_layout` (the checkout root and the contract folders), `content_digest`
  (SHA-256), `serde`, `serde_json`, `clap` and `thiserror`; `tool_test_support`, `jsonschema` and
  `walkdir` in tests.
- Used by: `tools/xtask/src/cli/dispatch.rs`, for `cargo xtask ballistics`.
- Rules: tier 1 of `tools/commands` (`cargo xtask verify crate-tiers`); the trim never reads an
  unverified byte and never writes over a refusal; the written documents validate against
  `ballistics-catalog.schema.json` and `ballistics-calibration.schema.json` and pin each other by
  hash (`src/tests/trim_export/mod.rs`, and `cargo xtask schema validate` over the committed pair).

## Related documentation

- [Ballistics oracle run](/documentation/runbooks/ballistics_oracle_run.md) — producing the oracle
  output this command reads.
- [Ballistics catalogs](/contracts/catalogs/ballistics/README.md) — the catalog this command
  writes.
- [Ballistics calibration fixtures](/contracts/fixtures/ballistics/README.md) — the bundle and
  its refused variants.
