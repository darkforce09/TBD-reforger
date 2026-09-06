# REPORT-T-291

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-291
slice/T-291
```

First command this session: `pwd && git branch --show-current` matched that path and branch. Work stayed in this worktree.

## defect_verified_on_main

Verified on this worktree at `2eff41f30` (slice HEAD before any T-291 commit) — wave 250 brief commit on main.

| claim | evidence |
|---|---|
| Flatten already **emits** `settings.spectatorPolicy` / `settings.nightVision` / `environment.windDirDeg` (T-259 / T-682) | `derive_settings` ~L3120; `EnvironmentAxes::from_payload_bag` `windDirDeg`; tests `settings_reach_the_compiled_wire_when_authored`, `t682_environment_axes_serialise_when_authored` |
| Empty-string trim on `respawn`/`spectatorPolicy` is **not** "drop the fields" — only blank strings | comment at `derive_settings`; non-empty values reach the wire |
| `TBD_FrameworkManager.c` did **not** apply `windDirDeg` or `nightVision` | `rg` on both manager twins: no `windDirDeg` / `nightVision` / `GetSettings` |
| `TBD_SpectatorController.c` did **not** read `spectatorPolicy` | `rg` on both controller twins: no `spectatorPolicy`; `Enter()` ran with no delay / no `none` black-screen |
| MissionLoader applies faction restriction on the **server** only; `TBD_SpectatorTargets` is a process-local static (client `free` was a dead mechanism) | `ApplyMissionSettings` in `TBD_MissionLoader.c`; SpectatorTargets `SetFactionRestricted` has no RplProp |
| `factions[].color`, `roles[].radio`, `layers[]` not emitted; no editor-only comment | `ModFaction` had no `color`; `ModOrbatRole` had no `radio`; `ModMissionDocument` had no `layers` |

Ticket line numbers (flatten.rs:2104) are stale; live trim is `derive_settings`.

## changes

| path | why |
|---|---|
| `crates/map-engine-core/src/mission/flatten.rs` | Editor-only comments on `ModFaction` / `ModOrbatRole` / `ModMissionDocument` (color, radio, layers). `derive_settings` docs name T-291 readers. Tests `t291_runtime_orphans_reach_the_compiled_wire` and `t291_color_radio_layers_are_editor_only_and_do_not_reach_the_wire`. Did **not** invent T-290's emit-ledger table. |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_FrameworkManager.c` | Replicated `m_sSpectatorPolicy` / `m_bNightVision`. `ApplyAuthoredWeather` (`SetWindDirectionOverride` on `windDirDeg`, ABSENT sentinel). `ApplyAuthoredSettings` latches settings + `BumpMe()`. Spawn hook strips `EGadgetType.NIGHT_VISION` when policy is off (local named `gadgetMgr`, not `gadgets`, so T-705 unread baseline stays 6). |
| `apps/mod/tbd-export/Scripts/Game/TBD/Gamemode/TBD_FrameworkManager.c` | ASCII twin of the above. |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Spectator/TBD_SpectatorController.c` | Reads `TBD_FrameworkManager.GetSpectatorPolicy()` on the **client**. `none` → stay on death view; `own_side_delayed_60s` → 60 s delay + faction restriction ON; `free` → restriction OFF, enter immediately. |
| `apps/mod/tbd-export/Scripts/Game/TBD/Spectator/TBD_SpectatorController.c` | ASCII twin of the above. |

Not touched: `packages/tbd-schema/**`, `TBD_GadgetFlags.c`, `TBD_MissionLoader.c`, `TBD_WeatherRuntime.c`, `.ai/tickets/`, docs.

## perturbation

Re-trimmed `spectatorPolicy` in `derive_settings` (`spectator_policy: None`). RED output VERBATIM:

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.09s
     Running unittests src/lib.rs (/home/Samuel/.cache/tbd-target/debug/deps/map_engine_core-374f4772eaee4d5d)

running 1 test

thread 'mission::flatten::tests::t291_runtime_orphans_reach_the_compiled_wire' (2414547) panicked at crates/map-engine-core/src/mission/flatten.rs:5067:9:
assertion `left == right` failed
  left: Null
 right: "own_side_delayed_60s"
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test mission::flatten::tests::t291_runtime_orphans_reach_the_compiled_wire ... FAILED

failures:

failures:
    mission::flatten::tests::t291_runtime_orphans_reach_the_compiled_wire

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 999 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p map-engine-core --lib`
```

Restored `spectator_policy: trim(&s.spectator_policy)`, `touch crates/map-engine-core/src/mission/flatten.rs`, re-ran.

**restored_green:** `cargo test -p map-engine-core --all-features mission::flatten::tests::t291_runtime_orphans_reach_the_compiled_wire -- --exact` → `test result: ok. 1 passed`. Full `mission::flatten` module: `94 passed; 0 failed; 2 ignored`.

## gate_verdict_tail

Last lines of `cargo xtask platform wave gate --slice T-291` (waited ~60s on T-936.4 gate lock, then):

```
  T-437 destroy inert      PASS
  T-586 route tags         PASS
  T-296 reporter identity  PASS
  T-452 player identity    PASS
  no-python (T-620)        PASS

  gate verdict PASS @ 2eff41f30108 recorded: .ai/artifacts/verdicts/T-291.json
SLICE GATE: PASS
```

Recorded SHA is pre-commit HEAD; the gated working tree included these edits (`skip:` none).

## mod_compile_verdict

```
OK: compiled clean
    Module: Game; loaded 5753x files; 11419x classes
    Compiling Game scripts took: 874.900000 ms
    0 warning(s) in TBD sources
```

(After renaming the `gadgets` local so schema unread `gadgets` baseline stayed 6. First compile with that local also compiled clean; schema validate then failed until the rename.)

## files_outside_owns

[]

(Gate-written `.ai/artifacts/verdicts/T-291.json` is untracked harness output, not edited as source. EnfusionMCP copy is gitignored, not committed.)

## found_not_fixed

- `settings.respawn` is emitted (T-259) but has no pool reader in T-291 owns — T-181 lineage, as the plan states.
- `TBD_MissionValidator.c` still warns `nightVision=true authored but no NVG policy seam exists` — file not in owns; the seam now lives in FrameworkManager.
- `TBD_MissionLoader.c` still logs `no NVG setter in MissionLoader owns` — true for that file; FrameworkManager is the setter.
- T-682 `TBD_EnvironmentReader.Apply()` still applies `windDirDeg` at parse; FrameworkManager reapplies it in weather setup (idempotent `SetWindDirectionOverride`).
- JsonLoadContext: unauthored `nightVision` reads as `false` and therefore strips NVG gadgets on spawn (same absent/false collapse T-259 documented). Goldens already author `false`.
- T-654 `layers[]` / T-705 gadget flags / T-936.4 `weatherTimeline` not this slice.

## deviations

- Flatten emit for the three runtime fields was already on the tree (T-259/T-682). This slice did not re-implement emit; it added readers, editor-only comments, and a combined emit test.
- Local NVG manager named `gadgetMgr` instead of `gadgets` so T-705's unread `gadgets` baseline (6) does not trip. Do not re-pin `schema_gates.rs` (forbidden).
- Did not add rows to T-290's Fate/Ledger emit table.

## commits

- `9534104563246ca4c3de6afa89aa02100b00e181` T-291: read orphan settings, wind, and spectator policy

## manual_checklist

Human in-game (gate cannot run a round):

1. Mission with `spectatorPolicy: own_side_delayed_60s` — after death, wait ~60 s, then own-side follow-cam only; cannot spectate the enemy.
2. Mission with `spectatorPolicy: none` — dead player stays on the death view; no spectator camera / roster.
3. Mission with `spectatorPolicy: free` — spectator camera immediately; enemy players appear in the roster.
4. Mission with `nightVision: false` — spawned kits lose NVG gadgets (~1.5 s after spawn).
5. Mission with `nightVision: true` — NVG gadgets remain.
6. Mission with `environment.windDirDeg` authored — wind direction override at boot (also applied by T-682 EnvironmentReader).

## twins_confirmed

Both trees edited. Logic lockstep: FrameworkManager weather/settings/NVG + SpectatorController policy. Remaining `diff` is pre-existing ASCII punctuation (export already used `-` / `==` / `S2` vs framework `—` / `══` / `§`).
