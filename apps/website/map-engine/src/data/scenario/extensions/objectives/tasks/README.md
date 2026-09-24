# Authored tasks

The check on a [mission](/documentation_v2/glossary.md#mission)'s authored `tasks` block: the
primary, secondary and optional assignments the game's HUD and briefing list, each starting
`assigned` and ending `succeeded` or `failed`, with the transition table between those states and
the rules for a timed window. The module is exposed as `data::scenario::tasks`.

## Contents

```text
apps/website/map-engine/src/data/scenario/extensions/objectives/tasks/
├── hierarchy.rs  `parse` and `validate`; tiers, states, the transition table and schedule checks
├── mod.rs        the module tree; re-exports the task types, the vocabularies and the checks
└── tests/        unit tests for the parse, the transition table, schedules and the carrier
```

## How it works

`parse` reads an array of tasks as `contracts_v2/definitions/mission.schema.json` shapes each one in
`$defs/task`: `{id, title, tier, state, triggerId?, markerId?, description?, schedule?}`, with
`tier` one of `TIERS` (`primary`, `secondary`, `optional`), `state` one of `STATES` (`assigned`,
`succeeded`, `failed`), every string non-blank once trimmed and every id unique. A key the schema
does not declare refuses the block, and the first problem found is the answer, a sentence with its
path. Unlike the other list blocks, an empty array passes, as the schema allows; the
[Mission Creator](/documentation_v2/glossary.md#mission-creator) clears the block with `null`
instead.

A `schedule` is `{startAfterS, windowS}` in whole seconds from mission start. `validate_schedule`
wants `windowS` above zero and `startAfterS` at zero or more and, when it is given the mission
length, before that length, where a length of 0 means the round has no limit. The block's own
check passes no length, because `flow.timeLimitSeconds` lies outside the block; the Mission
Creator's tasks panel passes it.

`TaskState` and `TaskTier` are the typed vocabularies. `LEGAL_TRANSITIONS` holds the only two
moves, `assigned` to `succeeded` and `assigned` to `failed`, which `is_legal_transition` and
`transition` apply. `validate`, `parse` with the value dropped, is the check of the `tasks` row of
`AUTHORED_BLOCKS` in `crate::data::scenario::extensions`, and the compile carries a valid block
verbatim to the compiled document's root. In the game,
`apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/TBD_TaskStateMachine.c` runs the
same table on the server and `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Hud/TBD_TaskHud.c` marks
each assigned task.

## Boundaries

- Depends on: `serde` (the derives of `TaskState` and `TaskTier`) and `serde_json`.
- Used by: `crate::data::scenario::extensions`, whose `tasks` row calls `validate`; the Mission
  Creator's tasks panel (`apps/website/frontend/src/v2/apps/editor/ui/inspector/tasks_panel.rs`),
  which offers `TIERS` and `STATES`, checks each edit with `validate`, and checks each schedule
  with `validate_schedule` against `flow.timeLimitSeconds`.
- Rules: only the two moves out of `assigned` are legal, as in the game's table
  (`every_other_pair_is_illegal` in `tests/cases_1.rs`); a window of zero or a negative start is
  refused, and a start at or past the mission length only when a length is given
  (`a_zero_window_is_refused`, `a_negative_start_is_refused`,
  `start_at_or_past_mission_length_is_refused`); a blank optional string is refused rather than
  stored (`a_blank_optional_is_refused_rather_than_stored`).

## Related documentation

- [Mission schema](/contracts_v2/definitions/mission.schema.json) — `$defs/task` and
  `$defs/taskSchedule`, the shape this module checks.
