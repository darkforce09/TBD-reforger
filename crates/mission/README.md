# Mission crates

The library crates of the [mission](/documentation/glossary/g_to_m.md#mission) domain: the parts of
the mission model, its checks and its compilation that the API and the map engine share.

## Contents

```text
crates/mission/
├── formation_geometry/   `formation_geometry`: the placement patterns, align, space, orient and garrison positions of the arrange commands
├── mission_compiler/     `mission_compiler`: the game-document compiler, its compile findings and the compiler identity
├── mission_crdt/         `mission_crdt`: the native id arrays, slot columns and undo grouping clocks of the mission document
├── mission_document/     `mission_document`: the mergeable mission document, its rows, selection, ids and undo
├── mission_model/        `mission_model`: the compiled rows, ORBAT projection, authored blocks, slot line and ids of a mission
├── mission_operations/   `mission_operations`: the authoring commands and row projections of the mission document
├── mission_payload/      `mission_payload`: the editor payload, export envelope and version body compiler, with the kit aliases
├── mission_validation/   `mission_validation`: the ordered validation rules of an editor payload, their findings and self-check
└── mission_wire_safety/  `mission_wire_safety`: the control-character and cargo capacity scans of an editor payload
```

## How it works

Mission crates depend on foundation, mission and geometry crates only, so the mission domain
compiles without graphics, terrain or application code; the API links them with no graphics crate
in its tree. Each crate declares `category = "crates/mission"` and a tier one above the
highest workspace crate it depends on.

## Boundaries

- Depends on: external crates, and the foundation, mission and geometry crates.
- Used by: the API and the map engine.
- Rules: a mission crate declares `category = "crates/mission"`, depends on no crate outside the
  foundation, mission and geometry categories and never on `legacy/`, an application or a tool
  (`cargo xtask verify crate-tiers`).
