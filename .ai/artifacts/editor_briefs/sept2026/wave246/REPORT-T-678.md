# REPORT-T-678

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-678
slice/T-678
```

First command this session: `pwd && git branch --show-current` matched that path and branch. Work stayed in this worktree. Native host `cargo`. `CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target`. Dedicated server engine version **192142**.

## defect_verified_on_main

Verified on this worktree at `475aec391` (slice base, before the T-678 commit) — T-677 is already on that tree.

| claim | path:line | command |
|---|---|---|
| `TBD_GroupState.c` does not exist in either tree | `apps/mod/tbd-framework/Scripts/Game/TBD/AI/` contained only `TBD_WaypointRuntime.c` | `ls apps/mod/tbd-framework/Scripts/Game/TBD/AI/` and the export twin — no GroupState |
| Loader group struct has no combatMode / behaviour / formation / speedMode | `TBD_MissionLoader.c:139-159` `TBD_MissionOrbatGroupStruct` ends at `leaderSlotId` | `python3` print of those lines; `rg -n 'combatMode' apps/mod/tbd-framework/Scripts/Game/TBD/Backend/` — zero hits |
| Group-level combatMode/formation have no Enfusion reader | WP comments only in `TBD_WaypointRuntime.c:6,:42` | `rg -n 'combatMode\|formation' apps/mod/tbd-framework --glob '*.c'` — no identifier, only T-678-forward comments and unrelated "formation" geology prose |
| T-706 already widened `$defs/group` | `packages/tbd-schema/schema/mission.schema.json:268-296` enums blue..red / careless..stealth / column..diamond / limited\|normal\|full | not edited (T-706 owns) |
| Golden already authors all four on Alpha | `packages/tbd-schema/golden-missions/schema-1_3-wire-fields.json:45-48` `combatMode=yellow` `behaviour=aware` `formation=wedge` `speedMode=normal`; Grom has none | not edited |
| Baseline Enfusion compile clean before edits | n/a | first `cargo xtask mod compile` after the new files (APIs proven live): `OK: compiled clean` / `loaded 5747x files; 11375x classes` |

Did **not** reimplement waypoints. Did **not** touch `TBD_SpawnManager.c` (T-680) or `TBD_MissionLoader.c` (T-684). Did **not** fix T-946.34.

## changes

| path | line | why |
|---|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/AI/TBD_GroupState.c` | NEW (524 lines) | Second `JsonLoadContext` pass over `GetRawJson()` (`:156`) for `orbat.*.groups[]` `combatMode`/`behaviour`/`formation`/`speedMode`. Presence is `IsEmpty()` (`HasAnyAttr` `:55`). At LIVE, find the T-677 `SCR_AIGroup` already parenting a slot body (`FindLiveGroup` `:286`) and apply defaults. Heartbeat is `modded class SCR_BaseGameMode` (`:490`). |
| `apps/mod/tbd-export/Scripts/Game/TBD/AI/TBD_GroupState.c` | NEW | Byte-identical twin (`cmp` identical, 17537 bytes, pure ASCII) |

Exact API calls (engine 192142):

- Combat: `SCR_AIGroupUtilityComponent.SetCombatMode(EAIGroupCombatMode)` (`:343`). Map: blue/green → `HOLD_FIRE`, white → `RETURN_FIRE`, yellow/red → `FIRE_AT_WILL`.
- Formation: `AIFormationComponent.SetFormation(SCR_Enum.GetEnumName(SCR_EAIGroupFormation, …))` (`:390`). Engine enum is four names: `Wedge`, `Line`, `Column`, `StaggeredColumn`. TBD extras: vee/diamond → Wedge, echelon_left/right → Line, file → Column, stagger_column → StaggeredColumn.
- Speed: `SCR_AIGroupCharactersMovementSpeedSetting.Create(SCR_EAISettingOrigin.DEFAULT, EMovementType)` then `SCR_AIGroupSettingsComponent.AddSetting(setting, false, true)` (`:437-442`). Origin DEFAULT priority 1000; T-677 waypoint settings are WAYPOINT priority 4000, so a waypoint that authors `speedMode`/`behaviour` still wins for that waypoint. limited→WALK, normal→RUN, full→SPRINT.
- Behaviour: no engine behaviour-setting class (T-677 measured this). When `speedMode` is absent, behaviour selects the same speed ceiling as `TBD_WaypointRuntime.SpeedFromWire` (careless/safe/stealth walk, aware run, combat sprint). When `speedMode` is authored, it wins for speed.

