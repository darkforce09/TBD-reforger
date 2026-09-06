# REPORT T-936.2

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-936.2
slice/T-936.2
```

Confirmed before the first edit (`pwd && git branch --show-current`). HEAD after owns revert: `0d9c07394`.

`git diff --name-only 65f4d44f3...HEAD` — `flatten.rs` **absent**. `compile.rs` **absent**.

## defect_verified_on_main

| claim | path:line | command |
|---|---|---|
| `tasks` is not an authored block | `crates/map-engine-core/src/mission/extensions.rs` test `win_conditions_is_the_registered_block_and_the_document_models_it` asserted `!is_authored_block("tasks")` and `AUTHORED_BLOCKS.len() == 1` | `cargo test -p map-engine-core --all-features --lib win_conditions_is_the_registered_block_and_the_document_models_it` → **ok** (defect present) |
| A payload carrying `tasks` in the env bag is not promoted to the payload root | `crates/map-engine-core/src/mission/compile.rs:1311` `an_unlisted_environment_key_is_not_promoted_to_the_payload_root` (`tasks` dummy, message "`tasks` has no AUTHORED_BLOCKS row until T-936.2") | `cargo test -p map-engine-core --all-features --lib an_unlisted_environment_key` → **ok** on merge-base (defect present) |
| Schema has no `tasks` array; `task` is only an enum value on objectives | `packages/tbd-schema/schema/mission.schema.json` top-level properties lacked `tasks` | `python3` dump of `properties` keys — no `tasks` |

## changes

| path | line | why |
|---|---|---|
| `packages/tbd-schema/schema/mission.schema.json` | 98 (`tasks` property), 1216 (`$defs/task`) | Top-level `tasks[]`; items `{id,title,tier,state,triggerId?,markerId?,description?}`, `additionalProperties: false` |
| `packages/tbd-schema/golden-missions/schema-1_3-tasks.json` | NEW (300 lines) | Three tiers (primary/secondary/optional), all `assigned`; two `editorTriggers` for the linked ids |
| `crates/map-engine-core/src/mission/tasks.rs` | NEW; `LEGAL_TRANSITIONS` :35-38; `validate` registered | Typed model; assigned→succeeded\|failed only; compile_payload + `ExtensionBlocks` tests (no flatten.rs) |
| `crates/map-engine-core/src/mission/mod.rs` | 17 `pub mod tasks` | Register the module |
| `crates/map-engine-core/src/mission/extensions.rs` | 81 `key: "tasks"` | AUTHORED_BLOCKS row so `copy_authored_blocks` / `ExtensionBlocks::from_payload` emit `tasks` |
| `apps/website/frontend/src/editor/panels/tasks_panel.rs` | NEW; `add_task`/`remove_task`/`move_task`/`with_field`/`env_patch` | Undoable list; tier, trigger, marker pickers |
| `apps/website/frontend/src/editor/panels/mod.rs` | 33 `pub mod tasks_panel` | Register the panel |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Objectives/TBD_TaskStateMachine.c` | NEW; `TryTransition` :185; `IsLegal` :209 | Server-authoritative table; T-676 FIRED → succeeded; INERT/missing trigger → failed |
| `apps/mod/tbd-export/Scripts/Game/TBD/Objectives/TBD_TaskStateMachine.c` | NEW twin | T-946.26 |
| `apps/mod/tbd-framework/Scripts/Game/TBD/UI/TBD_TaskHud.c` | NEW; `InsertStaticMarker` :115; `BuildSnapshot` :122 | One HUD marker per assigned task via `TBD_MarkerIcons`; hidden once omitted from snapshot |
| `apps/mod/tbd-export/Scripts/Game/TBD/UI/TBD_TaskHud.c` | NEW twin | T-946.26 |

## perturbation

**Break:** added `(TaskState::Succeeded, TaskState::Assigned)` to `LEGAL_TRANSITIONS`.

**red_output VERBATIM:**

```
    Blocking waiting for file lock on package cache
   Compiling map-engine-core v0.1.0 (/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-936.2/crates/map-engine-core)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.06s
     Running unittests src/lib.rs (/home/Samuel/.cache/tbd-target-T-936.2/debug/deps/map_engine_core-374f4772eaee4d5d)

running 1 test

thread 'mission::tasks::tests::succeeded_to_assigned_is_illegal' (753244) panicked at crates/map-engine-core/src/mission/tasks.rs:423:14:
succeeded → assigned must be refused: Assigned
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test mission::tasks::tests::succeeded_to_assigned_is_illegal ... FAILED

failures:

failures:
    mission::tasks::tests::succeeded_to_assigned_is_illegal

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 968 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p map-engine-core --lib`
```

**restored_green:** table restored, `touch crates/map-engine-core/src/mission/tasks.rs`, `cargo test -p map-engine-core --all-features --lib succeeded_to_assigned_is_illegal` → `ok. 1 passed`.

## gate_verdict_tail

Last 15 lines of `cargo xtask platform wave gate --slice T-936.2` after the owns revert (HEAD `0d9c07394`):

```
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

  gate verdict PASS @ 0d9c073943f1 recorded: .ai/artifacts/verdicts/T-936.2.json
