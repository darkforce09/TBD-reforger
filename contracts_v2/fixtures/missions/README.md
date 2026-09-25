# Mission fixtures

The [mission](/documentation_v2/glossary.md#mission) contract's golden corpus: complete missions
the contract must accept, and wrapped documents it must reject, each for one stated reason.

## Contents

```text
contracts_v2/fixtures/missions/
├── invalid/  wrapped documents the gates must reject, each naming its gate and JSON pointer
└── valid/    complete missions that must pass the schema, the size ceiling and the kit-alias check
```

## How it works

`cargo xtask schema validate` reads both folders
(`tools_v2/xtask/src/verifications/schemas/checks/mission_validation.rs`). Every file in `valid/`
must pass `contracts_v2/definitions/mission.schema.json`, stay under the schema's
`x-tbd-missionFileMaxBytes` ceiling (8 MiB, the value of `MISSION_FILE_MAX_BYTES` in the
[mod](/documentation_v2/glossary.md#mod)'s mission loader) and use only kit aliases that the mod's
spawn registry defines. Every file in `invalid/` must be rejected by the gate its wrapper names,
with every finding at or below the wrapper's JSON pointer, so a fixture rejected for an unrelated
reason fails the run. Beyond the gate, the xtask mod commands stage missions from `valid/` by name,
and the map engine's compiler, the [API](/documentation_v2/glossary.md#api)'s contract tests and the
xtask schema tests each load single missions from it.

## Format

- Encoding: UTF-8 JSON, one document per file, named in lowercase hyphenated words after what the
  file holds.
- Schema: a file in `valid/` is a mission following
  `contracts_v2/definitions/mission.schema.json`; a file in `invalid/` is a wrapper whose
  `document` is such a mission with one defect, and whose `mustFail` names the gate (`schema` or
  `registry`), the pointer and the reason. The child READMEs give the details.
- Adding a file: put it in the folder that matches, check a mission alone with
  `cargo xtask schema validate-file <path>`, then run `cargo xtask schema validate`.

## Producers and consumers

- Producers: people, apart from `valid/compiler-shaped-two-faction.json`, which the map engine's
  mission flatten tests regenerate.
- Consumers: `cargo xtask schema validate` and its `schema-validate` CI task; the xtask mod
  commands `world-boot`, `test-mission` and `dev-server`; the map engine's compiler flatten tests;
  the API's contract tests; and the xtask schema tests. `valid/README.md` lists each with its path.

## Boundaries

- Depends on: `contracts_v2/definitions/mission.schema.json`, and the spawn registry
  `apps/mod/tbd-framework/Data/registry.json`, whose `entries[].alias` values are the kit aliases a
  mission may name.
- Used by: the xtask schema gate, the xtask mod commands and the tests named in the child READMEs.
- Rules: a valid mission stays valid and under the size ceiling; an invalid fixture breaks exactly
  one rule, at its declared pointer, and is never malformed JSON (`cargo xtask schema validate`);
  commands and tests load missions by file name, so a rename updates every `git grep` hit in the
  same change.
