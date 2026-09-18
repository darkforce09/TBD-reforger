# Valid Mission Fixtures (`contracts_v2/fixtures/missions/valid/`)

Nine missions that must always parse, validate, and compile.

| Fixture | What it holds open |
|:---|:---|
| `bridgehead-at-levie.json` | A full playable mission; the default for boot and staging tests |
| `last-stand-at-montfort.json` | Second full mission, used by the API validator round trip |
| `compiler-shaped-two-faction.json` | Two-faction ORBAT the compiler must flatten correctly |
| `slot-loadout-coverage.json` | Every gear slot populated across slots |
| `slot-y-absent-and-present.json` | Slot elevation both omitted and supplied, so the absent case stays legal |
| `empty-warning-fields.json` | Empty-but-present warning fields, which must not be read as missing |
| `schema-1_3-wire-fields.json` | The schema 1.3 wire additions |
| `schema-1_3-tasks.json` | Task definitions |
| `schema-1_3-tactical-graphics.json` | Tactical graphics |

The three `schema-1_3-*` fixtures exist because the additions were made schema-first: they are hand-staged missions that exercise fields ahead of the editor emitting them, which is what lets readers ship before writers.
