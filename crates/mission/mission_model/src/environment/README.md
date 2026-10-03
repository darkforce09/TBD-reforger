# Environment blocks

The authored blocks that change a [mission](/documentation/glossary/g_to_m.md#mission)'s surroundings
while it runs: the weather timeline, and the audio emitters and music cues. Each child types and
checks one block: `mission_model::environment::weather` and
`mission_model::environment::audio`.

## Contents

```text
crates/mission/mission_model/src/environment/
├── audio/    the `audio` block: positional sound emitters and music cues
├── mod.rs    the module tree
└── weather/  the `weatherTimeline` block: weather keyframes once the round is live
```

## How it works

Both children have the shape every block module of `crate::authored_blocks` has:
`parse` types the block from a `serde_json::Value` and answers the first problem as a sentence
naming its path, and `validate`, `parse` with the value dropped, is the check the block's row in
`AUTHORED_BLOCKS` holds. Both blocks are carried: the compile copies a valid one verbatim to the
compiled document's root, where `TBD_WeatherRuntime` and `TBD_AudioEmitter` in the
[mod](/documentation/glossary/g_to_m.md#mod) read it.

The static environment a mission starts with (its `time` and `weather`, then wind direction, fog,
wind and view distance) lives in the same environment bag of the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s document but is not an authored
block: the compiler reads it itself into the compiled `meta` and `environment`, in
`crates/mission/mission_compiler/src/game_document/environment.rs`.

## Public surface

- `audio`, exposed as `mission_model::environment::audio`: `validate`, for the `audio` row of
  `AUTHORED_BLOCKS` and the Mission Creator's audio panel, and `MUSIC_EVENTS`, the panel's cue list.
- `weather`, exposed as `mission_model::environment::weather`: `validate`, for the `weatherTimeline` row and
  the Mission Creator's weather timeline panel, and `WEATHER_PRESETS`, the panel's preset list.

## Boundaries

- Depends on: `serde_json`.
- Used by: `crate::authored_blocks`, whose `audio` and `weatherTimeline` rows call the
  two `validate` functions; the Mission Creator's `audio_emitters.rs` and `weather_timeline.rs`
  panels in `apps/frontend/src/workspaces/editor/ui/inspector/`.
- Rules: both blocks stay carried rather than document-owned
  (`win_conditions_is_the_registered_block_and_the_document_models_it` in
  `crates/mission/mission_model/src/authored_blocks/tests/cases_1.rs`), and a mission
  that authors neither compiles with neither key
  (`an_unauthored_payload_still_omits_the_audio_key` and
  `an_unauthored_payload_still_omits_the_weather_timeline_key` in
  `crates/mission/mission_payload/src/tests/extension_round_trips/`).

## Related documentation

- [Mission schema](/contracts/definitions/mission.schema.json) — `$defs/weatherTimeline` and
  `$defs/audio`, the two blocks' shapes.