Groups with none of the four attrs are never collected (engine defaults). Groups whose bodies have no parent `SCR_AIGroup` are skipped until T-677 (or anyone) AI-enables them — this slice does not spawn groups and does not ActivateAI.

Not touched: `packages/tbd-schema/**`, `flatten.rs`, `TBD_SpawnManager.c`, `TBD_MissionLoader.c`, `xtask/src/schema_gates.rs`, editor UI, `.ai/tickets/`.

## perturbation

Inserted `int T678_PERTURB = PleaseFailCompile;` as the first member of `TBD_GroupState` in **both** twins (lockstep still agrees so the failure is the compiler, not the mirror). RED output VERBATIM:

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.13s
     Running `/home/Samuel/.cache/tbd-target/debug/xtask mod compile`
==> compiling tbd-framework + tbd-export (native headless server, no Workbench)

FAIL: Enfusion compile errors
------------------------------------------------------------
Scripts/Game/TBD/AI/TBD_GroupState.c:96: Can't find variable 'PleaseFailCompile'
------------------------------------------------------------
1 error(s) in TBD sources, 0 cascaded into vanilla.
```

Files were untracked, so restore was a strip of that member (not `git checkout`), then `cp` framework → export, `touch` both, re-ran.

**restored_green:** `cargo xtask mod compile` → `OK: compiled clean` / `loaded 5747x files; 11375x classes` / `0 warning(s) in TBD sources`.

## gate_verdict_tail

Last 15 lines of `cargo xtask platform wave gate --slice T-678`:

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

  gate verdict FAIL @ 6826c010397f recorded: .ai/artifacts/verdicts/T-678.json
SLICE GATE: FAIL
```

Height-labels SKIP is the worktree LFS-pointer DEM (environmental, as briefed). Schema validate's unread-fields check (`cargo xtask schema validate`):

```
T-706 unread 1.3 wire fields (each must stay reader-free until its ticket lands):
  FAIL  a 1.3 wire field gained a reader — update mission.schema.json
        'combatMode' now has 9 mod identifier(s) (baseline 0) — if T-678 landed the reader, DROP the field's "no reader on any shipped build" wording in mission.schema.json and remove/repin its UNREAD_WIRE_FIELDS row; if this is an unrelated change to the pre-existing 'clean' identifier, re-pin the baseline here on purpose
        'formation' now has 17 mod identifier(s) (baseline 0) — if T-678 landed the reader, DROP the field's "no reader on any shipped build" wording in mission.schema.json and remove/repin its UNREAD_WIRE_FIELDS row; if this is an unrelated change to the pre-existing 'clean' identifier, re-pin the baseline here on purpose
```

This is the T-677 shape: the slice gate went RED **because the ticket succeeded**. `JsonLoadContext` binds by field name, so `combatMode` / `formation` ARE the contract. Brief owns-list + "Do not edit xtask/src/schema_gates.rs" / "Do not touch packages/tbd-schema" forbids the files the gate's own message names. Did not widen. Command centre retires those two `UNREAD_WIRE_FIELDS` rows (baselines were 0 — retire, do not re-pin). `speedMode` / `behaviour` unread rows were already retired with T-677.

## mod_compile_verdict

After ship (and after perturbation restore):

```
==> compiling tbd-framework + tbd-export (native headless server, no Workbench)
OK: compiled clean
    Module: Game; loaded 5747x files; 11375x classes
    Compiling Game scripts took: 857.892000 ms
    0 warning(s) in TBD sources
```

EnfusionMCP: 19 `.c` files copied into the worktree before the first compile, not committed.

## files_outside_owns

[] — no extra files edited.

