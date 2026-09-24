# Authored radio plan

The check on a [mission](/documentation_v2/glossary.md#mission)'s authored `radioPlan` block: the
radio nets, each with an id, a label and a frequency, and optionally a side and a radio class. The
compiled document has a typed field for the plan, so the compiler reads the parsed value rather
than carrying the author's JSON. The module is exposed as `data::scenario::radio_plan`.

## Contents

```text
apps/website/map-engine/src/data/scenario/extensions/radio/
├── mod.rs   the module tree; re-exports the plan types, the limits, `freq_key` and the checks
├── nets.rs  `parse` and `validate`; the band, the net and label limits, duplicate frequencies
└── tests/   unit tests for the parse, each refusal and the block's way onto the payload root
```

## How it works

`parse` reads `{nets: [...]}` as `contracts_v2/definitions/mission.schema.json` shapes it in
`$defs/radioPlan` and `$defs/net`: 1 to `MAX_NETS` (32) nets, each `{id, label, freqMHz, faction?,
range?}`. The id is `net:` followed by lowercase letters, digits and underscores, and is unique;
the label runs 1 to `MAX_LABEL_CHARS` (48) characters; `freqMHz` lies within `FREQ_MIN_MHZ` and
`FREQ_MAX_MHZ` (30 to 512); `faction` is a lowercase faction key and `range` one of `RANGES`
(`short`, `long`). Strings are trimmed and must not be blank, and a key the schema does not declare
refuses the block. No two nets share a frequency: `freq_key` compares frequencies at 1 kHz, so `30`
and `30.0` collide while `30.0` and `30.5` do not, and `refuse_duplicate_frequency` names the
earlier net. The net and label limits are where the [mod](/documentation_v2/glossary.md#mod)
truncates, so a plan that passes is the plan the game runs.

`validate`, `parse` with the value dropped, is the check of the `radioPlan` row of
`AUTHORED_BLOCKS` in `crate::data::scenario::extensions`, which lists this block among the
document-owned ones and hands the parsed `AuthoredRadioPlan` to the compiler. The compiler writes
those nets into the compiled plan and, when no valid plan is authored, derives one from the
[ORBAT](/documentation_v2/glossary.md#orbat). In the game,
`apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Radio/TBD_RadioPlan.c` loads the nets.

## Boundaries

- Depends on: `serde_json`.
- Used by: `crate::data::scenario::extensions`, whose `radioPlan` row calls `validate` and whose
  `AuthoredBlocks::parse` calls `parse`; `crate::data::scenario::flatten`, whose
  `apps/website/map-engine/src/data/scenario/compiler/flatten/radio.rs` turns `AuthoredRadioPlan`
  into the compiled plan; the [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s
  radio panel (`apps/website/frontend/src/v2/apps/editor/ui/inspector/radio_panel.rs`), which
  bounds its fields by the limits and `RANGES`, picks and compares frequencies with `freq_key`,
  and checks each edit with `validate`.
- Rules: a frequency another net already uses is refused (`a_duplicate_frequency_is_refused` in
  `tests/cases_1.rs`), as are a frequency outside the band (`an_out_of_range_frequency_is_refused`)
  and more than 32 nets (`more_than_max_nets_is_refused`); `MAX_NETS` and `MAX_LABEL_CHARS` equal
  the schema's `maxItems` and `maxLength` and the `TBD_RadioPlan` constants of the mod; the carrier
  never emits the plan, which the document models itself
  (`radio_plan_is_registered_and_document_modelled`).

## Related documentation

- [Mission schema](/contracts_v2/definitions/mission.schema.json) — `$defs/radioPlan` and
  `$defs/net`, the shape this module checks.
- [Voice bridge contract](/documentation_v2/contracts_v2/definitions/bridge_messages.md) — how the
  plan's nets map to voice channels.
