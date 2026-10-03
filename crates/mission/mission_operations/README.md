# Mission operations

The `mission_operations` crate: the headless authoring commands the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) runs on the
[mission](/documentation/glossary/g_to_m.md#mission) document. Each takes an explicit
`MissionDocCore` and plain values: reading rows for the docks and dialogs, placing and arranging
entities, the [ORBAT](/documentation/glossary/n_to_z.md#orbat) roster and faction templates,
loadouts and cargo, compositions, zones and triggers, tactical graphics, Editor Layers and the
document search.

## Contents

```text
crates/mission/mission_operations/
├── Cargo.toml  the package: `mission_document`, `mission_crdt`, `mission_model`, `mission_payload`, `formation_geometry`, layout tier 5
└── src/        the authoring commands, the entity half, the row projections, the error and the prelude
```

## How it works

An operation reads the document through its JSON views and `materialize`, decides in plain values,
and writes through the `MissionDocCore` mutators, so every change keeps the document's field-level
transactions and undo grouping. A decision only the host can make arrives as a value or a callback
run at the point it is needed: the folder a placement files into, a bulk confirmation, the crew
toggle, the cargo seed. The ids a command takes or returns are the newtype ids of
`mission_document` (`FactionId`, `SquadId`, `LayerId`, `EntityId`, `VehicleId`, `CommentId`,
`CompositionId`, `ConnectionId`), of `mission_model` (`SlotId`, `ZoneId`, `TriggerId`,
`MarkerId`, `TacticalGraphicId`) and `mission_validation::AssetId`; each is written into the
document as its bare string. The Apply anchor of a faction template is the terrain centre from
`map_coordinates::terrain_frames` (`ARLAND_CENTRE` on Arland, `ANCHOR` elsewhere). The
[source README](src/README.md) describes the modules and the session state.

## Getting started

Run from the repository root:

```bash
cargo test -p mission_operations   # entity, roster, faction apply, cargo, tactical draw and prelude tests
```

## Configuration

No features and no environment variables. The undo-grouping tests enable `mission_document`'s
`test_fixtures` feature from `[dev-dependencies]`.

## Public surface

- The modules `apply_faction`, `assets`, `attrs`, `cargo`, `cargo_rules`, `compositions`,
  `document_index`, `entity`, `environment`, `faction_library`, `place_orbat`, `projections`,
  `reassign`, `rotation`, `rows`, `slot_ids`, `tactical_graphics`, `transform` and `zones`.
- `Error` (`InvalidSide`, `WouldCollapseSquads`) and `Result`: the refusals of
  `apply_faction::apply_faction_library` and `place_orbat::place_character_under_side`.
- `prelude`: the side-level commands, the plain rows and their projections, and the authoring
  session state a host keeps between gestures.

## Boundaries

- Depends on: `mission_document`, `mission_crdt`, `mission_model`, `mission_payload` (terrain
  bounds), `mission_validation` (`AssetId`), `formation_geometry`, `map_coordinates`, `serde`,
  `serde_json`, `thiserror`.
- Used by: the map engine (`legacy/map_engine/src/editing/hosted_commands/` borrows the hosted
  document and calls these) and the Mission Creator in `apps/frontend/src/workspaces/editor/`.
- Rules: mission tier 5 (`cargo xtask verify crate-tiers`); every write goes through a
  `MissionDocCore` mutator, so a command is one undo step and a refusal writes nothing; no
  primitive public id field or parameter (`cargo xtask ci verify-workspace-laws`).

## Related documentation

- [Mission crates](/crates/mission/README.md) — the mission domain's crates and their tiers.
- [Mission document](/crates/mission/mission_document/README.md) — the document these commands
  edit.
- [Mission Creator feature inventory](/documentation/apps/frontend/workspaces/editor/feature_inventory/README.md)
  — the placement, arrange, loadout, zone and ORBAT features these operations serve.
