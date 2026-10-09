# Game-document compiler tests

Unit tests of the game-document compiler: the locked compile contract, the committed goldens, each
stage's rows, the compile findings and the kit substitutions.

## Contents

```text
crates/mission/mission_compiler/src/game_document/tests/
├── cases_1.rs                    the locked contract, vehicles, entities, settings, orphans and win conditions
├── cases_2.rs                    authored modes, the slot loadout mapper, one-faction compiles, zones, radio
├── cases_3.rs                    radio, the authored blocks, substitutions, briefings, markers and the compiler-shaped golden
├── cases_4.rs                    the parse precheck, type scans, flow, environment and the compile findings
├── cases_5.rs                    slot identity, squad leaders, the schema's enums and the vehicle roster
├── mod.rs                        the shared payload fixtures, metadata, goldens and schema
└── unsupported_authored_data.rs  the authored gameplay data a document cannot carry
```

## Boundaries

- Depends on: the parent module through `use super::*`, the crate root, and the committed
  `contracts/definitions/mission.schema.json` and `contracts/fixtures/missions/valid/` goldens,
  read with `include_str!` from the crate folder.
- Rules: `regen_compiler_shaped_fixture` is an `#[ignore]`d writer run by hand.
