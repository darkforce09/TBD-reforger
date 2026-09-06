# REPORT T-680 — Vehicle states: lock, fuel, ammo

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-680
slice/T-680
```

First action of the run, before any edit. Tree left committed and clean on `0132ecacef6e9647bb8ce81e3904142972aea17f`.

## defect_verified_on_main

The worktree was branched from main at `475aec391` (T-942 twin widen). Proved before writing:

| claim | path:line | command |
|---|---|---|
| `TBD_VehicleState.c` does not exist | (file absent both trees) | `test ! -f apps/mod/tbd-framework/Scripts/Game/TBD/Vehicles/TBD_VehicleState.c` → MISSING_FW; same for tbd-export → MISSING_EX |
| No vehicle-lock / fuel-set / ammo-set reader in `apps/mod` | no hits | python walk of `apps/mod/tbd-framework/Scripts/**/*.c`: zero files contain `TBD_VehicleState` or `LockPilotControls` or `SetTotalFuelPercentage` or `SetAmmoCount` |
| T-675.2 spawn exists but does not apply lock/fuel/ammo | `TBD_SpawnManager.c:1028` | `TBD_MissionVehicleRoster.SeatAuthoredCrews(this);` then the `built <= 0` return — no Apply |
| Word-boundary `fuel`/`lock`/`ammo` in spawn path is not vehicle state | SpawnManager / MissionVehicleStruct | schema `UNREAD_WIRE_FIELDS` baseline 0 for all three; `TBD_MissionVehicleStruct` has no lock/fuel/ammo members (class starts ~line 68, fields alias/uid/x/z/headingDeg/faction/seats only) |

Not already fixed. Implemented.

## changes

| path | line | why |
|---|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/Vehicles/TBD_VehicleState.c` | NEW (365 lines). Wire structs 39–83. `ApplySpawned` 98. `Apply(vehicle, lock, fuel, ammo)` 149. Second parse `GetRawJson` 169. `LockPilotControls` 237. `SetFuel` 282. `SetAmmoCount` 363. | T-677-style second `JsonLoadContext` pass over `vehicles[]` lock/fuel/ammo. Apply at spawn. Unset numerics are `ABSENT = -1000000` and leave engine defaults. |
| `apps/mod/tbd-export/Scripts/Game/TBD/Vehicles/TBD_VehicleState.c` | NEW, byte-identical ASCII twin | T-946.26 mandatory twin. `mirror_lockstep` walks export only. |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c` | 1029–1031 | After `SeatAuthoredCrews` (vehicles now exist), `TBD_VehicleState.ApplySpawned();` before the `built <= 0` return so a vehicle-only roster still applies. |
| `apps/mod/tbd-export/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c` | 1029–1031 | Same call site, ASCII comments. |

No `packages/tbd-schema` edits. No editor UI. No `flatten.rs`. No `TBD_MissionLoader.c`. No `TBD_MissionVehicleStruct.c`.

### Ammo scope (plan: document the chosen scope)

- **Turret / hull weapons:** `BaseWeaponManagerComponent.GetWeapons` → `BaseWeaponComponent.GetCurrentMagazine` → `SetAmmoCount(round(max * fraction))` on the vehicle **and** `SlotManagerComponent` attached entities (turret slots).
- **Cargo magazines:** `BaseInventoryStorageComponent.GetAll(items, true)` (child components included so a mag in a weapon well still counts) → items with `BaseMagazineComponent`.
- **Out of scope:** loose world magazines not in this vehicle's inventory; magazine entities that are neither current-weapon nor cargo.

### Lock semantics

`lock` is `VehicleControllerComponent.LockPilotControls` (pilot controls), matching the plan's "vehicle controller" and vanilla `SCR_VehicleLockControlsAction`. Not door/compartment lock.

Bool presence: same as T-676 `repeat`. Absent and authored-false bind identically. Apply lock **only when the bound value is true**. Authored `lock: false` cannot unlock a prefab that spawns already locked.

### Fuel

`FuelManagerComponent.GetFuelNodesList` + `BaseFuelNode.SetFuel(max * fraction)` on the vehicle and slotted attachments. Fraction 0 is authored empty (sentinel is `-1000000`, not 0).

## perturbation

Broke `LockPilotControls` → `LockPilotControls_T680` in the framework copy only. `cargo xtask mod compile` RED, verbatim:

```
FAIL: Enfusion compile errors
------------------------------------------------------------
Scripts/Game/TBD/Vehicles/TBD_VehicleState.c:237: Undefined function 'VehicleControllerComponent.LockPilotControls_T680'
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

Restored the identifier, `touch`ed `TBD_VehicleState.c`, twins identical again.

**restored_green:**

```
OK: compiled clean
    Module: Game; loaded 5747x files; 11368x classes
    Compiling Game scripts took: 851.161000 ms
    0 warning(s) in TBD sources
green_exit=0
```

