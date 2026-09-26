# Authored win rule

The check on a [mission](/documentation_v2/glossary/g_to_m.md#mission)'s authored `winConditions` block:
the win rule, the triggers that may end the round, and the one parameter the rule takes. The
compiled document has a typed field for this block, so the compiler reads the parsed value rather
than carrying the author's JSON. The module is exposed as `data::scenario::win_conditions`.

## Contents

```text
apps/website/map-engine/src/data/scenario/extensions/objectives/win_conditions/
├── conditions.rs  `parse` and `validate`; the modes, the end triggers and the per-mode parameters
├── mod.rs         the module tree; re-exports the rule types, the vocabularies, limits and checks
└── tests/         unit tests for each mode and parameter, and the lockstep with both schemas
```

## How it works

`parse` reads `{mode, endOn, extractionZoneId?, vipSlotId?, timeoutMinutes?}`. `mode` is one of
`AUTHORED_MODES` (`attrition`, `objective`, `extraction`, `vip`, `timeout`), the five the
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) authors;
`contracts_v2/definitions/mission.schema.json` admits two more, hand-authored values no editor
payload produces, and this check refuses them. `endOn` holds one or more of `END_ON_TRIGGERS`
(`time_limit`, `all_objectives_captured`, `faction_eliminated`, `objective_destroyed`,
`hold_expired`), kept in the author's order with repeats dropped. Each parameter belongs to one
mode (`param_key_for_mode`): `extraction` needs `extractionZoneId`, `vip` needs `vipSlotId` and may
also carry `extractionZoneId` (`optional_param_keys_for_mode`), and `timeout` needs
`timeoutMinutes` within `TIMEOUT_MINUTES_MIN` and `TIMEOUT_MINUTES_MAX` (1 to 1440). A parameter
on a mode that does not read it is refused, an id is trimmed before it is kept, and any other key
is ignored, since the compiled rule is rebuilt from the typed value. The first problem found is the
answer, a sentence that names the key and the mode.

The result is an `AuthoredWinConditions` holding `WinConditionParams`, which serialise with no key
for an absent parameter. `validate`, `parse` with the value dropped, is the check of the
`winConditions` row of `AUTHORED_BLOCKS` in `crate::data::scenario::extensions`, which lists this
block among the document-owned ones and hands the parsed value to the compiler. The compiler
checks the parameters against the zones and [slots](/documentation_v2/glossary/n_to_z.md#slot) it emitted
and falls back to `FALLBACK_TRIGGER` when no trigger survives; with no valid block it derives an
`attrition` rule. In the game,
`apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Stages/TBD_WinConditionEvaluator.c` evaluates
the rule.

## Boundaries

- Depends on: `serde` (`WinConditionParams` serialises) and `serde_json`.
- Used by: `crate::data::scenario::extensions`, whose `winConditions` row calls `validate` and
  whose `AuthoredBlocks::parse` calls `parse`; `crate::data::scenario::flatten`, which resolves the
  compiled rule from `AuthoredWinConditions` and `FALLBACK_TRIGGER`, and
  `crate::data::scenario::ast`, whose `ModWinConditions` carries `WinConditionParams`; the Mission
  Creator's win conditions card
  (`apps/website/frontend/src/v2/apps/editor/ui/inspector/win_conditions_card.rs` and its
  `view.rs`), which builds its mode, trigger and parameter controls from the vocabularies, the
  parameter maps and the timeout limits, and shows `validate`'s refusal.
- Rules: `AUTHORED_MODES` equals the `mode` enum of
  `contracts_v2/definitions/mission-editor-payload.schema.json` and lies inside the one of
  `mission.schema.json`, and `END_ON_TRIGGERS` equals the latter's `endOn` enum
  (`the_authored_modes_are_the_editor_payload_schema_s_enum` in `tests/cases_1.rs`); the timeout
  limits are the schema's own (`the_timeout_bounds_are_the_schema_s_own`); a parameter of another
  mode is refused (`a_param_belonging_to_another_mode_is_refused`), and every optional parameter
  has its parse branch (`every_optional_param_key_has_a_parse_branch`).

## Related documentation

- [Mission schema](/contracts_v2/definitions/mission.schema.json) — `$defs/winConditions`, the
  compiled rule and the reasons for its seven modes.
- [Mission editor payload schema](/contracts_v2/definitions/mission-editor-payload.schema.json) —
  the five modes a saved payload may carry.
