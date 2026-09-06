# REPORT-T-133

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-133
slice/T-133
```

Command: `pwd && git branch --show-current`

## defect_verified_on_main

| claim | path:line | command |
|---|---|---|
| T-936.2 tasks model has no `schedule` / `startAfterS` / `windowS` | `crates/map-engine-core/src/mission/tasks.rs` on `dbb8d4998` (worktree parent / main at dispatch) | `git show dbb8d4998:crates/map-engine-core/src/mission/tasks.rs \| grep -n -i 'schedule\|startAfter\|windowS'` → no hits |
| `$defs/task` has `additionalProperties: false` and no `schedule` property | `packages/tbd-schema/schema/mission.schema.json:1216-1255` | same `git show` of that span; properties stop at `description` |
| State machine has no mission-clock schedule | `apps/mod/tbd-framework/Scripts/Game/TBD/Objectives/TBD_TaskStateMachine.c` on parent: `TBD_TaskStruct` is id/title/tier/state/triggerId/markerId/description only | `git show dbb8d4998:...TBD_TaskStateMachine.c` |

The defect still existed. T-936.2 / T-115 are shipped; they do not implement T-133.

## changes

| path | line | why |
|---|---|---|
| `crates/map-engine-core/src/mission/tasks.rs` | 133-177 | `Schedule {start_after_s, window_s}`, `window_is_legal` (`window_s > 0`, perturbation target), `validate_schedule` (window, non-negative start, start strictly inside mission length when length > 0) |
| `crates/map-engine-core/src/mission/tasks.rs` | 294, 306, 354+ | parse `schedule` on `AuthoredTask`; `KNOWN_KEYS` includes `schedule` (the tasks schema block that lives in this file) |
| `crates/map-engine-core/src/mission/tasks.rs` | 708-775 | validator tests: round-trip, omit-key, zero window (perturbation), negative window/start, start at/past mission length, T+0, non-object, unknown property, missing windowS |
| `apps/website/frontend/src/editor/panels/tasks_panel.rs` | 198-263 | `with_schedule` / `schedule_seconds`; empty-empty clears; half-filled refused; calls `validate_schedule(..., Some(mission_length_s))` |
| `apps/website/frontend/src/editor/panels/tasks_panel.rs` | 529-574 | unmounted panel still authors Start after (s) / Window (s); wasm reads `flow.timeLimitSeconds` (default 5400) |
| `apps/website/frontend/src/editor/panels/tasks_panel.rs` | 725-761 | wasm-native tests for write, zero-window refusal copy, start-past-length refusal copy, clear, half-fill |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Objectives/TBD_TaskStateMachine.c` | 26-34, 87-90, 202-203, 231, 267, 386-499 | `TBD_TaskScheduleStruct` with ABSENT sentinels; LIVE mission clock; inactive before `startAfterS`; evaluate inside `windowS`; fail when the window closes; log `[TBD][Task] id=<n> t=<s> -> <state>` |
| `apps/mod/tbd-export/Scripts/Game/TBD/Objectives/TBD_TaskStateMachine.c` | same | export twin, identical ASCII |

## perturbation

Predicate: `window_is_legal` (`tasks.rs:144-145`) `window_s > 0` → temporarily `window_s >= 0`.

**red_output VERBATIM** (`cargo test -p map-engine-core --all-features mission::tasks::tests::a_zero_window_is_refused -- --nocapture --exact`):

```
running 1 test

thread 'mission::tasks::tests::a_zero_window_is_refused' (1570376) panicked at crates/map-engine-core/src/mission/tasks.rs:708:39:
window 0: [AuthoredTask { id: "t1", title: "A", tier: Primary, state: Assigned, trigger_id: None, marker_id: None, description: None, schedule: Some(Schedule { start_after_s: 0, window_s: 0 }) }]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test mission::tasks::tests::a_zero_window_is_refused ... FAILED

failures:

failures:
    mission::tasks::tests::a_zero_window_is_refused

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 983 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p map-engine-core --lib`
```

**restored_green:** restored `window_s > 0`, `touch crates/map-engine-core/src/mission/tasks.rs`, same test `ok. 1 passed`. Full filter afterwards: `26 passed` (list total 26).

## gate_verdict_tail

```
  db_migrate persist       PASS
  T-439 objects aliases    PASS
  T-444 wiki seed          PASS
  T-440 faction library seed PASS
  T-438 deploy-staging     PASS
  T-456 REST size gate     PASS
  T-468 CI schema parity   PASS
  T-437 destroy inert      PASS
  T-586 route tags         PASS
  T-296 reporter identity  PASS
  T-452 player identity    PASS
  no-python (T-620)        PASS

  gate verdict PASS @ 2ec37726ec87 recorded: .ai/artifacts/verdicts/T-133.json
SLICE GATE: PASS
```

First gate at `802c16dda` was FAIL (`clippy::collapsible_if` on the mission-length check). Fixed with an edition-2024 let-chain in `2ec37726e`, re-ran, PASS.

## mod_compile_verdict

```
OK: compiled clean
    Module: Game; loaded 5749x files; 11389x classes
    Compiling Game scripts took: 880.338000 ms
    0 warning(s) in TBD sources
```

`cargo xtask mod compile` exit 0 (after EnfusionMCP copy already in the worktree; not committed).

## files_outside_owns

[]

## found_not_fixed

| path:line | repro |
|---|---|
| `packages/tbd-schema/schema/mission.schema.json:1220` (`$defs/task` `additionalProperties: false`, no `schedule`) | This slice must not edit that file. `KNOWN_KEYS` in `tasks.rs` accepts `schedule`; a compiled document that actually *emits* `tasks[].schedule` will fail json-schema until a schema slice adds the property. Flatten currently drops `tasks[]`, so `/compiled` does not hit this yet. |
| `crates/map-engine-core/src/mission/flatten.rs:1404-1409` `authored_blocks_root` copies only `winConditions` | T-936.3 / T-946.35. `GetRawJson()` (`/compiled`) has no `tasks[]` on a live compile. The state machine parses a hand-staged document with `tasks`+`schedule`. Not this slice's file. |

## deviations

[]

Schedule lives on the task object via the `KNOWN_KEYS` block in `tasks.rs` (the owned schema mirror). It is not a sidecar editor-only key. Sidecar would never reach a hand-staged `tasks[]` document the state machine reads.

## commits

- `802c16dda` T-133: add task schedule {startAfterS, windowS}
- `2ec37726e` T-133: collapse schedule length check for clippy

## manual_checklist

- Live dedicated-server boot of a **hand-staged** compiled JSON that contains `tasks: [{id, title, tier, state: assigned, triggerId, schedule: {startAfterS: 120, windowS: 60}}]`. Confirm the script log shows `[TBD][Task] id=<id> t=120 -> assigned` at T+2 min (LIVE clock), then either `[TBD][Task] id=<id> t=<s> -> succeeded` when the linked trigger FIRED inside the window, or `t=180 -> failed` if the window closes still assigned. Untimed tasks (no `schedule` key) must still succeed from triggers immediately.
- After T-946.33 mounts the tasks panel: set Window to `0` and confirm the panel shows the refusal containing `windowS` and `> 0`; set Start after to the authored `timeLimitSeconds` and confirm refusal containing `within mission length`.
- Headless `mod compile` cannot print those live log lines; they need a LIVE world clock.

## twins_confirmed

- `apps/mod/tbd-framework/Scripts/Game/TBD/Objectives/TBD_TaskStateMachine.c` exists
- `apps/mod/tbd-export/Scripts/Game/TBD/Objectives/TBD_TaskStateMachine.c` exists
- byte-identical, pure ASCII