(First green, before perturbation, was the same verdict at 851.605000 ms.)

## gate_verdict_tail

`cargo xtask platform wave gate --slice T-680` — last 15 lines:

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

  gate verdict FAIL @ 0132ecacef6e recorded: .ai/artifacts/verdicts/T-680.json
SLICE GATE: FAIL
```

Did **not** end `SLICE GATE: PASS`. Sole red step: `schema`. Height-labels SKIP is the worktree LFS pointer (environmental, per brief). The real failure is unread-wire-fields (next section). Brief forbade editing `xtask/src/schema_gates.rs`.

`cargo check` / wasm32 / fmt / clippy: PASS. Cheap gate did not compile Enfusion (T-946.17); that is `mod compile` below.

## mod_compile_verdict

After restore (the shippable tree):

```
OK: compiled clean
    Module: Game; loaded 5747x files; 11368x classes
    Compiling Game scripts took: 851.161000 ms
    0 warning(s) in TBD sources
```

T-946.27: copied 19 `Scripts/WorkbenchGame/EnfusionMCP/*.c` from the main checkout into the worktree before the first compile. Gitignored; not committed.

## files_outside_owns

[]

## found_not_fixed

| path:line | repro |
|---|---|
| `TBD_VehicleStateWireStruct.lock` (`TBD_VehicleState.c:49`) | Authored `lock: false` vs omitted `lock` are indistinguishable (`JsonLoadContext` bool default). Apply only on `true`. A prefab that spawns locked cannot be unlocked by authored false. Same class as T-676 `repeat`. |
| `vehicles[]` only (`TBD_VehicleStateDocStruct` line 80–83) | Brief: second pass "declares `vehicles[]` lock/fuel/ammo". `$defs/entity` also has lock/fuel/ammo. A vehicle that exists only as an `entities[]` row with no `vehicles[]` twin will not get state. T-675.2 authored vehicles emit both rows; this is the residual hole for a hand-edited entities-only document. |

## deviations

1. **Slice gate is FAIL, not PASS.** `cargo xtask schema validate` / gate `schema` step:

```
T-706 unread 1.3 wire fields (each must stay reader-free until its ticket lands):
  FAIL  a 1.3 wire field gained a reader — update mission.schema.json
        'fuel' now has 15 mod identifier(s) (baseline 0) — if T-680 landed the reader, DROP the field's "no reader on any shipped build" wording in mission.schema.json and remove/repin its UNREAD_WIRE_FIELDS row; ...
        'lock' now has 6 mod identifier(s) (baseline 0) — if T-680 landed the reader, ...
        'ammo' now has 18 mod identifier(s) (baseline 0) — if T-680 landed the reader, ...
1 validation failure(s).
```

   Brief: "UNREAD_WIRE_FIELDS `fuel`/`lock`/`ammo` expected 0. A real identifier will FAIL the schema gate. Expected. Do NOT edit `xtask/src/schema_gates.rs`." JSON binding requires those member names. Command center must retire/repin the three UNREAD rows (same as T-677 `waypoints` / T-675.2 `vehicles`) and drop the schema "on the wire only" wording. I did not touch `schema_gates.rs` or `mission.schema.json`.

2. No cargo tests written (owns are Enfusion `.c` only). Non-vacuity is the compile perturbation above, not a Rust test.

## commits

- `0132ecacef6e9647bb8ce81e3904142972aea17f` — `T-680: apply authored vehicle lock, fuel, and ammo at spawn.`

Working tree clean. Not pushed. Not merged. Tickets/registry untouched.

## manual_checklist

IN-GAME BEHAVIOUR CANNOT BE PROVEN HERE. One human-runnable line each:

1. Author a roster vehicle with `"lock": true`, boot the dedicated server on that mission, get in as driver: pilot controls are locked (`ArePilotControlsLocked` true).
2. Author the same vehicle with `"fuel": 0.5` (and no lock): fuel nodes read ~50% of max (`GetFuel()/GetMaxFuel()`), not prefab-full.
3. Author `"ammo": 0.5`: currently loaded turret magazine `GetAmmoCount()` is half of `GetMaxAmmoCount()`, and cargo magazines in vehicle inventory match the same fraction.
4. Omit `lock`/`fuel`/`ammo` entirely: vehicle matches prefab defaults (unlocked, full fuel, full mags) — same as pre-T-680.
5. Author `"fuel": 0` (empty tank, not omitted): vehicle spawns with empty fuel nodes, not the ABSENT no-op path.

## twins_confirmed

| path | on disk |
|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/Vehicles/TBD_VehicleState.c` | yes (12308 bytes, ASCII, committed) |
| `apps/mod/tbd-export/Scripts/Game/TBD/Vehicles/TBD_VehicleState.c` | yes (byte-identical twin) |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c` | yes (ApplySpawned at 1031) |
| `apps/mod/tbd-export/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c` | yes (same call site) |
