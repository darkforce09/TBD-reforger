# REPORT T-941.8 — Vehicles spawn fuelled with inventory

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-941.8
slice/T-941.8
```

First action of the run, before any edit: `pwd && git branch --show-current`. EnfusionMCP count was 0; copied 19 `.c` from main (gitignored, not committed).

## defect_verified_on_main

Worktree tracked `8c4669ca9` at dispatch. Defect still present before the edit.

| claim | path:line | command |
|---|---|---|
| Roster spawn then T-680 authored state; no default full fuel | `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c:1054-1065` (export twin identical code) | `git show HEAD:…/TBD_SpawnManager.c \| sed -n '1054,1065p'` |
| T-680 leaves unset fuel at engine default (ABSENT sentinel) | `apps/mod/tbd-framework/Scripts/Game/TBD/Vehicles/TBD_VehicleState.c:147-161` | `Apply()` only calls `ApplyFuel` when `fuel != ABSENT` |
| No per-class cargo table / inventory insert on the spawn path | SpawnManager | no `TBD_VehicleSpawnDefaults` / no `inventory` insert after `SeatAuthoredCrews` |

Quoted (framework, pre-edit):

```
		TBD_MissionVehicleRoster.SeatAuthoredCrews(this);
		// T-680 -- authored lock / fuel / ammo. Vehicles exist only after the roster
		// join-or-spawn above; unset attributes are sentinels and leave engine defaults.
		TBD_VehicleState.ApplySpawned();
		// T-681 -- authored health / allowDamage / showModel / size (stamina logged, not applied).
		// entities[] bodies were recorded at SpawnMissionEntities; unset attrs leave defaults.
		TBD_EntityState.ApplySpawned();
```

Not already fixed. Implemented.

## changes

| path | what |
|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c` | Call site `MaterializeSlotBodies` ~1062–1067: `ApplyCargo()` then T-680 `ApplySpawned()` then `ApplyDefaultFuel()`. Helper classes at EOF: `TBD_VehicleSpawnInvRow` / `TBD_VehicleSpawnWire` (fuel ABSENT = `-1000000`, inventory presence = `Count()`) / `TBD_VehicleSpawnDoc` / `TBD_VehicleSpawnDefaults`. Class table keyed by alias class `jeep`/`truck`/`apc`/`helo`/`default`. Missing prefab: one `WARNING` naming it. Default fuel is `TBD_VehicleState.Apply(body, false, 1.0, ABSENT)` only when `!HasFuel()`. |
| `apps/mod/tbd-export/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c` | Export twin. New block is ASCII-identical to framework (existing file still has the pre-existing em-dash fold elsewhere). |

Did not edit `TBD_VehicleState.c`, `TBD_MissionLoader.c`, flatten, or schema. Crew seating left at T-675.2.

## perturbation

Broke the fuel API in the **framework** copy only: `TBD_VehicleState.Apply` → `TBD_VehicleState.ApplyFuel_T9418`.

**red_output VERBATIM** (`cargo xtask mod compile` after the rename):

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.12s
     Running `/home/Samuel/.cache/tbd-target/debug/xtask mod compile`
==> compiling tbd-framework + tbd-export (native headless server, no Workbench)

FAIL: Enfusion compile errors
------------------------------------------------------------
Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c:3764: Undefined function 'TBD_VehicleState.ApplyFuel_T9418'
------------------------------------------------------------
1 error(s) in TBD sources, 11 cascaded into vanilla.
Cascade (fix the TBD errors first; these usually vanish):
  Scripts/Game/GameMode/Loadout/SCR_PlayerArsenalLoadout.c:324: Error in parameters
  Scripts/Game/GameMode/Loadout/SCR_PlayerArsenalLoadout.c:333: Can't find variable 'sortSlots'
  Scripts/Game/GameMode/Loadout/SCR_PlayerArsenalLoadout.c:335: Can't find variable 'sortSlots'
  Scripts/Game/GameMode/Loadout/SCR_PlayerArsenalLoadout.c:337: Can't find variable 'sortSlots'
  Scripts/Game/GameMode/Loadout/SCR_PlayerArsenalLoadout.c:337: Syntax error
  Scripts/Game/ScenarioFramework/Actions/ActionGetters/SCR_ScenarioFrameworkGetCountEntitiesInTrigger.c:10: Can't find class SCR_ScenarioFrameworkParam
  Scripts/Game/ScenarioFramework/Actions/ActionGetters/SCR_ScenarioFrameworkGetLastFinishedTaskLayer.c:11: Can't find class SCR_ScenarioFrameworkParam
  Scripts/Game/UI/Components/SCR_SpinningWidgetComponent.c:25: Can't find class SCR_SpinningWidgetAnimation
  Scripts/Game/UI/Components/SCR_SpinningWidgetComponent.c:27: Can't find class SCR_SpinningWidgetAnimation
  Scripts/Game/UI/Menu/ContentBrowser/Common/SCR_ScenarioUICommon.c:452: Can't find class Tuple2
  … 1 more
