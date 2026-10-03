# Ballistics model source

The source of `ballistics_model`: the catalog and its lookups, the shell flight model, the wind
and angular units, the typed catalog identifiers, and the crate root that exports them.

## Contents

```text
crates/ballistics/ballistics_model/src/
├── angular_units.rs  `MilsConvention`: degrees, radians and weapon mils; azimuth folding
├── catalog/          the game ballistics catalog types, their decode and typed lookups
├── error.rs          the crate's `Error` over the catalog, lookup, flight, wind and angle errors
├── flight_model/     point-mass shell flight: the engine's single-precision step, crossing, apex
├── ids.rs            `CatalogId`, `ExportGenerationId`, `WeaponId`, `ShellId`
├── lib.rs            the crate root: module header, `mod` lines and the re-exports
├── prelude.rs        the catalog, flight and wind types and the ids for glob import
├── tests/            unit tests of the angle units and the wind convention
└── wind.rs           `Wind`: speed and "from" direction to the air-velocity vector
```

## How it works

`catalog/` and `flight_model/` each carry their own README with their rules. `angular_units.rs`
and `wind.rs` are the conventions both share: a weapon's mils per circle and the folding of an
azimuth into one turn, and the air velocity a "from" wind subtracts from the shell's velocity.

## Boundaries

- Depends on: `newtype_ids`; `serde`, `serde_json`, `thiserror` and `libm`.
- Used by: the other ballistics crates and the consumers that decode catalogs, through the crate
  root.
- Rules: the angle units and the wind convention are pinned in `tests/angular_units.rs` and
  `tests/wind.rs`.
