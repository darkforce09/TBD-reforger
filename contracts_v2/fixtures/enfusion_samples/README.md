# Enfusion mission DTO samples

One sample document for each part of the [mission](/documentation_v2/glossary/g_to_m.md#mission)
contract that the game [mod](/documentation_v2/glossary/g_to_m.md#mod)'s JSON classes read, from the
whole mission down to a zone's circle. The schema gate validates each against its definition in
the mission schema.

## Contents

```text
contracts_v2/fixtures/enfusion_samples/
└── *.sample.json  one document each, named after the `mission.schema.json` definition it samples
```

## How it works

`cargo xtask schema validate`
(`tools_v2/xtask/src/verifications/schemas/checks/contract_validation/validate_all.rs`) walks the
folder in name order and takes the name before `.sample.json` as a definition of
`contracts_v2/definitions/mission.schema.json`. `root.sample.json` is validated as a whole
mission; every other file is validated against `#/$defs/<name>`, and a file whose name has no
definition fails the run. The ten samples cover `root`, `meta`, `faction`, `orbatFaction`,
`group`, `role`, `slot`, `zone`, `shape` and `circle`, the definitions the mod's DTO classes cite
with `@contract mission.schema.json#/$defs/<name>` (for example in
`apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/TBD_MissionLoader.c`).

## Format

- Encoding: UTF-8 JSON, one document per file, named `<definition>.sample.json` with the
  definition's exact spelling, `orbatFaction` included.
- Schema: `contracts_v2/definitions/mission.schema.json`, the whole document for `root` and
  `#/$defs/<definition>` for the rest.
- Adding a file: name it after an existing definition of the mission schema and run
  `cargo xtask schema validate`; files that do not end in `.sample.json` are skipped.

## Producers and consumers

- Producers: people; no tool writes these files.
- Consumers: `cargo xtask schema validate` only, a step of the `schema-validate` CI task.

## Boundaries

- Depends on: the definitions under `$defs` in `contracts_v2/definitions/mission.schema.json`.
- Used by: the xtask schema gate.
- Rules: every sample validates against the definition its name selects, and renaming or removing
  a definition means renaming or removing its sample in the same change
  (`cargo xtask schema validate`).
