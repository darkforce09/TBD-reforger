# Mission model

The `mission_model` crate: what a compiled [mission](/documentation/glossary/g_to_m.md#mission)
document holds and what a mission maker may author beside the
[ORBAT](/documentation/glossary/n_to_z.md#orbat) and the map. It holds the compiled rows the
[mod](/documentation/glossary/g_to_m.md#mod) loads, the ORBAT templates the
[API](/documentation/glossary/a_to_f.md#api) reads, the seven authored extension blocks with their
parse and validate rules, the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s
plain-text slot line, and the newtype ids every one of them names.

## Contents

```text
crates/mission/mission_model/
├── Cargo.toml  the package: `newtype_ids`, `serde`, `serde_json`, `thiserror`; layout tier 1
└── src/        the compiled rows, the ORBAT projection, the authored blocks, the slot line, the ids
```

## How it works

`compiled` holds the `Mod*` rows of the compiled document, serialise-only, each projecting a
`$defs` entry of `contracts/definitions/mission.schema.json`. `orbat` reads a saved payload's ORBAT
(an explicit `orbat` array, else derived from the editor graph) and checks a faction join key.
`authored_blocks` lists the seven optional blocks in compile order and carries them from the
editor's environment bag to the payload root and on to the compiled document; each block module
(`radio_plan`, `objectives::{tasks, win_conditions}`, `environment::{weather, audio}`,
`spawn_modules`, `tactical_graphics`) parses its block into typed rows and answers the first
problem as one readable sentence, an `Error::Refused`. `ids` declares one serde-transparent newtype
per referent, so every JSON shape, stored payload, API golden and artifact digest stays
byte-identical while a slot's durable `SlotUid` and its derived `SlotId` cannot be confused.

The [source README](/crates/mission/mission_model/src/README.md) has the block table and the steps
for adding a block.

## Getting started

Run from the repository root:

```bash
cargo test -p mission_model    # the ORBAT projection, the slot line and every block's parse and validate
cargo clippy -p mission_model --all-targets -- -D warnings
```

## Configuration

None: no features and no environment variables.

## Public surface

The modules, named from the crate root (the prelude also carries the error, every id, the compiled
rows and the block registry):

- `compiled::{entities, mission}`: `ModEntity`, `ModVehicle`, `ModVehicleSeat`, `ModSlot`,
  `ModSlotLoadout`, `ModSlotGear` (with `is_empty`), `ModSlotCargo`, `ModOrbatFaction`,
  `ModOrbatGroup`, `ModOrbatRole`, `ModNet`; `ModMeta`, `ModFaction`, `ModZone`, `ModZoneShape`,
  `ModCircle`, `ModEnvironment`, `ModFlow`, `ModWinConditions`, `ModSettings`, `ModMarker`,
  `ModBriefing`, `ModRadioPlan`.
- `orbat`: `OrbatSquadTemplate`, `OrbatSlotTemplate`, `parse_orbat_template`,
  `derive_orbat_from_editor`, `validate_faction_join_key`.
- `authored_blocks`: `AUTHORED_BLOCKS`, `DOCUMENT_OWNED_BLOCKS`, `AuthoredBlock`, `AuthoredBlocks`,
  `ExtensionBlocks`, `copy_authored_blocks`, `is_authored_block`.
- the block modules' `parse`, `validate`, typed rows and vocabulary constants (`KINDS`,
  `MAX_POINTS`, `FREQ_MIN_MHZ`, `AUTHORED_MODES`, `WEATHER_PRESETS`, `MUSIC_EVENTS`, …).
- `slot_line::format_slot_line`.
- `ids`: `MissionId`, `MissionTemplateId`, `FactionPresetId`, `ZoneId`, `SlotId`, `SlotUid`,
  `NetId`, `TaskId`, `TriggerId`, `MarkerId`, `SpawnModuleId`, `AudioEmitterId`, `MusicCueId`,
  `TacticalGraphicId`.
- `Error` (`Refused`, `FactionJoinKeyMissing`, `FactionJoinKeyPadded`) and `Result`.

## Boundaries

- Depends on: `newtype_ids` (foundation), `serde`, `serde_json` (`preserve_order`) and `thiserror`.
- Used by:
  - `mission_payload`, which copies the authored blocks onto the payload root and derives the
    export payload's ORBAT;
  - `mission_compiler`: the game-document compiler
    (`crates/mission/mission_compiler/src/game_document/`), which builds the compiled rows and
    reads both block carriers, and the editor input structs
    (`crates/mission/mission_compiler/src/authoring.rs`);
  - `mission_operations`, whose tactical graphics draw tool reads the kinds and point limits;
  - the API's operations domain (`validate_faction_join_key`, the ORBAT templates);
  - the Mission Creator's inspector panels and ORBAT manager
    (`apps/frontend/src/workspaces/editor/ui/`).
- Rules: mission tier 1, so the crate depends on no compiler and no map engine
  (`cargo xtask verify crate-tiers`); no primitive public id field
  (`cargo xtask verify crate-anatomy`); the round trips through the payload compiler live in
  `mission_payload`, never as a dev-dependency here.

## Related documentation

- [Mission crates](/crates/mission/README.md) — the category and its dependency rule.
- [Mission schema](/contracts/definitions/mission.schema.json) — the compiled document's
  definitions the rows and blocks project.
- [Mission editor payload schema](/contracts/definitions/mission-editor-payload.schema.json) —
  the saved payload, whose open root carries the blocks.
