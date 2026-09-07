# REPORT T-936.5 — Positional audio emitters and music cues

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-936.5
slice/T-936.5
```

First actions matched the brief. EnfusionMCP count was already 19. `export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target` for xtask/schema/mod compile. Tests that RUN a binary used `/home/Samuel/.cache/tbd-target-T-936.5` (shared cache can serve another slice's test binary).

## defect_verified_on_main

The worktree is `slice/T-936.5` at merge-base + T-942 packing. Before any code:

- `crates/map-engine-core/src/mission/audio.rs` — missing
- `apps/website/frontend/src/editor/panels/audio_emitters.rs` — missing
- both `TBD_AudioEmitter.c` — missing
- `mission.schema.json` — no `audio` property
- `AUTHORED_BLOCKS` keys: `radioPlan`, `winConditions`, `tasks`, `weatherTimeline` only

`cargo test -p map-engine-core --all-features an_unlisted_environment_key_is_not_promoted_to_the_payload_root` **ok**: a payload carrying `environment.audio` was not copied to the payload root (`p.get("audio").is_none()`). Missions carried no authored sound.

## changes

| path | why |
|---|---|
| `packages/tbd-schema/schema/mission.schema.json` | Optional top-level `audio` + `$defs/audio` / `audioEmitter` / `musicCue` (`additionalProperties` false, `radiusM` exclusiveMinimum 0, closed cue events). |
| `crates/map-engine-core/src/mission/audio.rs` | NEW. Parse/validate, `MUSIC_EVENTS`, `radius_above_zero` (perturbation target), compile/carrier tests. |
| `crates/map-engine-core/src/mission/mod.rs` | Register `audio`. |
| `crates/map-engine-core/src/mission/extensions.rs` | AUTHORED_BLOCKS row `audio`; `len()==5`; not in `DOCUMENT_OWNED_BLOCKS`. |
| `apps/website/frontend/src/editor/panels/audio_emitters.rs` | NEW. Emitter list + cue table, undoable via `update_environment`, place-on-map via `begin_place_marker`. |
| `apps/website/frontend/src/editor/panels/mod.rs` | Register `audio_emitters`. |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_AudioEmitter.c` | NEW. Server arms emitters (triggerId via T-676) and fires cues; clients spawn one `TBD_AudioSourceEntity` per emitter and honour radius/loop. Presence is array `Count()`. |
| `apps/mod/tbd-export/Scripts/Game/TBD/Gamemode/TBD_AudioEmitter.c` | ASCII twin. |

`flatten.rs` was not edited (T-291 / T-946.44). No `schema_gates.rs`.

## perturbation

Widened `radius_above_zero` from `>` to `>=`. Rebuilt in the private target dir.

**red_output VERBATIM:**

```
thread 'mission::audio::tests::radius_zero_is_refused' (2769198) panicked at crates/map-engine-core/src/mission/audio.rs:427:10:
radius 0 must be refused: AuthoredAudio { emitters: [AuthoredEmitter { id: "ae-zero", x: 1.0, z: 2.0, y: None, sound: "SOUND_HINT", radius_m: 0.0, loop_sound: false, trigger_id: None }], music_cues: [] }
```

Restored `>`, `touch crates/map-engine-core/src/mission/audio.rs`, re-ran: **restored_green** (`test mission::audio::tests::radius_zero_is_refused ... ok`).

## gate_verdict_tail

```
  T-437 destroy inert      PASS
  T-586 route tags         PASS
  T-296 reporter identity  PASS
  T-452 player identity    PASS
  no-python (T-620)        PASS

  gate verdict PASS @ 2302ece0ea1f recorded: .ai/artifacts/verdicts/T-936.5.json
SLICE GATE: PASS
```

## mod_compile_verdict

```
OK: compiled clean
    Module: Game; loaded 5756x files; 11448x classes
    Compiling Game scripts took: 842.952000 ms
    0 warning(s) in TBD sources
```

First compile: `Syntax error` / `Unexpected scope` at `string event` (`event` is an Enforce keyword: `proto event void`). Field renamed `cueEvent`; ReadWire copies the raw JSON and in-place `Replace`s `"event":` -> `"cueEvent":` before `JsonLoadContext` (Format-copy so MissionLoader's cache is not mutated). Second compile: clean.

## files_outside_owns

Previous slices left next-ticket sentinels that go red when `audio` registers (the tests name T-936.5). Advanced the dummy key to `spawnModules` (T-936.6):

- `crates/map-engine-core/src/mission/compile.rs` — unlisted-key witness
- `crates/map-engine-core/src/mission/weather.rs` — `!is_authored_block("audio")`
- `crates/map-engine-core/src/mission/tasks.rs` — unlisted-key witness (comment said "using audio (T-936.5)")

No other paths.

## found_not_fixed

`flatten.rs` `EditorPayload::authored_blocks_root` still copies only `winConditions` / `tasks` / `radioPlan`. This slice does not own `flatten.rs`. `compile_payload` + `copy_authored_blocks` + `ExtensionBlocks::from_payload` carry `audio` on the payload root; `/compiled` will still drop the key until a named `EditorPayload` field lands (T-946.44 class, same as weatherTimeline). Brief: STOP and report — do not edit flatten.rs.

## deviations

- Ticket verify named `ci-local` / `leptos-gates`; brief says ignore those. Ran `cargo xtask schema validate` + slice gate + `mod compile`.
- No new golden-missions file: not in YOUR FILES. Existing goldens still PASS (`audio` is optional).
- `cargo test -p map-engine-core --all-features`: 1022 passed with `--skip everon_peaks_max_above_350`; the peaks test fails `Invalid PNG signature` (worktree LFS pointer DEM, as briefed). All `mission::audio` tests passed.
- Unread-fields gate stayed PASS. `audio` is not in `UNREAD_WIRE_FIELDS`; a real reader landed and CC does not need to retire a row.
- Enforce cannot declare a field named `event`; JSON key is rewritten to `cueEvent` at read (see mod_compile_verdict). Schema contract is still `event`.

## commits

- `2302ece0ea1f9c8bf09ff5b21a0c659556b9fa8f` — T-936.5: author positional audio emitters and music cues.

## manual_checklist

1. Author two emitters (one looping, one one-shot with triggerId) and one `mission_start` cue in Mission Settings once the panel is mounted (`settings_modal.rs` is outside owns, same as T-936.1–.4). Place on map: Place on map, click canvas, Use last marker. Save, `/compiled` after flatten learns the named field, load on a dedicated server.
2. Confirm `[TBD][Audio] armed` / `cue event='mission_start'` logs after LIVE. Emitter audible only inside radiusM; loop repeats inside, one-shot fires once on first enter. Trigger-gated emitter stays silent until that trigger FIRED.
3. Succeed / fail a task: matching cues fire. End the round: `mission_end` cue fires.
4. A mission with no audio still compiles to the same bytes as today (no `audio` key).
5. radius 0, unknown event, or duplicate id is refused in the panel and never reaches the wire.

## twins_confirmed

`diff -q` of both `TBD_AudioEmitter.c` paths: identical.
