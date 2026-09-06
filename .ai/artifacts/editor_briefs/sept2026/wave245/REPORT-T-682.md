# REPORT-T-682

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-682
slice/T-682
```

First command this session: `pwd && git branch --show-current` matched that path and branch. Work stayed in this worktree.

## defect_verified_on_main

Verified on this worktree at `65f4d44f3` (slice base, before any T-682 commits) — that is main as of wave 245 dispatch.

| claim | path:line | command |
|---|---|---|
| `ModEnvironment` serialises only `dateTime` / `weatherPreset`; no `windDirDeg` / `fog` / `wind` / `viewDistance` fields | `crates/map-engine-core/src/mission/flatten.rs:407` (`pub struct ModEnvironment` was two skip-empty strings) | `rg -n 'windDirDeg\|struct ModEnvironment' crates/map-engine-core/src/mission/flatten.rs` — zero `windDirDeg` hits |
| No Enfusion environment reader | (file absent) | `ls apps/mod/tbd-framework/Scripts/Game/TBD/Backend/` — no `TBD_EnvironmentReader.c` |
| Loader does not bind or apply environment | `TBD_MissionLoader.c` document struct ended at `settings` | `rg -n 'environment\|fog\|wind\|viewDistance' apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionLoader.c` — zero hits |
| Editor still refuses to author those keys (must stay that way) | `apps/website/frontend/src/editor/panels/env.rs:328` `keys_nothing_reads_are_not_authored` | left untouched; test still passes |

## changes

| path | line | why |
|---|---|---|
| `crates/map-engine-core/src/mission/flatten.rs` | 407–426 | `ModEnvironment` now has optional `windDirDeg` / `fog` / `wind` / `viewDistance` with `skip_serializing_if = Option::is_none` so absent keys keep today's bytes |
| `crates/map-engine-core/src/mission/flatten.rs` | 3526–3527 | Latch `any_1_3_key` on axes that will actually serialise (not on a mention in the payload) |
| `crates/map-engine-core/src/mission/flatten.rs` | 3657–3665 | Copy axes into the emitted `ModEnvironment` |
| `crates/map-engine-core/src/mission/flatten.rs` | 3720+ | `EnvironmentAxes` + range gates; `0` fog/wind/dir is `Some(0)`, not drop |
| `crates/map-engine-core/src/mission/flatten.rs` | 7543+ | Four tests: authored emit + 1.3 latch, authored zeros kept, absent omit + no 1.3 bump, malformed/out-of-range dropped |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_EnvironmentReader.c` | NEW (180 lines) | Bind struct with `ABSENT = -1e6`; apply fog/wind via `BaseWeatherManagerEntity` overrides; view distance via `GetGame().SetViewDistance`. Does **not** apply `dateTime`/`weatherPreset` (every compiled doc already has those; applying them would change boot for missions without fog/wind/viewDistance) |
| `apps/mod/tbd-export/Scripts/Game/TBD/Backend/TBD_EnvironmentReader.c` | NEW | Byte-identical twin |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionLoader.c` | 359, 1094 | `environment` on `TBD_MissionDocumentStruct`; `TBD_EnvironmentReader.Apply()` after `ApplyMissionSettings()` |
| `apps/mod/tbd-export/Scripts/Game/TBD/Backend/TBD_MissionLoader.c` | 359, 1094 | Same code (comments already ASCII-divergent from framework, as before) |

Not touched: `packages/tbd-schema/**`, `apps/website/frontend/src/editor/panels/env.rs`, `.ai/tickets/`.

## perturbation

Broke the thing the serialise test guards: `EnvironmentAxes::from_payload_bag` forced `wind_dir_deg: None`. RED output VERBATIM:

```
   Compiling map-engine-core v0.1.0 (/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-682/crates/map-engine-core)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 8.58s
     Running unittests src/lib.rs (/home/Samuel/.cache/tbd-target/debug/deps/map_engine_core-374f4772eaee4d5d)

running 1 test

thread 'mission::flatten::tests::t682_environment_axes_serialise_when_authored' (700766) panicked at crates/map-engine-core/src/mission/flatten.rs:7558:9:
assertion `left == right` failed
  left: None
 right: Some(45.0)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test mission::flatten::tests::t682_environment_axes_serialise_when_authored ... FAILED

failures:

failures:
    mission::flatten::tests::t682_environment_axes_serialise_when_authored

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 957 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p map-engine-core --lib`
```

Restored the `env_opt_closed(env, "windDirDeg", 0.0, 360.0)` line, `touch crates/map-engine-core/src/mission/flatten.rs`, re-ran.

**restored_green:** `cargo test -p map-engine-core --all-features --lib -- t682` → `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 954 filtered out`. `--list` also showed exactly those 4 tests (this worktree's binary `map_engine_core-374f4772eaee4d5d`, not a foreign T-300 cache).

`compiler_shaped_golden_is_a_fresh_emitter_output` still ok (optional keys omitted). `keys_nothing_reads_are_not_authored` still ok (env.rs untouched).

## gate_verdict_tail

Last 15 lines of `cargo xtask platform wave gate --slice T-682`:

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

  gate verdict FAIL @ 611106680d1e recorded: .ai/artifacts/verdicts/T-682.json
SLICE GATE: FAIL
```

The only failing step was `schema`. Height-labels SKIP is the worktree LFS-pointer DEM (environmental, as briefed). Schema validate's unread-fields check:

```
T-706 unread 1.3 wire fields (each must stay reader-free until its ticket lands):
  FAIL  a 1.3 wire field gained a reader — update mission.schema.json
        'fog' now has 11 mod identifier(s) (baseline 0) — if T-682 landed the reader, DROP the field's "no reader on any shipped build" wording in mission.schema.json and remove/repin its UNREAD_WIRE_FIELDS row; ...
        'wind' now has 10 mod identifier(s) (baseline 0) — ...
        'viewDistance' now has 8 mod identifier(s) (baseline 0) — ...
```

This is the T-676 / T-674.2 shape: the slice gate went RED **because the ticket succeeded**. `JsonLoadContext` binds by field name, so the identifiers `fog` / `wind` / `viewDistance` ARE the contract. Brief owns-list + "Do not touch packages/tbd-schema" forbids the two files the gate's own message names. Did not widen. Command centre retires the three `UNREAD_WIRE_FIELDS` rows (baselines were 0 — retire, do not re-pin) and drops the "no reader on any shipped build" wording on `$defs/environment`.

## mod_compile_verdict

```
==> compiling tbd-framework + tbd-export (native headless server, no Workbench)
    (calibrating vanilla-only baseline, one time)
OK: compiled clean
    Module: Game; loaded 5743x files; 11335x classes
    Compiling Game scripts took: 868.069000 ms
    0 warning(s) in TBD sources
```

EnfusionMCP: 19 `.c` files present in the worktree, not committed.

## files_outside_owns

[] — no extra files edited.

Required for `SLICE GATE: PASS` (not edited; command centre):

- `xtask/src/schema_gates.rs` — retire `fog` / `wind` / `viewDistance` `UNREAD_WIRE_FIELDS` rows
- `packages/tbd-schema/schema/mission.schema.json` — drop "no reader on any shipped build" on those properties (brief forbade this file)

## found_not_fixed

| path:line | repro |
|---|---|
| `xtask/src/schema_gates.rs` ~2650 (`UNREAD_WIRE_FIELDS` fog/wind/viewDistance expected 0) | `cargo xtask schema validate` after this reader: fog 11, wind 10, viewDistance 8. Same command the slice gate runs. |
| `packages/tbd-schema/schema/mission.schema.json` `$defs/environment` description | Still says nothing reads `fog`/`wind`/`viewDistance` and that `ModEnvironment` does not serialise `windDirDeg`. Both sentences are now false. |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionValidator.c` ~1160 (`environment` unconsumed presence warning) and export twin | `CheckUnconsumedKeys` still treats the whole `environment` block as unconsumed. Fog/wind/viewDistance now have a consumer; `dateTime`/`weatherPreset` still do not (T-290 inventory; not applied here on purpose). Validator is not in owns. Wallpaper warning on every compiled mission. |

`cargo test -p map-engine-core --all-features --lib`: 955 passed, 1 failed (`dem::peaks::tests::everon_peaks_max_above_350` → `Invalid PNG signature`) — worktree LFS pointer, environmental, not chased.

## deviations

- Brief asked for `SLICE GATE: PASS`. Gate is FAIL on schema unread-fields **because the reader landed**. Owns list + "do not touch packages/tbd-schema" + "STOP and report, do not widen" is the T-676 instruction; followed that over the PASS line. Command centre's T-676 commit (`aa9ca991f`) is the template.
- Did not apply `dateTime` / `weatherPreset` in the reader. Ticket acceptance is fog, wind, and view distance. Every compiled document already carries the first two; applying them would change boot for missions that never authored the T-682 axes ("missions without those fields boot unchanged").

## commits

- `2f998c3f4` T-682: serialise fog, wind, windDirDeg, viewDistance on ModEnvironment
- `611106680` T-682: apply fog, wind, viewDistance at mission boot

HEAD `611106680d1e3c0cd786a949a775cfb99a653b17`. Worktree clean. No push.

## manual_checklist

- Dedicated-server boot `packages/tbd-schema/golden-missions/schema-1_3-wire-fields.json` (`fog: 0.2`, `wind: 3.5`, `windDirDeg: 45`, `viewDistance: 2500`). Confirm log lines `[TBD][Environment] fog=... applied`, `wind=... m/s applied`, `windDirDeg=... applied`, `viewDistance=... m applied`, and that fog/wind/far-clip are actually visible in-world (headless compile cannot prove this).
- Dedicated-server boot `packages/tbd-schema/golden-missions/compiler-shaped-two-faction.json` (only `dateTime`/`weatherPreset`). Confirm **no** `[TBD][Environment]` apply lines and that world fog/wind/view-distance stay at the scenario default.
- Open Mission Settings in the editor: still no fog / wind / view-distance controls (`author_env` still refuses those keys).

## twins_confirmed

| path | on disk |
|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_EnvironmentReader.c` | yes (7716 bytes) |
| `apps/mod/tbd-export/Scripts/Game/TBD/Backend/TBD_EnvironmentReader.c` | yes (7716 bytes, `cmp` identical) |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionLoader.c` | yes (edited) |
| `apps/mod/tbd-export/Scripts/Game/TBD/Backend/TBD_MissionLoader.c` | yes (edited; code agrees after comment-strip lockstep; framework comments still use em-dashes in older hunks) |
