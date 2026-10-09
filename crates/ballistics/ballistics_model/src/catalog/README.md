# Game ballistics catalog

The typed form of one game ballistics catalog: the weapons and shells whose flight parameters the
fire-mission solver flies, pinned to the game build and equipment export they came from. It
decodes a catalog document and answers typed lookups of a weapon, a shell and a charge, with the
flight model inputs of that firing.

## Contents

```text
crates/ballistics/ballistics_model/src/catalog/
├── mod.rs     `BallisticsCatalog`, `CatalogResource`, `GravitySource`, `from_json_slice` and its error
├── weapon.rs  `WeaponSystem`: sight convention, elevation limits, muzzle factor, dispersion disc
├── shell.rs   `Shell`, `ShellRole`, `Charge`, `TimeFuze`; charge by ring count and the default charge
├── lookup.rs  `CatalogLookupError`, `ResolvedFiring`, `resolve_firing`, flight parameters, muzzle speed
└── tests/     decode and schema-parity tests and lookup tests over the hand-written sample catalog
```

## How it works

Every type mirrors a definition of
[`ballistics-catalog.schema.json`](/contracts/definitions/ballistics-catalog.schema.json) field
for field and refuses unknown fields. `BallisticsCatalog::from_json_slice` decodes one document
and refuses any `schema_version` but 1; it checks shape, not the schema's value ranges or the
cross-references between weapons and shells.

`resolve_firing(weapon_id, shell_id, rings)` refuses, in this order, an unknown weapon, an unknown
shell, a shell the weapon does not fire and a ring count the shell has no charge for. Otherwise it
returns the weapon, shell and charge with the muzzle speed `init_speed_m_s × charge
init_speed_coef × weapon muzzle_init_speed_coef` and the
[`FlightParameters`](../flight_model/mod.rs): catalog gravity, shell mass, air drag, wind influence
and lifetime at the engine's fixed 1/30 s step (`DEFAULT_INTEGRATION_STEP_S`). The shell's `side_air_drag_scale` is decoded and kept but
never enters the flight parameters, because the flight model's drag is isotropic. A weapon's
`mils_per_circle` becomes its [`MilsConvention`](../angular_units.rs).

## Boundaries

- Depends on: `serde`, `serde_json` and `thiserror`; the flight model's `FlightParameters`, the
  mils convention and the typed identifiers (`ids`) of the crate. Its tests also use the `jsonschema` dev-dependency.
- Used by: the firing solver and dispersion of `ballistics_solver`, the fuze of
  `fire_mission_planning` and the calibration of `ballistics_calibration`.
- Rules: the sample catalog `contracts/fixtures/ballistics/minimal_catalog.json` validates against the schema, decoded
  and re-encoded (`sample_catalog_conforms_to_the_schema`,
  `re_encoded_catalog_conforms_to_the_schema_and_round_trips`); decode and schema refuse the same
  unknown fields, missing fields and enum values; each lookup refusal is typed and ordered; the
  side air-drag scale leaves the flight parameters unchanged.
