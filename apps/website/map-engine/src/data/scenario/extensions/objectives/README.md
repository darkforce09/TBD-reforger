# Objective blocks

The authored blocks that say what a [mission](/documentation_v2/glossary.md#mission)'s sides work
towards and how its round ends: the tasks with their state machine, and the win rule. Each child
types and checks one block, and `apps/website/map-engine/src/data/scenario/mod.rs` exposes them as
`data::scenario::tasks` and `data::scenario::win_conditions`.

## Contents

```text
apps/website/map-engine/src/data/scenario/extensions/objectives/
├── mod.rs           the module tree
├── tasks/           the `tasks` block: assignments with their tiers, states and timed windows
└── win_conditions/  the `winConditions` block: the win rule, its end triggers and its parameter
```

## How it works

Both children have the shape every block module of `crate::data::scenario::extensions` has:
`parse` types the block from a `serde_json::Value` and answers the first problem as a sentence
naming its path, and `validate`, `parse` with the value dropped, is the check the block's row in
`AUTHORED_BLOCKS` holds. The two reach the compiled document differently. `tasks` is carried: the
compile copies a valid block verbatim to the document's root. `winConditions` is document-owned:
the compiler builds the compiled rule from the parsed value and derives an `attrition` rule when
the block is absent or refused, so every compiled document has one.

The blocks stay apart as the schema keeps them apart: a task watches a trigger and never ends the
round, while the win rule's `endOn` lists the only triggers that may end it. In the
[mod](/documentation_v2/glossary.md#mod), `TBD_TaskStateMachine` and `TBD_TaskHud` read the tasks,
and `TBD_WinConditionEvaluator` evaluates the rule.

## Public surface

- `tasks`, exposed as `data::scenario::tasks`: `validate`, for the `tasks` row of
  `AUTHORED_BLOCKS` and the [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s
  tasks panel, and `validate_schedule`, `TIERS` and `STATES`, which the panel also imports.
- `win_conditions`, exposed as `data::scenario::win_conditions`: `parse`, `validate`,
  `AUTHORED_MODES`, `END_ON_TRIGGERS`, `FALLBACK_TRIGGER`, the timeout limits, the parameter maps,
  and `AuthoredWinConditions` with its `WinConditionParams`; the compiler reads the parsed rule,
  and the Mission Creator's win conditions card builds its controls from the vocabularies.

## Boundaries

- Depends on: `serde` and `serde_json`.
- Used by: `crate::data::scenario::extensions`, whose rows call both `validate` functions and whose
  `AuthoredBlocks::parse` parses the win rule; `crate::data::scenario::flatten` and
  `crate::data::scenario::ast`, which build and hold the compiled rule; the Mission Creator's
  `tasks_panel.rs` and `win_conditions_card.rs` in
  `apps/website/frontend/src/v2/apps/editor/ui/inspector/`.
- Rules: `winConditions` is document-owned and `tasks` is carried
  (`win_conditions_is_the_registered_block_and_the_document_models_it` in
  `apps/website/map-engine/src/data/scenario/extensions/authored/tests/cases_1.rs`,
  `tasks_is_registered_and_not_document_modelled` in `tasks/tests/cases_1.rs`).

## Related documentation

- [Mission schema](/contracts_v2/definitions/mission.schema.json) — `tasks`, `$defs/task` and
  `$defs/winConditions`, the two blocks' shapes.
