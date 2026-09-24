# Weather timeline

The check on a [mission](/documentation_v2/glossary.md#mission)'s authored `weatherTimeline`
block: the weather keyframes a round moves through once it goes live, each a preset at a minute
offset with an optional wind direction and fog density. The weather a mission starts with is the
separate `environment.weatherPreset`. The module is exposed as `data::scenario::weather`.

## Contents

```text
apps/website/map-engine/src/data/scenario/extensions/environment/weather/
├── mod.rs       the module tree; re-exports the timeline types, `WEATHER_PRESETS` and the checks
├── tests/       unit tests for the parse, each refusal and the block's way onto the payload root
└── timeline.rs  `parse` and `validate` for the block; `WEATHER_PRESETS` and the keyframe row
```

## How it works

`parse` reads the block as `contracts_v2/definitions/mission.schema.json` shapes it in
`$defs/weatherTimeline`: an object whose only key, `keyframes`, holds one keyframe or more. A
keyframe is `{atMinutes, weatherPreset, windDirDeg?, fog?}`: `atMinutes` counts whole minutes from
the moment the round goes live, from 0 up (a whole-valued number such as `15.0` counts);
`weatherPreset` is one of `WEATHER_PRESETS` (`clear`, `overcast`, `heavy_rain`, `dense_fog`);
`windDirDeg` lies within 0 to 360 and `fog` within 0 to 1, both bounds included. Each keyframe's
`atMinutes` is above the one before it (`minutes_strictly_increase`), a rule the schema leaves to
this check. A key the schema does not declare refuses the block, and the first problem found is
the answer, a sentence with its path, such as "`weatherTimeline.keyframes[1]`.atMinutes is 0 —
atMinutes must be strictly increasing (previous was 0)".

`validate` is `parse` with the value dropped: the check of the `weatherTimeline` row of
`AUTHORED_BLOCKS` in `crate::data::scenario::extensions`. The compile carries a valid block
verbatim to the compiled document's root, and in the game
`apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Ingestion/TBD_WeatherRuntime.c` applies
each keyframe at its offset.

## Boundaries

- Depends on: `serde_json`.
- Used by: `crate::data::scenario::extensions`, whose `weatherTimeline` row calls `validate`; the
  [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s weather timeline panel
  (`apps/website/frontend/src/v2/apps/editor/ui/inspector/weather_timeline.rs`), which checks each
  edit with `validate` and offers `WEATHER_PRESETS`.
- Rules: an offset equal to or below the one before it is refused (`equal_at_minutes_are_refused`
  and `out_of_order_at_minutes_are_refused` in `tests/cases_1.rs`); `WEATHER_PRESETS` is the
  vocabulary of `environment.weatherPreset` (`the_preset_vocabulary_matches_environment` pins it),
  and the compiler keeps its own copy in
  `apps/website/map-engine/src/data/scenario/compiler/flatten/environment.rs`, so a new preset goes
  into both; a mission that authors no timeline compiles with no `weatherTimeline` key
  (`an_unauthored_payload_still_omits_the_weather_timeline_key`).

## Related documentation

- [Mission schema](/contracts_v2/definitions/mission.schema.json) — `$defs/weatherTimeline` and
  `$defs/weatherKeyframe`, the shape this module checks.