SLICE GATE: PASS
```

## mod_compile_verdict

```
OK: compiled clean
    Module: Game; loaded 5744x files; 11346x classes
    Compiling Game scripts took: 875.479000 ms
    0 warning(s) in TBD sources
```

EnfusionMCP (19 gitignored `.c`) was present; not committed.

## files_outside_owns

[] — current `65f4d44f3...HEAD` contains none of `flatten.rs` or `compile.rs`.

**Note (compile.rs, reverted):** T-936.1's `an_unlisted_environment_key_is_not_promoted_to_the_payload_root` uses `tasks` as the dummy unlisted key and asserts it is **not** promoted. Registering `tasks` in AUTHORED_BLOCKS makes that test red without any compile.rs edit. The unlisted-key witness was moved to `tasks.rs::an_unlisted_environment_key_is_not_promoted` against `audio` (T-936.5). After revert, this sibling test is red:

```
thread 'mission::compile::tests::an_unlisted_environment_key_is_not_promoted_to_the_payload_root' (801543) panicked at crates/map-engine-core/src/mission/compile.rs:1320:9:
`tasks` has no AUTHORED_BLOCKS row until T-936.2: {...,"tasks":[{"id":"t1"}]}
```

Command center / T-936.1 bookkeeping should retarget that dummy key (same `audio` swap this slice used, then reverted). This slice does not keep a compile.rs edit.

**Note (flatten.rs, reverted):** an `EditorPayload.tasks` field was added then removed per command-center order. `git diff --name-only 65f4d44f3...HEAD` has no `flatten.rs`.

## found_not_fixed

| path:line | repro |
|---|---|
| `crates/map-engine-core/src/mission/flatten.rs` `EditorPayload::authored_blocks_root` (merge-base: copies only `winConditions`) | `flatten_to_mod_document` on a payload whose root has `tasks` still omits `tasks` from the compiled document, because serde drops the unknown key and `authored_blocks_root` never reinserts it. `compile_payload` + `ExtensionBlocks::from_payload` **do** carry the block. T-936.1's comment in flatten.rs says a later slice adds a one-liner here; this slice was ordered **not** to. |
| `crates/map-engine-core/src/mission/compile.rs:1320` | `cargo test -p map-engine-core --all-features --lib an_unlisted_environment_key_is_not_promoted_to_the_payload_root` — red solely because AUTHORED_BLOCKS now lists `tasks`. See files_outside_owns. |
| `apps/website/frontend/src/editor/panels/settings_modal.rs` (not owned) | `tasks_panel` is registered and unit-tested but not mounted. Same T-936.1 card pattern. |

## deviations

- `flatten.rs` was briefly edited then **reverted** to `65f4d44f3` (`0d9c07394`). No flatten.rs in the slice diff.
- `compile.rs` dummy-key rename was briefly applied then **reverted**. Assertion lives in `tasks.rs`.
- Acceptance "flattens to a tasks block" is proven at **compile_payload / ExtensionBlocks**, not at `flatten_to_mod_document` (T-682 owns that file).
- `cargo test -p map-engine-core --all-features` is not fully green: one sibling compile.rs test (above) + environmental `dem::peaks::tests::everon_peaks_max_above_350` (`Invalid PNG signature` — LFS pointer in worktree). Owned `mission::tasks` tests: 16 passed. List vs run on private target: 969 listed; 966 passed + 1 LFS fail + 2 ignored = 969.
- `tasks_panel` is not mounted in Mission Settings (file not owned).

## commits

- `fcc946cf2` T-936.2: schema — top-level tasks[] with tier and state
- `ec8e61194` T-936.2: core — tasks model, transition table, AUTHORED_BLOCKS row
- `fe3ba2969` T-936.2: editor — tasks panel with tier, trigger and marker pickers
- `5e58e5669` T-936.2: mod — task state machine and HUD markers, both trees
- `0d9c07394` T-936.2: revert flatten.rs and compile.rs owns breaches

## manual_checklist

- Mount `{tasks_panel(ctrl)}` immediately after `{win_conditions_card(ctrl)}` in `settings_modal.rs` (one line; T-936.1's card waited for wave bookkeeping the same way).
- In the editor: add three tasks (primary / secondary / optional), set trigger and marker pickers, reorder, undo — one undo step per write.
- In-game: a mission with those three tasks — each starts assigned; firing the linked T-676 trigger succeeds that task; an inert or missing trigger fails it; a task with no `triggerId` stays assigned.
- HUD: only assigned tasks show a placed-custom marker (`TBD_MarkerIcons`); succeeded/failed markers disappear on every client (listen host + dedicated).
- After T-682 (or bookkeeping) adds the flatten.rs `tasks` one-liner T-936.1 documented: confirm `/compiled` JSON has a root `tasks` array matching the payload.

## twins_confirmed

| path | on disk |
|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/Objectives/TBD_TaskStateMachine.c` | yes (12778 B) |
| `apps/mod/tbd-export/Scripts/Game/TBD/Objectives/TBD_TaskStateMachine.c` | yes (12778 B, byte-identical, ASCII) |
| `apps/mod/tbd-framework/Scripts/Game/TBD/UI/TBD_TaskHud.c` | yes (8084 B) |
| `apps/mod/tbd-export/Scripts/Game/TBD/UI/TBD_TaskHud.c` | yes (8084 B, byte-identical, ASCII) |
