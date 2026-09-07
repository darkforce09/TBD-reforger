# REPORT-T-302

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-302
slice/T-302
```

## defect_verified_on_main

On this worktree (main-equivalent at `b59b99116`, pre-slice):

| claim | evidence |
|---|---|
| No per-slot equip result log | `rg '\[TBD\]\[Equip\] slot='` on both `TBD_LoadoutEquipHelper.c` trees → zero hits. T-310 only logs `attach=<res> result=<ok\|failed>`. |
| Fixture has one weapon | `xtask/src/mod_world_boot.rs` `seed_fixture_body` weapons array: one `{slotIndex:0, slotType:primary, M16A2}` on `sl_ar` / `Character_US_AR`. |
| World-boot does not count weapon-slot lines | `mod_world_boot.rs` greps existing sentinels via `mod_world_boot_verdict::assess_log`; no `[TBD][Equip] slot=` assertion. A silent replace would pass. |

## changes

| path | what |
|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_LoadoutEquipHelper.c` | `LogWeaponEquipResult` prints `[TBD][Equip] slot=<n> weapon=<res> result=<ok\|replaced\|failed>` at each weapon-row IssueEquip outcome (insert=ok, TryReplace=replaced, fail/skip-fail=failed, same-prefab skip=ok). Equip APIs unchanged. |
| `apps/mod/tbd-export/Scripts/Game/TBD/Gamemode/TBD_LoadoutEquipHelper.c` | Lockstep (same CODE). Pre-existing comment punctuation twins (em-dash / `→` vs ASCII) left as-is. |
| `xtask/src/mod_world_boot.rs` | Four-weapon compiled fixture on `sl_ar` (`Character_US_Unarmed` so inserts are `result=ok`). `t302_assert` requires four `result=ok` lines covering slots 0–3 and zero replaced/failed. Wired into `--compiled` boot and `--selftest`. Helpers compacted under `#[rustfmt::skip]` to stay SIZE-3 ≤1000 (995 lines). |

Weapons (arsenal WEAPON_SLOTS pairs):

- slot 0 `primary` — `{3E413771E1834D2F}…/Rifle_M16A2.et`
- slot 1 `primary` — `{9C5C20FB0E01E64F}…/Launcher_M72A3.et`
- slot 2 `secondary` — `{1353C6EAD1DCFE43}…/Handgun_M9.et`
- slot 3 `grenade` — `{E8F00BF730225B00}…/Grenade_M67.et`

Did **not** edit SpawnManager, flatten.rs, schema, or `mod_world_boot_verdict.rs`.

## perturbation

`T302_EQUIP_OK` 4 → 3, then `cargo xtask mod world-boot --selftest`. Restore + `touch xtask/src/mod_world_boot.rs`.

**red VERBATIM** (exit 1):

```
==> T-302 four-weapon equip assertion
  FAIL  T-302 four-weapon equip: ok=4 other=0 slots=[true, true, true, true] (want ok=3 other=0 slots 0-3)
  ok    T-302 selftest rejects 3 weapons
  ok    T-302 selftest rejects replaced
```

**restored_green** (exit 0):

```
==> T-302 four-weapon equip assertion
  ok    T-302 four-weapon equip (4 ok, slots 0-3)
  ok    T-302 selftest rejects 3 weapons
  ok    T-302 selftest rejects replaced
```

## gate_verdict_tail

```
gate: lock acquired after ~600s.
touch_workspace: invalidated 658 workspace .rs file(s) and 129 include_str!/include_bytes! input(s) across 9 member(s)
  cargo check              PASS
  wasm32 (frontend)        PASS
  fmt (changed)            PASS
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

  gate verdict PASS @ b59b99116a09 recorded: .ai/artifacts/verdicts/T-302.json
SLICE GATE: PASS
```

## mod_compile_verdict

```
OK: compiled clean
    Module: Game; loaded 5755x files; 11434x classes
    Compiling Game scripts took: 1668.971000 ms
    0 warning(s) in TBD sources
```

## files_outside_owns

- `.ai/artifacts/editor_briefs/sept2026/wave252/REPORT-T-302.md` — required by the slice brief.
- `.ai/artifacts/verdicts/T-302.json` — written by the slice gate; not committed.

## found_not_fixed

`cargo xtask mod world-boot --compiled` **T-302 assert passed** (`ok T-302 four-weapon equip (4 ok, slots 0-3)`) on a live dedicated-server log. Overall `WORLD BOOT: FAIL` is the **pre-existing compiled warning ratchet**: `2 > baseline 1` (`faction:opfor` empty stub = T-299, plus `environment` unmodelled). Baseline comment in `.world-boot-warning-baseline` documents `compiled 1, boots 2` and forbids widening. Not this ticket; did not add a second faction (that would leave the T-186 one-faction pad branch untested).

## deviations

- SIZE-3: `mod_world_boot.rs` was already 1000 lines. New assert + four-weapon JSON required compacting existing helpers (`env_fail`, `api_*`, `host_bridge`, `seed_fixture_body`, …) under `#[rustfmt::skip]`. Behaviour of those helpers is unchanged.
- `--selftest` is the reproducible T-302 test (no Workbench). `--compiled` is the live proof; default `mod world-boot` (TBD_Dev_POC, no compiled fixture) does not run `t302_assert`.

## commits

See git log on `slice/T-302` after this report is committed.

## manual_checklist

1. Boot compiled mission **T-186 compiled-boot fixture** (`cargo xtask mod world-boot --compiled`, or place that compiled JSON as the server `missionId`).
2. Take slot **`sl_ar`** (role AR) — Unarmed US body with the four-weapon loadout.
3. Confirm all four carried, none replacing another:
   - engine slot 0: M16A2
   - engine slot 1: M72A3
   - engine slot 2: M9
   - engine slot 3: M67
4. Console should show four `[TBD][Equip] slot=<n> weapon=<res> result=ok` lines and no `result=replaced`.

## twins_confirmed

IssueEquip / `LogWeaponEquipResult` CODE matches on framework vs export (19 log call sites each). Remaining twin diff is historical comment punctuation only (`—`/`→`/`…` vs ASCII).
