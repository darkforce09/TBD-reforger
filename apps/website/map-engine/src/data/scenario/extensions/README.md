# Authored mission blocks

The optional blocks a [mission](/documentation_v2/glossary.md#mission) maker writes beside the
[ORBAT](/documentation_v2/glossary.md#orbat) and the map: the radio plan, the win rule, tasks, the
weather timeline, audio, spawn modules and tactical graphics. A module per block types and checks
it, and `authored/` lists every block and moves them from the
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s document to the saved payload
and on to the compiled document.

## Contents

```text
apps/website/map-engine/src/data/scenario/extensions/
├── authored/           the block list, its copy onto the saved payload, the compile's two readers
├── environment/        the weather timeline and audio blocks
├── mod.rs              the module tree; re-exports every item of `authored`
├── modules/            the `spawnModules` block: AI waves and garrisons
├── objectives/         the tasks and win rule blocks
├── radio/              the `radioPlan` block: radio nets and their frequencies
└── tactical_graphics/  the `tacticalGraphics` block: multi-point control measures on the map
```

## How it works

The blocks, in the order of `AUTHORED_BLOCKS`, with the alias
`apps/website/map-engine/src/data/scenario/mod.rs` gives each module:

| Block | Module, as `data::scenario::…` | In the compiled document | Read in the game by |
|---|---|---|---|
| `radioPlan` | `radio/`, `radio_plan` | a typed field, built from the parse | `TBD_RadioPlan` |
| `winConditions` | `objectives/win_conditions/`, `win_conditions` | a typed field, built from the parse | `TBD_WinConditionEvaluator` |
| `tasks` | `objectives/tasks/`, `tasks` | carried verbatim | `TBD_TaskStateMachine`, `TBD_TaskHud` |
| `weatherTimeline` | `environment/weather/`, `weather` | carried verbatim | `TBD_WeatherRuntime` |
| `audio` | `environment/audio/`, `audio` | carried verbatim | `TBD_AudioEmitter` |
| `spawnModules` | `modules/`, `spawn_modules` | carried verbatim | `TBD_DynamicSpawner` |
| `tacticalGraphics` | `tactical_graphics/`, `tactical_graphics` | carried verbatim | nothing |

The Mission Creator writes each block into its document's environment bag, `meta.environment`,
and checks each edit with the block's `validate`. On save, `compile_payload` copies every listed
block onto the payload root unchecked, so work in progress survives, and an unlisted key stays in
the bag. On compile, each block is checked again: the two document-owned blocks are parsed into
the document's typed fields, the other five are carried verbatim to its root, and a refused block
is dropped with a warning finding, which keeps the [API](/documentation_v2/glossary.md#api) from
making an [artifact](/documentation_v2/glossary.md#artifact) of that version. `authored/` holds
that path in detail.

Every block module has one shape: `parse` types the block from a `serde_json::Value` and answers
the first problem as a sentence naming its path, `validate` is `parse` with the value dropped (the
`fn(&Value) -> Result<(), String>` a row of `AUTHORED_BLOCKS` holds), and public constants carry
the vocabularies and limits the Mission Creator's panels import instead of restating. Every
function is pure, and nothing here keeps state. A mission that authors none of the blocks compiles
to a document that spends not one byte on them: an absent or `null` block copies nothing, and an
empty carrier emits nothing.

### Adding a block

1. A module with `parse` and `validate` in this folder, given an alias in
   `apps/website/map-engine/src/data/scenario/mod.rs`, and the block's definition in
   `contracts_v2/definitions/mission.schema.json`.
2. A row in `AUTHORED_BLOCKS`, plus an entry in `DOCUMENT_OWNED_BLOCKS` and a parse arm in
   `AuthoredBlocks::parse` only when the compiled document gets a typed field for the block.
3. A named field and an `authored_block_value` arm in `EditorPayload`
   (`apps/website/map-engine/src/data/scenario/ast/authoring.rs`); without them the compile drops
   the block without a word, which `every_authored_block_key_reaches_the_wire` catches.

The copy onto the payload and the carrier need no change; the tests that pin the list at seven
rows change with it.

## Public surface

- `data::scenario::extensions`, the items of `authored/`: `AUTHORED_BLOCKS` for the payload
  parser, `copy_authored_blocks` and `is_authored_block` for the payload builder, and
  `AuthoredBlocks` and `ExtensionBlocks` for the compiler; no code outside the crate imports them.
- The block modules, under the aliases in the table: each block's `validate` and vocabularies for
  the Mission Creator's inspector panels, the typed win rule and radio plan for the compiler, and
  the tactical graphic floor and cap for the draw tool in `crate::data::store::operations`.

## Boundaries

- Depends on: `serde` and `serde_json`, and no code of the crate outside this folder, whose modules
  `authored/` reaches through their `data::scenario` aliases; the tests also call
  `crate::data::scenario::compile::compile_payload`.
- Used by:
  - inside `data::scenario`: `compile` (the copy onto the payload root), `ast` (the list the
    payload parser walks, and `WinConditionParams`), and `flatten` (both readers, the carrier, the
    typed win rule and radio plan);
  - `crate::data::store::operations`, whose tactical graphics draw tool takes `min_points` and
    `MAX_POINTS`;
  - the Mission Creator's inspector panels in
    `apps/website/frontend/src/v2/apps/editor/ui/inspector/` (`audio_emitters.rs`,
    `radio_panel.rs`, `spawn_modules.rs`, `tasks_panel.rs`, `weather_timeline.rs`,
    `win_conditions_card.rs` and the zones panel), with their tests and the
    bridge tests in `apps/website/frontend/src/v2/apps/editor/bridge/tests/tactical_graphics/`;
  - the API, only through the compiler in `crate::data::scenario::flatten`.
- Rules:
  - every listed block reaches the compiled document
    (`every_authored_block_key_reaches_the_wire` in
    `apps/website/map-engine/src/data/scenario/compiler/flatten/tests/cases_3.rs`), and the list
    holds exactly seven (`tactical_graphics_registered_here_must_also_be_readable_by_flatten` in
    `tactical_graphics/tests/cases_1.rs`);
  - an unlisted environment key never reaches the payload root
    (`copy_carries_a_listed_key_verbatim_and_leaves_everything_else` in
    `authored/tests/cases_1.rs`, and `an_unlisted_environment_key_is_not_promoted` in the tests of
    `objectives/tasks/`, `modules/` and `tactical_graphics/`);
  - a mission that authors no block gains no key (`an_empty_carrier_adds_nothing_to_the_document`,
    and each carried block's `an_unauthored_payload_still_omits_*` test).

## Related documentation

- [Mission schema](/contracts_v2/definitions/mission.schema.json) — the seven blocks' definitions
  in the compiled document.
- [Mission editor payload schema](/contracts_v2/definitions/mission-editor-payload.schema.json) —
  the saved payload, whose open root carries the blocks.
