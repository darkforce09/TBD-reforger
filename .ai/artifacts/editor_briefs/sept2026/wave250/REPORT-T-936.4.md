# REPORT T-936.4 — Dynamic weather timeline

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-936.4
slice/T-936.4
```

First actions matched the brief. EnfusionMCP count was already 19. `export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target` for xtask/schema/mod compile. Tests that RUN a binary used `/home/Samuel/.cache/tbd-target-T-936.4` (shared cache can serve another slice's test binary).

## defect_verified_on_main

The worktree is `slice/T-936.4` at merge-base + T-942 packing. Before any code:

- `crates/map-engine-core/src/mission/weather.rs` — missing
- `apps/website/frontend/src/editor/panels/weather_timeline.rs` — missing
- both `TBD_WeatherRuntime.c` — missing
- `mission.schema.json` — no `weatherTimeline` property
- `AUTHORED_BLOCKS` keys: `radioPlan`, `winConditions`, `tasks` only

Weather was one static `environment.weatherPreset`. A payload carrying `weatherTimeline` would not be copied by `copy_authored_blocks`.

## changes

| path | why |
|---|---|
| `packages/tbd-schema/schema/mission.schema.json` | Optional top-level `weatherTimeline` + `$defs/weatherTimeline` / `weatherKeyframe` (`additionalProperties` false, strictly-increasing `atMinutes` documented, preset enum shared with environment). |
| `crates/map-engine-core/src/mission/weather.rs` | NEW. Parse/validate, `WEATHER_PRESETS`, `minutes_strictly_increase` (perturbation target), compile/carrier tests. |
| `crates/map-engine-core/src/mission/mod.rs` | Register `weather`. |
| `crates/map-engine-core/src/mission/extensions.rs` | AUTHORED_BLOCKS row `weatherTimeline`; `len()==4`; not in `DOCUMENT_OWNED_BLOCKS`. |
| `apps/website/frontend/src/editor/panels/weather_timeline.rs` | NEW. Add/edit/delete/reorder, undoable via `update_environment`, refuses equal/out-of-order `atMinutes`. |
| `apps/website/frontend/src/editor/panels/mod.rs` | Register `weather_timeline`. |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_WeatherRuntime.c` | NEW. Server applies keyframes at `atMinutes` via `ForceWeatherTo`; fog/windDirDeg overrides; logs each transition. Presence is `keyframes.Count()`. |
| `apps/mod/tbd-export/Scripts/Game/TBD/Gamemode/TBD_WeatherRuntime.c` | ASCII twin. |

`compile.rs` was not edited (generic `copy_authored_blocks`). `flatten.rs` was not edited (T-291). No `schema_gates.rs`.

## perturbation

Widened `minutes_strictly_increase` from `>` to `>=`. Rebuilt in the private target dir.

**red_output VERBATIM:**

```
thread 'mission::weather::tests::equal_at_minutes_are_refused' (2406468) panicked at crates/map-engine-core/src/mission/weather.rs:306:10:
equal atMinutes must be refused: AuthoredWeatherTimeline { keyframes: [AuthoredKeyframe { at_minutes: 10, weather_preset: "clear", wind_dir_deg: None, fog: None }, AuthoredKeyframe { at_minutes: 10, weather_preset: "overcast", wind_dir_deg: None, fog: None }] }
```

Restored `>`, `touch crates/map-engine-core/src/mission/weather.rs`, re-ran: **restored_green** (`ok`. Compiling line present).

## gate_verdict_tail

```
  T-437 destroy inert      PASS
  T-586 route tags         PASS
  T-296 reporter identity  PASS
  T-452 player identity    PASS
  no-python (T-620)        PASS

  gate verdict PASS @ e2e3e6fd8cb0 recorded: .ai/artifacts/verdicts/T-936.4.json
SLICE GATE: PASS
```

## mod_compile_verdict

```
OK: compiled clean
    Module: Game; loaded 5754x files; 11429x classes
    Compiling Game scripts took: 844.722000 ms
    0 warning(s) in TBD sources
```

First compile: `Method 'WeatherManager' is private` (name collision) — renamed helper to `GetTimeAndWeather`. Second: export ASCII scan failed on em-dash / arrow — transliterated both twins. Third: clean.

## files_outside_owns

[]

## found_not_fixed

`flatten.rs` `EditorPayload::authored_blocks_root` still copies only `winConditions` / `tasks` / `radioPlan`. This slice does not own `flatten.rs` (T-291). `compile_payload` + `copy_authored_blocks` + `ExtensionBlocks::from_payload` carry `weatherTimeline` on the payload root; `/compiled` will still drop the key until T-291 (or a T-946.xx follow-on like T-946.35 for tasks) adds the named `EditorPayload` field. Brief: STOP and report — do not edit flatten.rs.

## deviations

- Ticket verify named `ci-local` / `leptos-gates`; brief says ignore those. Ran `cargo xtask schema validate` + slice gate + `mod compile`.
- No new golden-missions file: not in YOUR FILES. Existing goldens still PASS (`weatherTimeline` is optional).
- `cargo test -p map-engine-core --all-features`: 1009 passed, 1 failed `dem::peaks::tests::everon_peaks_max_above_350` (`Invalid PNG signature`) — worktree LFS pointer DEM, as briefed. All `mission::weather` (14) and `mission::extensions` registration tests passed.
- Unread-fields gate stayed PASS (12 unread). `weatherTimeline` is not in `UNREAD_WIRE_FIELDS`; a real reader landed and CC does not need to retire a row.

## commits

- `e2e3e6fd8cb08b9826d8f9312e4740270d15e4f6` — T-936.4: author weatherTimeline keyframes through AUTHORED_BLOCKS.

## manual_checklist

1. Author three keyframes (e.g. 0 / 15 / 40 min, clear → overcast → heavy_rain) in Mission Settings once the panel is mounted (`settings_modal.rs` is outside owns, same as T-936.1/.2/.3). Save, `/compiled` after flatten learns the named field, load on a dedicated server.
2. Confirm `[TBD][Weather] transition` logs fire at the authored offsets after LIVE (not briefing).
3. Clients see the same weather without a second apply (ForceWeatherTo is server-only; engine replicates).
4. A mission with no timeline still compiles to the same bytes as today (no `weatherTimeline` key).
5. Out-of-order / equal `atMinutes` is refused in the panel and never reaches the wire.

## twins_confirmed

`diff -q` of both `TBD_WeatherRuntime.c` paths: identical.
