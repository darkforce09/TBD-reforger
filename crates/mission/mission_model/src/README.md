# Mission model source

The source of `mission_model`: the compiled rows of a [mission](/documentation/glossary/g_to_m.md#mission)
document, the [ORBAT](/documentation/glossary/n_to_z.md#orbat) projection of the editor graph, the
optional blocks a mission maker writes beside the ORBAT and the map (the radio plan, the win rule,
tasks, the weather timeline, audio, spawn modules and tactical graphics), the plain-text slot line
and the ids all of them name. A module per block types and checks it, and `authored_blocks/` lists
every block and moves them from the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s
document to the saved payload and on to the compiled document.

## Contents

```text
crates/mission/mission_model/src/
├── authored_blocks/    the block list, its copy onto the saved payload, the compile's two readers
├── compiled/           the compiled rows: entities, vehicles, slots, ORBAT groups, nets, mission sections
├── environment/        the weather timeline and audio blocks
├── error.rs            `Error` and `Result`: the refusal sentence of every check
├── ids.rs              the newtype ids: mission, template, preset, zone, net, task, trigger…
├── lib.rs              the crate root: module header, `mod` lines and the error re-exports
├── objectives/         the tasks and win rule blocks
├── orbat/              the ORBAT squad and slot templates, derived from the editor graph
├── prelude.rs          the error, every id, the compiled rows and the block registry for glob import
├── radio_plan/         the `radioPlan` block: radio nets and their frequencies
├── slot_line/          the plain-text slot line of the ORBAT manager
├── spawn_modules/      the `spawnModules` block: AI waves and garrisons
└── tactical_graphics/  the `tacticalGraphics` block: multi-point control measures on the map
```

## How it works

The blocks, in the order of `AUTHORED_BLOCKS`:

| Block | Module | In the compiled document | Read in the game by |
|---|---|---|---|
| `radioPlan` | `radio_plan` | a typed field, built from the parse | `TBD_RadioPlan` |
| `winConditions` | `objectives::win_conditions` | a typed field, built from the parse | `TBD_WinConditionEvaluator` |
| `tasks` | `objectives::tasks` | carried verbatim | `TBD_TaskStateMachine`, `TBD_TaskHud` |
| `weatherTimeline` | `environment::weather` | carried verbatim | `TBD_WeatherRuntime` |
| `audio` | `environment::audio` | carried verbatim | `TBD_AudioEmitter` |
| `spawnModules` | `spawn_modules` | carried verbatim | `TBD_DynamicSpawner` |
| `tacticalGraphics` | `tactical_graphics` | carried verbatim | nothing |

The Mission Creator writes each block into its document's environment bag, `meta.environment`,
and checks each edit with the block's `validate`. On save, `mission_payload::compile_payload`
copies every listed block onto the payload root unchecked, so work in progress survives, and an
unlisted key stays in the bag. On compile, each block is checked again: the two document-owned
blocks are parsed into the document's typed fields, the other five are carried verbatim to its
root, and a refused block is dropped with a warning finding, which keeps the
[API](/documentation/glossary/a_to_f.md#api) from making an
[artifact](/documentation/glossary/a_to_f.md#artifact) of that version. `authored_blocks/` holds
that path in detail.

Every block module has one shape: `parse` types the block from a `serde_json::Value` and answers
the first problem as an `Error::Refused` sentence naming its path, `validate` is `parse` with the
value dropped (the `fn(&Value) -> Result<()>` a row of `AUTHORED_BLOCKS` holds), and public
constants carry the vocabularies and limits the Mission Creator's panels import instead of
restating. Every function is pure, and nothing here keeps state. A mission that authors none of
the blocks compiles to a document that spends not one byte on them: an absent or `null` block
copies nothing, and an empty carrier emits nothing.

The compiled rows in `compiled/` and the parsed blocks name their ids through `ids.rs`: one
serde-transparent type per referent, and every serialised shape stays the bare string. Slot rows
name `orbat_slot_ids` directly, so a slot's durable identity (`SlotUid`, which seats, squad
leaders and the VIP rule reference) cannot be passed where its derived wire id (`SlotId`) belongs.

### Adding a block

1. A module with `parse` and `validate` in this folder, declared in `lib.rs`, and the block's
   definition in `contracts/definitions/mission.schema.json`.
2. A row in `AUTHORED_BLOCKS`, plus an entry in `DOCUMENT_OWNED_BLOCKS` and a parse arm in
   `AuthoredBlocks::parse` only when the compiled document gets a typed field for the block.
3. A named field and an `authored_block_value` arm in `EditorPayload`
   (`crates/mission/mission_compiler/src/authoring.rs`); without them the compile drops
   the block without a word, which `every_authored_block_key_reaches_the_wire` catches.
4. A round trip through the payload compiler in
   `crates/mission/mission_payload/src/tests/extension_round_trips/`.

The copy onto the payload and the carrier need no change; the tests that pin the list at seven
rows change with it.

## Boundaries

- Depends on: `newtype_ids`, `serde`, `serde_json` and `thiserror`.
- Used by: every consumer through the crate root's public modules (see the crate
  [README](/crates/mission/mission_model/README.md)).
- Rules: the modules reach each other through `crate::` paths only; the payload compiler is never
  named here, so the round trips that need it live in `mission_payload`.
