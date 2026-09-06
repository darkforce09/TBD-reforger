# REPORT T-705 — Player gadget flags: map, compass, watch, GPS, radio

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-705
slice/T-705
```

First action of the run, before any edit. HEAD at start: `2eff41f30` (T-942 wave 250 briefs). Code commit `85ad2358a9629ada22565b30b7759e85d780d7aa`. This report is a follow-on commit on the same branch.

## defect_verified_on_main

The worktree was branched from main at the T-942 wave-250 brief commit. Proved before writing:

| claim | path:line | command |
|---|---|---|
| `TBD_GadgetFlags.c` does not exist | (file absent both trees) | `ls apps/mod/tbd-framework/Scripts/Game/TBD/Backend/` — no GadgetFlags; same for tbd-export |
| Slot struct has no `gadgets` member | `TBD_MissionSlotStruct.c` fields stop at identity / loadout | `rg gadgets` over Backend `.c` — zero hits |
| MissionLoader never binds gadgets | `TBD_MissionLoader.c` | `rg gadgets` — zero hits |
| Wire exists since T-706 | `packages/tbd-schema/schema/mission.schema.json:464` `$defs/slot.gadgets` → `#/$defs/gadgetFlags` (`map`/`compass`/`watch`/`gps`/`radio`) | golden `schema-1_3-wire-fields.json:290` authors the block |
| UNREAD baseline | `xtask/src/schema_gates.rs` | `compass`/`watch`/`gps` expected 0; `gadgets` expected 6 (radio subsystem, unrelated) |

Not already fixed. Implemented.

## changes

| path | line | why |
|---|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_GadgetFlags.c` | NEW (494 lines). Wire `map<string,bool> gadgets` 33. `Bind` 73. `OnPlayerSpawned` 98. `ApplyToBody` 134. Second parse `GetRawJson` in `Parse`. | T-679-style second `JsonLoadContext` pass over `slots[].gadgets`. Authored false withholds; authored true ensures; omit (`Count()==0` / `Find` miss) keeps kit defaults. Apply 800 ms after player spawn so loadout cargo (`VerifyTick` 500 ms) cannot put a withheld gadget back. |
| `apps/mod/tbd-export/Scripts/Game/TBD/Backend/TBD_GadgetFlags.c` | NEW, byte-identical ASCII twin (16060 bytes, `non_ascii=0`) | T-946.26 mandatory twin. `mirror_lockstep` walks export only. |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionLoader.c` | 1127 `TBD_GadgetFlags.Bind()` | After `TBD_EnvironmentReader.Apply()`, before `TBD_MissionParams.Resolve()`. Struct **not** grown. |
| `apps/mod/tbd-export/Scripts/Game/TBD/Backend/TBD_MissionLoader.c` | 1127 | Same call site, ASCII comments. |

No `packages/tbd-schema` edits. No `flatten.rs`. No `schema_gates.rs`. No `TBD_MissionSlotStruct.c`. No `TBD_SpawnManager.c`. No editor panel.

### Bind / presence

`JsonLoadContext` allocates a nested `ref` when the key is absent, and bools cannot carry a sentinel (T-676 / T-946.37). A struct of five bools would bind omit and `{ "map": false }` the same (all false) and **strip gadgets on every mission that never authored the block**. `gadgets` is therefore `ref map<string, bool>`:

- `Count() < 1` — omit or `{}` → keep kit defaults
- `Find(key, value)` — per-flag presence; the bool is on/off

### Apply mapping

| flag | withhold (authored false) | ensure (authored true) |
|---|---|---|
| `map` | `GetGadgetsByType(EGadgetType.MAP)` + `DeleteEntityAndChildren` | insert `Prefabs/Items/Core/Map_Base.et` if none |
| `compass` | `EGadgetType.COMPASS` | insert `Compass_SY183.et` if none |
| `watch` | inventory prefab path contains `Watch` | insert `Watch_Vostok.et` if none |
| `gps` | inventory prefab path contains `GPS`/`Gps` | **cannot add** — no GPS item in the TBD registry; logs and keeps kit default |
| `radio` | `EGadgetType.RADIO` and `RADIO_BACKPACK` | insert `Radio_R148.et` if neither class is present |

`EGadgetType.WATCH` does not exist on this engine (compile error `Can't find variable 'WATCH'`). GPS was not probed as an enum member after that; withhold uses the prefab needle.

Spawn apply is **not** a SpawnManager call (not in owns). `Bind` inserts `GetOnPlayerSpawned` and resolves the seat via `TBD_SpawnManager.GetInstance().GetAssignedSlot`.

## perturbation

Broke `static void ApplyToBody(...)` → `static void ApplyToBody_T705(...)` in the **framework** copy only. `cargo xtask mod compile` RED, verbatim:

```
FAIL: Enfusion compile errors
------------------------------------------------------------
Scripts/Game/TBD/Backend/TBD_GadgetFlags.c:130: Undefined function 'TBD_GadgetFlags.ApplyToBody'
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
```

Restored the identifier, copied the twin, `touch`ed both `TBD_GadgetFlags.c` files.

**restored_green:**

```
OK: compiled clean
    Module: Game; loaded 5754x files; 11424x classes
    Compiling Game scripts took: 863.157000 ms
    0 warning(s) in TBD sources
```

