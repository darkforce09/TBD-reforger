# Environment blocks

The authored blocks that change a [mission](/documentation_v2/glossary.md#mission)'s surroundings
while it runs: the weather timeline, and the audio emitters and music cues. Each child types and
checks one block, and `apps/website/map-engine/src/data/scenario/mod.rs` exposes them as
`data::scenario::weather` and `data::scenario::audio`.

## Contents

```text
apps/website/map-engine/src/data/scenario/extensions/environment/
├── audio/    the `audio` block: positional sound emitters and music cues
├── mod.rs    the module tree
└── weather/  the `weatherTimeline` block: weather keyframes once the round is live
```

## How it works

Both children have the shape every block module of `crate::data::scenario::extensions` has:
`parse` types the block from a `serde_json::Value` and answers the first problem as a sentence
naming its path, and `validate`, `parse` with the value dropped, is the check the block's row in
`AUTHORED_BLOCKS` holds. Both blocks are carried: the compile copies a valid one verbatim to the
compiled document's root, where `TBD_WeatherRuntime` and `TBD_AudioEmitter` in the
[mod](/documentation_v2/glossary.md#mod) read it.

The static environment a mission starts with (its `time` and `weather`, then wind direction, fog,
wind and view distance) lives in the same environment bag of the
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s document but is not an authored
block: the compiler reads it itself into the compiled `meta` and `environment`, in
`apps/website/map-engine/src/data/scenario/compiler/flatten/environment.rs`.

## Public surface

- `audio`, exposed as `data::scenario::audio`: `validate`, for the `audio` row of
  `AUTHORED_BLOCKS` and the Mission Creator's audio panel, and `MUSIC_EVENTS`, the panel's cue list.
- `weather`, exposed as `data::scenario::weather`: `validate`, for the `weatherTimeline` row and
  the Mission Creator's weather timeline panel, and `WEATHER_PRESETS`, the panel's preset list.

## Boundaries

- Depends on: `serde_json`.
- Used by: `crate::data::scenario::extensions`, whose `audio` and `weatherTimeline` rows call the
  two `validate` functions; the Mission Creator's `audio_emitters.rs` and `weather_timeline.rs`
  panels in `apps/website/frontend/src/v2/apps/editor/ui/inspector/`.
- Rules: both blocks stay carried rather than document-owned
  (`win_conditions_is_the_registered_block_and_the_document_models_it` in
  `apps/website/map-engine/src/data/scenario/extensions/authored/tests/cases_1.rs`), and a mission
  that authors neither compiles with neither key
  (`an_unauthored_payload_still_omits_the_audio_key` and
  `an_unauthored_payload_still_omits_the_weather_timeline_key` in the children's tests).

## Related documentation

- [Mission schema](/contracts_v2/definitions/mission.schema.json) — `$defs/weatherTimeline` and
  `$defs/audio`, the two blocks' shapes.
