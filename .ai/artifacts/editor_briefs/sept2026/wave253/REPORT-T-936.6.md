# REPORT T-936.6 — Dynamic AI spawning and garrison modules

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-936.6
slice/T-936.6
```

Command: `pwd && git branch --show-current` (first action). EnfusionMCP already 19 files; no copy.

## defect_verified_on_main

Worktree tracked `8c4669ca9` at dispatch. Defect still present before the edit: no `spawnModules` in schema, no `spawn_modules.rs`, no `TBD_DynamicSpawner.c`, `AUTHORED_BLOCKS.len() == 5`.

Pre-edit pin (GREEN because the key was unlisted):

```
cargo test -p map-engine-core --all-features an_unlisted_environment_key
test mission::tasks::tests::an_unlisted_environment_key_is_not_promoted ... ok
test mission::compile::tests::an_unlisted_environment_key_is_not_promoted_to_the_payload_root ... ok
```

Those tests asserted `environment.spawnModules` is **not** copied onto the payload root (`spawnModules` has no AUTHORED_BLOCKS row until T-936.6). Flatten `authored_blocks_root` copied only `winConditions` / `tasks` / `radioPlan`.

## changes

| path | what |
|---|---|
| `packages/tbd-schema/schema/mission.schema.json` | Top-level `spawnModules[]` + `$defs/spawnModule` (`wave`\|`garrison`, x+z XOR zoneId via `oneOf`, additionalProperties false, maxAlive/count cap 32). |
| `crates/map-engine-core/src/mission/spawn_modules.rs` | Model + validator (known faction, exclusive placement, positive counts, MAX_ALIVE=32). Perturbation target: `placement_is_exclusive`. |
| `crates/map-engine-core/src/mission/mod.rs` | Register `spawn_modules`. |
| `crates/map-engine-core/src/mission/extensions.rs` | AUTHORED_BLOCKS row; len 5→6. Unlisted dummy is now `tacticalGraphics`. |
| `crates/map-engine-core/src/mission/compile.rs` | Flip future-pin: unlisted dummy is `tacticalGraphics`. |
| `crates/map-engine-core/src/mission/weather.rs` | `assert!(is_authored_block("spawnModules"))`. |
| `crates/map-engine-core/src/mission/audio.rs` | Same pin flip. |
| `crates/map-engine-core/src/mission/tasks.rs` | Unlisted dummy is `tacticalGraphics`. |
| `crates/map-engine-core/src/mission/flatten.rs` | Named fields for `weatherTimeline` / `audio` / `spawnModules`; `authored_blocks_root` copies every AUTHORED_BLOCKS key. Acceptance test: wave+garrison flattens. |
| `apps/website/frontend/src/editor/panels/spawn_modules.rs` | Undoable module list (kind, faction, template, x/z XOR zone, count, interval, maxAlive, trigger). |
| `apps/website/frontend/src/editor/panels/mod.rs` | Register `spawn_modules`. |
| `apps/website/frontend/src/editor/panels/settings_modal.rs` | Mount `{spawn_modules_panel(ctrl)}` + Class-R source pin. |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_DynamicSpawner.c` | Server waves on interval/trigger up to maxAlive; garrison once; cleanup on END. |
| `apps/mod/tbd-export/Scripts/Game/TBD/Gamemode/TBD_DynamicSpawner.c` | Export twin (byte-identical). |

## perturbation

Widened `placement_is_exclusive` from `has_position != has_zone` to `has_position \|\| has_zone` so both x/z and zoneId are accepted.

**red_output VERBATIM** (`cargo test -p map-engine-core --all-features --lib both_position_and_zone_are_refused -- --nocapture`):