red_exit=1
```

Restored the identifier, `touch`ed both `TBD_SpawnManager.c` twins.

**restored_green:**

```
OK: compiled clean
    Module: Game; loaded 5758x files; 11472x classes
    Compiling Game scripts took: 835.185000 ms
    0 warning(s) in TBD sources
```

After a cargo-before-T-680 split (same fuel API, `ApplyDefaultFuel` at ~3811): compile still `OK: compiled clean` (5758 files, 11472 classes, 0 TBD warnings). `cargo xtask mod world-boot` → `WORLD BOOT: PASS`.

## gate_verdict_tail

```
  no-python (T-620)        PASS

  gate verdict PASS @ 8c4669ca97a4 recorded: .ai/artifacts/verdicts/T-941.8.json
SLICE GATE: PASS
```

`cargo xtask platform wave gate --slice T-941.8` from the worktree. Schema arm PASS (`cargo xtask schema validate` also PASS independently; unread 1.3 baseline still 8 — `inventory` is not an `UNREAD_WIRE_FIELDS` row).

## mod_compile_verdict

`OK: compiled clean` — 5758 files, 11472 classes, 0 TBD warnings. `cargo xtask mod world-boot` → `WORLD BOOT: PASS` (roll-call includes `SpawnManager=ok`). POC boot has no `vehicles[]` roster, so spawn-cargo/spawn-fuel log lines did not fire. Did not restart :3000/:8080.

## files_outside_owns

[]

EnfusionMCP 19 `.c` copied into the worktree for compile (gitignored). Not committed.

## found_not_fixed

- In-game fuel fullness and cargo contents remain a human checklist. Compile + POC world-boot cannot prove a tank is full.
- `TBD_Dev_POC` world-boot loads no mission `vehicles[]`, so the new Print lines were not observed on boot.
- `entities[]`-only vehicles with no `vehicles[]` twin are not walked (same hole T-680 documents for lock/fuel/ammo).
- Items that load but fail `CanInsertItem`/`TryInsertItem` are deleted without a named warning. Only a **missing prefab** logs one warning naming it.
- Default table is four coarse alias classes (`jeep`/`truck`/`apc`/`helo`) plus `default`, not one row per `veh:` alias (plan: keep the table small).

## deviations

None on owns/scope. Perturbation was framework-only (same shape as T-680). Cargo/fuel call sites were split after the first green compile so T-680 ammo scales magazines inserted by this pass; fuel still runs after T-680 so authored fuel is not double-set.

## commits

`efcaf6979b27cbe997fdb0ffb69366aed7983c61` — `T-941.8: default full fuel and per-class cargo on spawn` (two `TBD_SpawnManager.c` twins). Report commit follows.

## manual_checklist

1. Spawn a roster vehicle with **no** authored `fuel` / `inventory`: tank is full; class cargo is present (jeep: jerrycan + 2× US field dressing; truck: 2× jerrycan + repair kit + 2× dressing; apc: jerrycan + repair + 2× dressing; helo: US medical kit + 2× dressing; other: jerrycan + 1× dressing).
2. Spawn a roster vehicle with authored `fuel` (including `0`): that fraction wins; this pass does not overwrite it.
3. Spawn a roster vehicle with authored `inventory` rows: those items are inserted instead of the class table.
4. Author a cargo prefab that does not exist: **one** warning names that prefab; the vehicle is still in the world.
5. Re-run `cargo xtask mod world-boot` after a mission with a real `vehicles[]` roster is loaded.

## twins_confirmed

Helper class block (`class TBD_VehicleSpawnInvRow` through EOF) is byte-identical in framework and export. Call sites `ApplyCargo` / `ApplyDefaultFuel` are identical. Remainder of `TBD_SpawnManager.c` still differs only by the pre-existing ASCII fold (em-dashes in comments/strings).
