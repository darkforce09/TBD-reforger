# Ballistics crates

The library crates of the mortar ballistics behind the field tools, driven by a catalog of the
game's own weapon and shell values: the shell flight model, its inverse (the firing solver, which
answers the azimuth and an elevation per charge from a gun to a target), the fire-mission
assembler, the calibration a catalog must pass, and the seeded cases that prove the browser build
solves like the native one. The [API](/documentation/glossary/a_to_f.md#api) re-solves every
saved fire mission with them and the mortar calculator solves on the device with the same code.

## Contents

```text
crates/ballistics/
├── ballistics_agreement_cases/  `ballistics_agreement_cases`: the seeded lattice of battery cases and their bit patterns
├── ballistics_calibration/      `ballistics_calibration`: a catalog judged against the game's tables and the engine oracle
├── ballistics_model/            `ballistics_model`: the catalog, the shell flight model, wind, angular units, typed ids
├── ballistics_solver/           `ballistics_solver`: elevation per charge, wind-corrected aim, crest clearance, dispersion
└── fire_mission_planning/       `fire_mission_planning`: battery, fuze, the fire-mission assembler, comparison, wording
```

## How it works

The tiers follow the data flow. `ballistics_model` (tier 1) decodes a catalog and flies a shell
the way the engine does. `ballistics_solver` (tier 2) inverts that flight per charge.
`fire_mission_planning` and `ballistics_calibration` (tier 3) build on both: the first assembles a
whole fire-mission solution for a battery, the second judges a catalog against the game's
tables. `ballistics_agreement_cases` (tier 4) draws battery problems over a catalog and hands
them to the assembler. Every crate declares `category = "crates/ballistics"` and a tier one above
the highest workspace crate it depends on; transcendentals come from `libm`, so a solution has the
same bits on native and wasm32 builds.

## Getting started

Run from the repository root:

```bash
cargo test -p ballistics_model -p ballistics_solver -p fire_mission_planning   # model, solver, planner
cargo test -p ballistics_calibration -p ballistics_agreement_cases             # calibration, agreement cases
cargo xtask verify crate-tiers                                                  # tiers and category edges
```

## Boundaries

- Depends on: external crates (`serde`, `serde_json`, `thiserror`, `libm`) and the foundation
  crates `newtype_ids`, `content_digest` and `deterministic_random`.
- Used by: the API's fire-mission and ballistics-catalog services, the mortar calculator and the
  agreement bench of the single-page app, and the agreement and offline mortar gates of the
  developer tools.
- Rules: a ballistics crate depends on foundation and ballistics crates only, never on an
  application or a tool (`cargo xtask verify crate-tiers`); every public identifier is a
  newtype id that serialises as its bare string, so catalog, fire-mission and calibration JSON keep
  their bytes (`cargo xtask verify crate-anatomy`).

## Related documentation

- [Game ballistics documentation](/documentation/crates/ballistics/README.md) — the feature
  documentation of the flight model, the solver, the calibration and the assembled fire-mission
  solution.
- [Library crates](/crates/README.md) — the categories and the tier rule.
