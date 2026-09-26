# Authored audio

The check on a [mission](/documentation_v2/glossary/g_to_m.md#mission)'s authored `audio` block: the
positional sound emitters and the music cues the author places, typed from JSON and refused with a
readable sentence when the [mod](/documentation_v2/glossary/g_to_m.md#mod) could not play them as
written. The module is exposed as `data::scenario::audio`.

## Contents

```text
apps/website/map-engine/src/data/scenario/extensions/environment/audio/
├── emitters.rs  `parse` and `validate` for the block; `MUSIC_EVENTS` and the emitter and cue rows
├── mod.rs       the module tree; re-exports the block types, `MUSIC_EVENTS` and the checks
└── tests/       unit tests for the parse, each refusal and the block's way onto the payload root
```

## How it works

`parse` reads the block as `contracts_v2/definitions/mission.schema.json` shapes it in
`$defs/audio`: an object with exactly the keys `emitters` and `musicCues`, both arrays, not both
empty, since an empty block is left out rather than stored. An emitter is
`{id, x, z, y?, sound, radiusM, loop, triggerId?}` with non-empty strings, finite coordinates, a
boolean `loop` and a `radiusM` above zero (`radius_above_zero`). A cue is `{id, event, track}`,
where `event` is one of `MUSIC_EVENTS`: `mission_start`, `task_succeeded`, `task_failed` and
`mission_end`. Ids are unique across both arrays, and a key the schema does not declare refuses
the block. The first problem found is the answer, a sentence with its path, such as
"`audio.emitters[0]`.radiusM is 0 — radiusM must be above zero".

`validate` is `parse` with the value dropped: the check of the `audio` row of `AUTHORED_BLOCKS`
in `crate::data::scenario::extensions`. The compile carries a valid block verbatim to the compiled
document's root, so no compile step reads the typed rows. In the game,
`apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Audio/TBD_AudioEmitter.c` arms the emitters and
plays the cues.

## Boundaries

- Depends on: `serde_json`.
- Used by: `crate::data::scenario::extensions`, whose `audio` row calls `validate`; the
  [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s audio panel
  (`apps/website/frontend/src/v2/apps/editor/ui/inspector/audio_emitters.rs` and its `view.rs`),
  which checks each edit with `validate` and offers `MUSIC_EVENTS` as the cue list.
- Rules: a radius of zero or below is refused, as the schema's `exclusiveMinimum: 0` refuses it
  (`radius_zero_is_refused` and `a_negative_radius_is_refused` in `tests/cases_1.rs`); ids are
  unique across emitters and cues (`a_duplicate_id_is_refused`); a mission that authors no audio
  compiles with no `audio` key (`an_unauthored_payload_still_omits_the_audio_key`);
  `MUSIC_EVENTS` and the key lists mirror `$defs/audio`, `$defs/audioEmitter` and `$defs/musicCue`
  by hand, so a schema change there changes this module in the same commit.

## Related documentation

- [Mission schema](/contracts_v2/definitions/mission.schema.json) — `$defs/audio`,
  `$defs/audioEmitter` and `$defs/musicCue`, the shape this module checks.
