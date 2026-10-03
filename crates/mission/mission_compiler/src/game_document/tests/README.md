# Game-document compiler tests

Unit tests of the game-document compiler: the locked compile contract, the committed goldens, each
stage's rows, the compile findings and the kit substitutions.

## Contents

```text
crates/mission/mission_compiler/src/game_document/tests/
├── cases_1.rs                    the locked contract: order, slot ids, loadouts and the committed goldens
├── cases_2.rs                    vehicles, crews, win conditions and the compiled slot's exact keys
├── cases_3.rs                    zones, radio, briefings, the authored blocks and the compiler-shaped golden
├── cases_4.rs                    parse refusals, type scans, entities and the findings of dropped identity
├── cases_5.rs                    vocabularies against the schema, flow, environment, settings, substitutions
├── mod.rs                        the shared payload fixtures, metadata, goldens and schema
└── unsupported_authored_data.rs  the authored gameplay data a document cannot carry
```

## Boundaries

- Depends on: the parent module through `use super::*`, the crate root, and the committed
  `contracts/definitions/mission.schema.json` and `contracts/fixtures/missions/valid/` goldens,
  read with `include_str!` from the crate folder.
- Rules: `regen_compiler_shaped_fixture` and `dump_roster_document_for_schema_validate` are
  `#[ignore]`d writers run by hand; the vehicle rows a writer-authored document compiles to are
  tested from the store (`crates/mission/mission_document/src/tests/vehicle_row_round_trips.rs`),
  since this crate never depends on the store.
