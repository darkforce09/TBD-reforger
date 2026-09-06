# REPORT-T-677

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-677
slice/T-677
```

First command this session: `pwd && git branch --show-current` matched that path and branch. Work stayed in this worktree.

## defect_verified_on_main

Verified on this worktree at `65f4d44f3` (slice base, before the T-677 commit) — that is main as of wave 245 dispatch. Ticket cites `TBD_SpawnManager.c:963,:1166`; those line numbers have drifted.

| claim | path:line | command |
|---|---|---|
| Every slot body is AI-disabled at spawn (one call site; `SpawnSlotBody` is also the rematerialize path) | `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c:1236` `DisableBodyAI(body);` (export twin same line) | `rg -n 'DisableBodyAI\(body\)' apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c` — one call |
| `DisableBodyAI` deactivates the agent + next-frame recheck | same file `:1491` / `:1505` | `rg -n 'DeactivateAI' …/TBD_SpawnManager.c` |
| No waypoint runtime, no `AI/` folder | (files absent) | `ls apps/mod/tbd-framework/Scripts/Game/TBD/AI/` — empty |
| Loader group struct has no `waypoints` field | `TBD_MissionLoader.c:139` `TBD_MissionOrbatGroupStruct` ends at `leaderSlotId` | `rg -n 'waypoints' apps/mod/tbd-framework/Scripts/Game/TBD/Backend/` — zero hits |
| Baseline Enfusion compile clean before edits | n/a | `cargo xtask mod compile` → `OK: compiled clean` / `loaded 5742x files; 11333x classes` |

Did **not** implement T-678 (group `combatMode` / `formation` / group-level defaults). `combatMode` and `formation` UNREAD_WIRE_FIELDS rows stayed at baseline 0.

## changes

| path | line | why |
|---|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/AI/TBD_WaypointRuntime.c` | NEW (881 lines) | Second `JsonLoadContext` pass over `GetRawJson()` for `orbat.*.groups[].waypoints` (loader is T-682-owned). At LIVE, form `SCR_AIGroup` from Group_Base, `AddAIEntityToGroup` unclaimed seats, issue waypoints in document order. Type→ScenarioFramework prefab; `radiusM`→`SetCompletionRadius`; `vehicleUid`→`SCR_EntityWaypoint.SetEntity`; `cycle`→`AIWaypointCycle.SetWaypoints` infinite; `speedMode`/`behaviour`→`SCR_AIGroupCharactersMovementSpeedSetting`. Heartbeat is `modded class SCR_BaseGameMode` (same idiom as T-676). |
| `apps/mod/tbd-export/Scripts/Game/TBD/AI/TBD_WaypointRuntime.c` | NEW | Byte-identical twin (`cmp` identical, 29545 bytes, pure ASCII) |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c` | 1234–1242 | AI spawn gate: `DisableBodyAI` unless `TBD_WaypointRuntime.ShouldEnableAIAtSpawn(slot)` (waypointed group **and** stage LIVE). Lobby/safestart still parks. |
| `apps/mod/tbd-export/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c` | 1234–1242 | Same code (pre-existing comment dash already ASCII in export) |

Not touched: `packages/tbd-schema/**`, `crates/map-engine-core/src/mission/flatten.rs` (T-682), `TBD_MissionLoader.c` (T-682), editor UI, `.ai/tickets/`.

Nine ATTR-FIELD-WP ids vs T-706 wire: TYPE/ORDER/POSITION/SPEED/BEHAVIOUR/CONDITION applied. DESCRIPTION, COMBAT-MODE, FORMATION are not on `$defs/waypoint` (last two are T-678 group fields).

Interaction ids: RIGHT-MODE-004 / KEY-WP-001 / ACTION-WP-QUICK-001 → ordered `waypoints[]`. CONN-WP-ATTACH-001 → `vehicleUid` + `SetEntity`. CONN-WP-ACT-001 → LIVE is the only activation the wire can name (no trigger-to-WP link on `$defs/waypoint`). CONN-RAND-START-001 → `cycle` wraps siblings; no random flag on the wire, authored order kept.

## perturbation

Inserted `int T677_PERTURB = PleaseFailCompile;` as the first member of `TBD_WaypointRuntime` in **both** twins (lockstep still agrees so the failure is the compiler, not the mirror). RED output VERBATIM:

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.13s
     Running `/home/Samuel/.cache/tbd-target/debug/xtask mod compile`
==> compiling tbd-framework + tbd-export (native headless server, no Workbench)

FAIL: Enfusion compile errors
------------------------------------------------------------
Scripts/Game/TBD/AI/TBD_WaypointRuntime.c:140: Can't find variable 'PleaseFailCompile'
------------------------------------------------------------
1 error(s) in TBD sources, 0 cascaded into vanilla.
```

Restored with `git checkout --` of both `TBD_WaypointRuntime.c` twins, `touch` both, re-ran.

**restored_green:** `cargo xtask mod compile` → `OK: compiled clean` / `loaded 5743x files; 11348x classes` / `0 warning(s) in TBD sources`.

## gate_verdict_tail

Last 15 lines of `cargo xtask platform wave gate --slice T-677`:

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

  gate verdict FAIL @ 9eeca4adda8a recorded: .ai/artifacts/verdicts/T-677.json
SLICE GATE: FAIL
```

The only failing step was `schema`. Height-labels SKIP is the worktree LFS-pointer DEM (environmental, as briefed). Schema validate's unread-fields check:

```
T-706 unread 1.3 wire fields (each must stay reader-free until its ticket lands):
  FAIL  a 1.3 wire field gained a reader — update mission.schema.json
        'waypoints' now has 8 mod identifier(s) (baseline 0) — if T-677 landed the reader, DROP the field's "no reader on any shipped build" wording in mission.schema.json and remove/repin its UNREAD_WIRE_FIELDS row; if this is an unrelated change to the pre-existing 'clean' identifier, re-pin the baseline here on purpose
        'vehicleUid' now has 6 mod identifier(s) (baseline 0) — if T-677 landed the reader, DROP the field's "no reader on any shipped build" wording in mission.schema.json and remove/repin its UNREAD_WIRE_FIELDS row; if this is an unrelated change to the pre-existing 'clean' identifier, re-pin the baseline here on purpose
        'speedMode' now has 6 mod identifier(s) (baseline 0) — if T-678 landed the reader, DROP the field's "no reader on any shipped build" wording in mission.schema.json and remove/repin its UNREAD_WIRE_FIELDS row; if this is an unrelated change to the pre-existing 'clean' identifier, re-pin the baseline here on purpose
        'behaviour' now has 11 mod identifier(s) (baseline 0) — if T-678 landed the reader, DROP the field's "no reader on any shipped build" wording in mission.schema.json and remove/repin its UNREAD_WIRE_FIELDS row; if this is an unrelated change to the pre-existing 'English word 'behaviour' only in comments (stripped); no identifier' identifier, re-pin the baseline here on purpose
```

This is the T-676 / T-674.2 / T-682 shape: the slice gate went RED **because the ticket succeeded**. `JsonLoadContext` binds by field name, so `waypoints` / `vehicleUid` / `speedMode` / `behaviour` ARE the contract (`speedMode` and `behaviour` are on `$defs/waypoint` as well as the group; T-677 must bind them). Brief owns-list + "Do not touch packages/tbd-schema" forbids the files the gate's own message names. Did not widen. Command centre retires those four `UNREAD_WIRE_FIELDS` rows (baselines were 0 — retire, do not re-pin). `combatMode` and `formation` stayed at 0 (T-678).

## mod_compile_verdict

Baseline (before edits):

```
==> compiling tbd-framework + tbd-export (native headless server, no Workbench)
    (calibrating vanilla-only baseline, one time)
OK: compiled clean
    Module: Game; loaded 5742x files; 11333x classes
    Compiling Game scripts took: 863.461000 ms
    0 warning(s) in TBD sources
```

After ship (and after perturbation restore):

```
==> compiling tbd-framework + tbd-export (native headless server, no Workbench)
OK: compiled clean
    Module: Game; loaded 5743x files; 11348x classes
    Compiling Game scripts took: 852.738000 ms
    0 warning(s) in TBD sources
```

EnfusionMCP: 19 `.c` files present in the worktree, not committed.

## files_outside_owns

[] — no extra files edited.

Required for `SLICE GATE: PASS` (not edited; command centre):

- `xtask/src/schema_gates.rs` — retire `waypoints` / `vehicleUid` / `speedMode` / `behaviour` `UNREAD_WIRE_FIELDS` rows (`:2544`, `:2553`, `:2572`, `:2579`; all expected 0)
- `packages/tbd-schema/schema/mission.schema.json` — drop "NOTHING reads this" / "AI-DISABLED" wording on `$defs/group.waypoints` and `$defs/waypoint` (brief forbade this file)

## found_not_fixed

| path:line | repro |
|---|---|
| `xtask/src/schema_gates.rs:2544` (`waypoints` expected 0), `:2553` (`vehicleUid` 0), `:2572` (`speedMode` 0), `:2579` (`behaviour` 0) | `cargo xtask schema validate` after this reader: waypoints 8, vehicleUid 6, speedMode 6, behaviour 11. Same command the slice gate runs. |
| `packages/tbd-schema/schema/mission.schema.json:295` and `:1064` | Still say nothing reads waypoints and every body spawns AI-disabled. Both sentences are now false for a document that carries `group.waypoints[]`. |
| `crates/map-engine-core/src/mission/flatten.rs` | `rg -n waypoints flatten.rs` → zero. Editor compile still drops `group.waypoints[]`. Runtime reads whatever is already on the wire (golden `schema-1_3-wire-fields.json` has the array). Flatten is T-682-owned this wave; not edited. |
| `$defs/waypoint` has no description / combatMode / formation keys | ATTR-FIELD-WP-DESCRIPTION, ATTR-FIELD-WP-COMBAT-MODE, ATTR-FIELD-WP-FORMATION cannot be applied without inventing schema. Last two are T-678 group fields. |
| `$defs/waypoint` has no trigger id | CONN-WP-ACT-001 cannot wait on an `editorTriggers[]` row the document cannot name. Activation is LIVE. |
| `$defs/waypoint` has no random-start flag | CONN-RAND-START-001: `cycle` repeats authored order; no shuffle. |

## deviations

- Brief asked for `SLICE GATE: PASS`. Gate is FAIL on schema unread-fields **because the reader landed**. Owns list + "do not touch packages/tbd-schema" + "STOP and report, do not widen" is the T-676 instruction; followed that over the PASS line. Command centre's T-676 commit (`aa9ca991f`) / T-682 report this wave is the template.
- Ticket notes "ship together with T-678 or not at all" and executor `workbench` are stale, as the brief said. T-678 was not implemented.
- Ticket line numbers `:963,:1166` are stale; the live disable was `:1236` (now gated at `:1241`). One call site, not two — rematerialize goes through the same `SpawnSlotBody`.
- `speedMode` / `behaviour` UNREAD rows are labelled T-678 in `schema_gates.rs`, but those keys also live on `$defs/waypoint`. Binding them here is required for ATTR-FIELD-WP-SPEED / ATTR-FIELD-WP-BEHAVIOUR. Command centre should retire those rows (reader exists), not leave them for T-678.

## commits

- `9eeca4adda8a56effd72275b4ba0a4d261089503` T-677: waypoint runtime and AI spawn gate for waypointed groups.

HEAD `9eeca4adda8a56effd72275b4ba0a4d261089503`. Worktree clean. No push.

## manual_checklist

- Dedicated-server boot `packages/tbd-schema/golden-missions/schema-1_3-wire-fields.json`. Confirm log `parsed waypointed groups=` ≥ 1. Leave Alpha unclaimed, `#tbd stage LIVE`. Confirm unclaimed Alpha bodies ActivateAI, board `veh_m151` on the `get_in` WP, then walk the `move` then `seek_and_destroy` WPs in that order (headless compile cannot prove pathing).
- Same mission, claim an Alpha seat as a player before LIVE: that body must stay player-controlled (no AI walk); any remaining unclaimed Alpha seats still follow the WPs.
- Boot a 1.1/1.2 compiler-shaped golden with no `waypoints` key. Confirm `parsed waypointed groups=0` and every body stays AI-disabled through LIVE.
- During LOBBY / SAFE_START on the 1.3 golden: waypointed unclaimed bodies must still stand parked; they only arm at LIVE.

## twins_confirmed

| path | on disk |
|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/AI/TBD_WaypointRuntime.c` | yes (29545 bytes) |
| `apps/mod/tbd-export/Scripts/Game/TBD/AI/TBD_WaypointRuntime.c` | yes (29545 bytes, `cmp` identical, pure ASCII) |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c` | yes (edited) |
| `apps/mod/tbd-export/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c` | yes (edited; code agrees after comment-strip lockstep; export comments already used ASCII hyphens in the CRF hunk) |