(First green, before perturbation, was the same verdict at 865.902000 ms, 5754 files / 11424 classes.)

## gate_verdict_tail

`cargo xtask platform wave gate --slice T-705` — last 15 lines:

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

  gate verdict FAIL @ 85ad2358a962 recorded: .ai/artifacts/verdicts/T-705.json
SLICE GATE: FAIL
```

Did **not** end `SLICE GATE: PASS`. Height-labels SKIP is the worktree LFS pointer (environmental, per brief). The real failure is unread-wire-fields (next section). Brief forbade editing `xtask/src/schema_gates.rs`. UNREAD FAIL on compass/watch/gps is expected.

`cargo check` / wasm32 / fmt / clippy: PASS. Cheap gate did not compile Enfusion (T-946.17); that is `mod compile` below.

## mod_compile_verdict

After restore (the shippable tree):

```
OK: compiled clean
    Module: Game; loaded 5754x files; 11424x classes
    Compiling Game scripts took: 863.157000 ms
    0 warning(s) in TBD sources
```

T-946.27: 19 `Scripts/WorkbenchGame/EnfusionMCP/*.c` were already in the worktree (command-center copy). Gitignored; not committed.

## files_outside_owns

[]

## found_not_fixed

| path:line | repro |
|---|---|
| flatten.rs (no `gadgets` hits under `apps/website` / `crates`) | Flatten does not emit `slot.gadgets`. Live `/compiled` (`GetRawJson()`) will not apply flags until T-946.36-class flatten work. Hand-staged 1.3 JSON / `golden-missions/schema-1_3-wire-fields.json` reach the reader. Brief forbade editing flatten. |
| `EGadgetType.WATCH` | Compile: `Can't find variable 'WATCH'`. Watch withhold/ensure uses inventory prefab-path needle `"Watch"`. |
| GPS item registry | `rg GPS` over `packages/tbd-schema/registry/registry-items.workbench.json` is empty. Authored `gps: true` cannot add a GPS; logs and keeps the kit. Authored `gps: false` still withholds a GPS-named item if one is present. |
| `TBD_SpawnManager.c` (not in owns) | Slot bodies sitting unpossessed keep kit gadgets until a player takes the seat (`OnPlayerSpawned`). Brief: stop rather than edit a file we do not own. |

## deviations

1. **Slice gate is FAIL, not PASS.** `cargo xtask schema validate` / gate `schema` step:

```
T-706 unread 1.3 wire fields (each must stay reader-free until its ticket lands):
  FAIL  a 1.3 wire field gained a reader — update mission.schema.json
        'compass' now has 3 mod identifier(s) (baseline 0) — if T-705 landed the reader, DROP the field's "no reader on any shipped build" wording in mission.schema.json and remove/repin its UNREAD_WIRE_FIELDS row; ...
        'watch' now has 3 mod identifier(s) (baseline 0) — if T-705 landed the reader, ...
        'gps' now has 3 mod identifier(s) (baseline 0) — if T-705 landed the reader, ...
        'gadgets' now has 33 mod identifier(s) (baseline 6) — if T-705 landed the reader, ...
1 validation failure(s).
```

   Brief: landing a real reader WILL fail `UNREAD_WIRE_FIELDS` until the command center retires the row. Expected. Did not edit `schema_gates.rs` or `mission.schema.json`. **Do not re-pin `gadgets`.** New count is **33** (was 6 radio/gadget subsystem identifiers).

2. No cargo tests written (owns are Enfusion `.c` only). Non-vacuity is the compile perturbation above, not a Rust test.

3. Watch/GPS are not `EGadgetType` members on this engine; map/compass/radio are.

## commits

- `85ad2358a9629ada22565b30b7759e85d780d7aa` — `T-705: apply authored player gadget flags after spawn.`
- report commit on this branch — `T-705: record wave 250 slice report.`

Not pushed. Not merged. Tickets/registry untouched.

## manual_checklist

IN-GAME BEHAVIOUR CANNOT BE PROVEN HERE. One human-runnable line each:

1. Hand-stage `golden-missions/schema-1_3-wire-fields.json` (slot `slot_sl` has `gps: false`, others true) onto the dedicated server mission file, boot, possess the SL seat, confirm **no GPS** after spawn and map/compass/watch/radio still present if the kit had them or they were added.
2. Same document, slot `slot_rfl` authors only `{ "map": false }`: that player has **no map**; compass/watch/gps/radio follow the kit (those keys were omitted).
3. Omit `gadgets` entirely: loadout/kit gadgets are unchanged from a pre-T-705 build.
4. Author `radio: false` on an SL kit that carries a radio: after the 800 ms post-loadout delay the handheld/backpack radio is gone.
5. Live editor `/compiled` (flatten): flags do **not** apply until flatten emits `gadgets` — confirm today's compiled payload still has no `slot.gadgets`.

## twins_confirmed

| path | on disk |
|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_GadgetFlags.c` | yes (16060 bytes, ASCII, committed) |
| `apps/mod/tbd-export/Scripts/Game/TBD/Backend/TBD_GadgetFlags.c` | yes (byte-identical twin) |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionLoader.c` | yes (`Bind()` at 1127) |
| `apps/mod/tbd-export/Scripts/Game/TBD/Backend/TBD_MissionLoader.c` | yes (same call site) |
