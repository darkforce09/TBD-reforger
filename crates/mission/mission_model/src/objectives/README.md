# Objective blocks

The authored blocks that say what a [mission](/documentation/glossary/g_to_m.md#mission)'s sides work
towards and how its round ends: the tasks with their state machine, and the win rule. Each child
types and checks one block: `mission_model::objectives::tasks` and
`mission_model::objectives::win_conditions`.

## Contents

```text
crates/mission/mission_model/src/objectives/
├── mod.rs           the module tree
├── tasks/           the `tasks` block: assignments with their tiers, states and timed windows
└── win_conditions/  the `winConditions` block: the win rule, its end triggers and its parameter
```

## How it works

Both children have the shape every block module of `crate::authored_blocks` has:
`parse` types the block from a `serde_json::Value` and answers the first problem as a sentence
naming its path, and `validate`, `parse` with the value dropped, is the check the block's row in
`AUTHORED_BLOCKS` holds. The two reach the compiled document differently. `tasks` is carried: the
compile copies a valid block verbatim to the document's root. `winConditions` is document-owned:
the compiler builds the compiled rule from the parsed value and derives an `attrition` rule when
the block is absent or refused, so every compiled document has one.

The blocks stay apart as the schema keeps them apart: a task watches a trigger and never ends the
round, while the win rule's `endOn` lists the only triggers that may end it. In the
[mod](/documentation/glossary/g_to_m.md#mod), `TBD_TaskStateMachine` and `TBD_TaskHud` read the tasks,
and `TBD_WinConditionEvaluator` evaluates the rule.

## Public surface

- `tasks`, exposed as `mission_model::objectives::tasks`: `validate`, for the `tasks` row of
  `AUTHORED_BLOCKS` and the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s
  tasks panel, and `validate_schedule`, `TIERS` and `STATES`, which the panel also imports.
- `win_conditions`, exposed as `mission_model::objectives::win_conditions`: `parse`, `validate`,
  `AUTHORED_MODES`, `END_ON_TRIGGERS`, `FALLBACK_TRIGGER`, the timeout limits, the parameter maps,
  and `AuthoredWinConditions` with its `WinConditionParams`; the compiler reads the parsed rule,
  and the Mission Creator's win conditions card builds its controls from the vocabularies.

## Boundaries

- Depends on: `serde` and `serde_json`.
- Used by: `crate::authored_blocks`, whose rows call both `validate` functions and whose
  `AuthoredBlocks::parse` parses the win rule; `mission_compiler` and
  `crate::compiled`, which build and hold the compiled rule; the Mission Creator's
  `tasks_panel.rs` and `win_conditions_card.rs` in
  `crates/frontend/workspaces/mission_creator_workspace/src/ui/inspector/`.
- Rules: `winConditions` is document-owned and `tasks` is carried
  (`win_conditions_is_the_registered_block_and_the_document_models_it` in
  `crates/mission/mission_model/src/authored_blocks/tests/cases_1.rs`).

## Related documentation

- [Mission schema](/contracts/definitions/mission.schema.json) — `tasks`, `$defs/task` and
  `$defs/winConditions`, the two blocks' shapes.