```
running 1 test

thread 'mission::spawn_modules::tests::both_position_and_zone_are_refused' (3893697) panicked at crates/map-engine-core/src/mission/spawn_modules.rs:406:9:
the predicate itself must refuse both
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test mission::spawn_modules::tests::both_position_and_zone_are_refused ... FAILED

failures:

failures:
    mission::spawn_modules::tests::both_position_and_zone_are_refused

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1049 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p map-engine-core --lib`
```

**restored_green:** restore `!=`, `touch crates/map-engine-core/src/mission/spawn_modules.rs`:

```
test mission::spawn_modules::tests::both_position_and_zone_are_refused ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1049 filtered out; finished in 0.00s
```

## gate_verdict_tail

Last lines of `cargo xtask platform wave gate --slice T-936.6`:

```
  clippy (changed crates)  PASS
  schema                   PASS
  T-278 catalogue drift    PASS
  db_migrate claim body    PASS
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

  gate verdict PASS @ 8c4669ca97a4 recorded: .ai/artifacts/verdicts/T-936.6.json
SLICE GATE: PASS
```

Verdict SHA is the pre-commit worktree HEAD (`8c4669ca9`). `cargo xtask schema validate`: All contracts valid. Unread-wire did not trip (spawnModules was not a pinned unread field).

## mod_compile_verdict

`OK: compiled clean` — 5759 files, 11473 classes, 0 TBD warnings.

```
OK: compiled clean
    Module: Game; loaded 5759x files; 11473x classes
    Compiling Game scripts took: 896.371000 ms
    0 warning(s) in TBD sources
```

Did not restart :3000/:8080.

## files_outside_owns

[]

(REPORT path is required by the slice brief; listed under commits.)

## found_not_fixed

| path:line | repro |
|---|---|
| `settings_modal.rs` tasks/radio/weather/audio panels | T-936.2–.5 panels are registered in `panels/mod.rs` but still unmounted. Brief: mount spawn_modules; do not repeat the T-936.5 miss. Win conditions is already mounted. |
| Group-template catalog | Validator requires nonempty `groupTemplate` + known `factionKey` (`blufor`/`opfor`/`indfor`/`civ`). There is no group-prefab catalog in this crate to check keys against. Unknown resources log and skip at runtime. |
| Garrison "hold" AI | Garrison spawns once and does not restock. Defend/hold-fire behaviour is T-678; not implemented. |
| `dem::peaks::tests::everon_peaks_max_above_350` | Worktree LFS pointer (`Invalid PNG signature`). Pre-existing; not this slice. |

## deviations

- Generic `authored_blocks_root` also added named serde fields for `weatherTimeline` and `audio` (brief: generic AUTHORED_BLOCKS copy is better; do not fix leftover flatten omits unless the generic path does it).
- Ticket verify listed `ci schema-validate` / `ci-local-leptos` / `leptos-gates`; wave brief forbids those. Ran `cargo xtask schema validate` + slice gate + `mod compile`.
- `cargo test -p map-engine-core --all-features`: 1047 passed, 1 failed (`dem::peaks` LFS), 2 ignored.
- Empty `spawnModules: []` is refused (omit rather than author), same as weather/audio empty blocks.
- Panel: switching zone → x defaults missing `z` to 0 so the row stays XOR-valid mid-edit.

## commits

- `33be8aff933085dd5effef097527248ee751e840` — T-936.6: authored spawnModules waves and garrisons.

## manual_checklist

In-game (gate does not stand in for this):

- Mission with one wave module (`intervalSeconds` + `maxAlive`) and one garrison: `/compiled` carries `spawnModules` with both rows.
- LIVE: wave spawns `count` groups, waits the interval, restocks while living groups < `maxAlive`.
- Garrison spawns once and does not restock when killed.
- `triggerId` delays first spawn until that T-676 trigger FIRED; missing trigger logs and stays idle.
- END/DEBRIEF: spawned groups are deleted (`[TBD][Spawn] cleanup`).
- Unknown `factionKey` or both x/z and `zoneId` refused in the Mission Settings panel.

## twins_confirmed

- `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_DynamicSpawner.c` — on disk
- `apps/mod/tbd-export/Scripts/Game/TBD/Gamemode/TBD_DynamicSpawner.c` — on disk (`cmp` identical)