Required for `SLICE GATE: PASS` (not edited; command centre):

- `xtask/src/schema_gates.rs` — retire `combatMode` / `formation` `UNREAD_WIRE_FIELDS` rows (expected 0)
- `packages/tbd-schema/schema/mission.schema.json` — drop "On the wire only" / "ZERO in both the frontend and the mod tree" wording on `$defs/group.combatMode` and `$defs/group.formation` (brief forbade this file)

## found_not_fixed

| path:line | repro |
|---|---|
| `xtask/src/schema_gates.rs` `combatMode` expected 0, `formation` expected 0 | `cargo xtask schema validate` after this reader: combatMode 9, formation 17. Same command the slice gate runs. |
| `packages/tbd-schema/schema/mission.schema.json:271` and `:291` | Still say combatMode/formation are on the wire only / no Enfusion reader. Both sentences are now false for a document that carries those keys on a group that T-677 AI-enables. |
| Groups with GRP attrs but **no** `waypoints[]` | T-677 still parks every body (`ShouldEnableAIAtSpawn` is waypointed+LIVE only). This slice finds no `SCR_AIGroup` parent and does not spawn one. Attrs are parsed and logged (`parsed groups with AI state=N`) but cannot be applied until something else AI-enables the group. Shared AI gate, as briefed. |
| T-946.34 claimed-seat spawn gate | Explicitly out of this slice. |

## deviations

- Brief asked for `SLICE GATE: PASS`. Gate is FAIL on schema unread-fields **because the reader landed**. Owns list + "Do NOT edit xtask/src/schema_gates.rs" + "A real identifier will FAIL the schema gate. Expected." is the instruction; followed that over the PASS line. T-677 report this wave is the template.
- Ticket notes that executor is `workbench` or "ship together with T-677" are stale, as the brief said. T-677 was already on the slice base.
- Engine formation enum has four names; schema has nine tokens. Extras are mapped (vee/diamond→Wedge, echelons→Line, file→Column), not invented as new engine formations.

## commits

- `6826c010397f0001288cff71aef9e95f1482ad92` T-678: apply group combatMode, behaviour, formation, speedMode after spawn.

HEAD `6826c010397f0001288cff71aef9e95f1482ad92`. Worktree clean. No push.

## manual_checklist

- Dedicated-server boot `packages/tbd-schema/golden-missions/schema-1_3-wire-fields.json`. Confirm log `parsed groups with AI state=` ≥ 1 (Alpha has all four attrs; Grom has none). Leave Alpha unclaimed, `#tbd stage LIVE`. After T-677 arms Alpha, confirm log `applied blufor:Alpha combatMode='yellow' behaviour='aware' formation='wedge' speedMode='normal'`. Yellow must be FIRE_AT_WILL (they shoot); wedge must be the group formation; default movement must be RUN when not on a waypoint that authors speed.
- Same mission, watch the authored `move` waypoint (`behaviour=combat`, `speedMode=full`): that waypoint must SPRINT (T-677 WAYPOINT origin), not stay on the group RUN default. After that waypoint, group default RUN is allowed to return.
- Same mission, Grom (no GRP attrs): no `applied opfor:Grom` line; engine defaults remain (no SetCombatMode / SetFormation / DEFAULT speed setting from this file).
- Boot a 1.1/1.2 compiler-shaped golden with no group AI keys. Confirm `parsed groups with AI state=0` and no `applied` lines through LIVE.
- Author a waypointed group with `combatMode=blue` (HOLD_FIRE) and confirm they do not open fire until engaged under RETURN_FIRE/FIRE_AT_WILL rules; contrast with Alpha yellow. Headless compile cannot prove ROE.
- Author `formation=stagger_column` on a waypointed group and confirm StaggeredColumn on the live group vs Alpha wedge.

## twins_confirmed

| path | on disk |
|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/AI/TBD_GroupState.c` | yes (17537 bytes, ASCII) |
| `apps/mod/tbd-export/Scripts/Game/TBD/AI/TBD_GroupState.c` | yes (17537 bytes, ASCII, `cmp` identical) |
