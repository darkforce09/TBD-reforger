# REPORT T-679 — Placement scatter: radius and area shape

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-679
slice/T-679
```

First action of the run, before any edit. HEAD at start: `4b01ae415` (T-942 twin-widen). Tree left committed on `580a5304c3e9a4fcfe5419eb7f0932484f9bde8f`.

## defect_verified_on_main

The worktree was branched from main at `4b01ae415` (T-942 twin widen). Proved before writing:

| claim | path:line | command |
|---|---|---|
| `TBD_PlacementScatter.c` does not exist | (file absent both trees) | `ls apps/mod/tbd-framework/Scripts/Game/TBD/Backend/` — no PlacementScatter; same for tbd-export |
| SpawnManager never reads scatter keys | `TBD_SpawnManager.c` | `rg -n "placementRadius\|placementShape\|Scatter" apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c` — zero hits |
| Slot/group structs have no scatter members | `TBD_MissionSlotStruct.c:51-85`, `TBD_MissionLoader.c` `TBD_MissionOrbatGroupStruct` (~139) | class fields stop at identity / `leaderSlotId`; no `placementRadius` |
| UNREAD baseline is still 0 | `xtask/src/schema_gates.rs:2552-2558` | `UnreadField { name: "placementRadius", expected: 0, ticket: "T-679" }` and same for `placementShape` |
| Flatten does not emit the keys | (no matches) | `rg -n "placementRadius\|placementShape" apps/website -g '*.rs'` empty |

Not already fixed. Implemented.

## changes

| path | line | why |
|---|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_PlacementScatter.c` | NEW (352 lines). Wire structs 44–90. `Scatter` 116. `ForSlot` 139. Second parse `GetRawJson` 205. Sentinel `ABSENT = -1000000` 46/64. | T-680-style second `JsonLoadContext` pass over `slots[]` + `orbat.*.groups[]` `placementRadius`/`placementShape`. Zero radius returns `center`. Slot jitter + shared group offset. Deterministic seed from slot key. |
| `apps/mod/tbd-export/Scripts/Game/TBD/Backend/TBD_PlacementScatter.c` | NEW, byte-identical ASCII twin (12338 bytes) | T-946.26 mandatory twin. `mirror_lockstep` walks export only. |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c` | 1199–1205 | `SpawnSlotBody` (initial materialize and respawn) calls `ForSlot` before `GetSurfaceY`. |
| `apps/mod/tbd-export/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c` | 1199–1205 | Same call site. New comments ASCII. Pre-existing emdash-vs-hyphen in nearby comments left alone. |

No `packages/tbd-schema` edits. No `flatten.rs`. No `schema_gates.rs`. No `TBD_MissionLoader.c` / `TBD_MissionSlotStruct.c` / `TBD_MissionValidator.c`. No WaypointRuntime / ZoneVolume / ObjectiveRegistry / zones_panel.

### Scatter semantics

- `Scatter(center, radius, shape, seed)`: `radius <= 0` returns `center` unchanged (zero-radius identity). `square` = axis-aligned `[-radius,+radius]` on X and Z. Empty / `circle` / anything else = uniform disk (`sqrt(u)*r`).
- Slot layer: scatter around the slot's authored `(x, z)` with that slot's radius/shape. Seed = djb2 of `slot.Key()` (uid, else id).
- Group layer: one SHARED offset (seed = `faction:callsign`) added to every member, so a group radius jitters the squad together and keeps relative seats. Groups have no `x`/`z` on the wire.
- Horizontal only. Y remains SpawnSlotBody policy (JSON `y`, else surface + capsule) after the new XZ.

## perturbation

Broke `static vector Scatter(...)` → `static vector Scatter_T679(...)` in the framework copy only. `cargo xtask mod compile` RED, verbatim:

```
FAIL: Enfusion compile errors
------------------------------------------------------------
Scripts/Game/TBD/Backend/TBD_PlacementScatter.c:159: Undefined function 'TBD_PlacementScatter.Scatter'
Scripts/Game/TBD/Backend/TBD_PlacementScatter.c:173: Undefined function 'TBD_PlacementScatter.Scatter'
------------------------------------------------------------
2 error(s) in TBD sources, 6 cascaded into vanilla.
Cascade (fix the TBD errors first; these usually vanish):
  Scripts/Game/ScenarioFramework/Actions/ActionGetters/SCR_ScenarioFrameworkGetCountEntitiesInTrigger.c:10: Can't find class SCR_ScenarioFrameworkParam
  Scripts/Game/ScenarioFramework/Actions/ActionGetters/SCR_ScenarioFrameworkGetLastFinishedTaskLayer.c:11: Can't find class SCR_ScenarioFrameworkParam
  Scripts/Game/UI/Components/SCR_SpinningWidgetComponent.c:25: Can't find class SCR_SpinningWidgetAnimation
  Scripts/Game/UI/Components/SCR_SpinningWidgetComponent.c:27: Can't find class SCR_SpinningWidgetAnimation
  Scripts/Game/UI/Menu/ContentBrowser/Common/SCR_ScenarioUICommon.c:452: Can't find class Tuple2
  Scripts/Game/UI/Menu/ContentBrowser/Common/SCR_ScenarioUICommon.c:517: Can't find class Tuple2
red_exit=1
```

Restored the identifier, `touch`ed both `TBD_PlacementScatter.c` twins, twins identical again.

**restored_green:**

```
OK: compiled clean
    Module: Game; loaded 5751x files; 11411x classes
    Compiling Game scripts took: 841.419000 ms
    0 warning(s) in TBD sources
green_exit=0
```

(First green, before perturbation, was the same verdict at 860.985000 ms, 5751 files / 11411 classes.)

Zero-radius identity is the first branch of `Scatter` (`if (radius <= 0) return center;`). Absent radius uses the `-1000000` sentinel and is treated as 0, so `ForSlot` returns `Vector(x, 0, z)` and SpawnSlotBody keeps today's transform. No Enfusion unit harness exists; identity is that branch plus the compile perturbation of the function the spawn path calls.

## gate_verdict_tail

`cargo xtask platform wave gate --slice T-679` — last 15 lines:

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

  gate verdict FAIL @ 580a5304c3e9 recorded: .ai/artifacts/verdicts/T-679.json
SLICE GATE: FAIL
```

Did **not** end `SLICE GATE: PASS`. Height-labels SKIP is the worktree LFS pointer (environmental, per brief). The real failure is unread-wire-fields (next section). Brief forbade editing `xtask/src/schema_gates.rs`.

`cargo check` / wasm32 / fmt / clippy: PASS. Cheap gate did not compile Enfusion (T-946.17); that is `mod compile` below.

## mod_compile_verdict

After restore (the shippable tree):

```
OK: compiled clean
    Module: Game; loaded 5751x files; 11411x classes
    Compiling Game scripts took: 841.419000 ms
    0 warning(s) in TBD sources
```

T-946.27: 19 `Scripts/WorkbenchGame/EnfusionMCP/*.c` were already in the worktree (command-center copy). Gitignored; not committed.

## files_outside_owns

[]

## found_not_fixed

| path:line | repro |
|---|---|
| flatten.rs (no `placementRadius` hits) | Flatten does not emit `placementRadius` / `placementShape`. Live `/compiled` (`GetRawJson()`) will not scatter until T-946.36-class flatten work. Hand-staged 1.3 JSON / `golden-missions/schema-1_3-wire-fields.json` reach the reader. Brief forbade editing flatten. |
| `$defs/group` has no `x`/`z` | Group scatter is a shared offset from each member's slot `(x,z)`, not a circle around a group origin that the wire does not carry. |
| SpawnSlotBody Y policy (`TBD_SpawnManager.c` ~1208) | Scattered XZ can sit on different terrain; authored JSON `y` still wins over live `GetSurfaceY`. No navmesh clamp (no API owned by this slice). |
| `TBD_WaypointRuntime.c:388` `SpawnGroup` | AI group entity origin is still the first member after scatter. Not in owns; left alone. |

## deviations

1. **Slice gate is FAIL, not PASS.** `cargo xtask schema validate` / gate `schema` step:

```
T-706 unread 1.3 wire fields (each must stay reader-free until its ticket lands):
  FAIL  a 1.3 wire field gained a reader — update mission.schema.json
        'placementRadius' now has 10 mod identifier(s) (baseline 0) — if T-679 landed the reader, DROP the field's "no reader on any shipped build" wording in mission.schema.json and remove/repin its UNREAD_WIRE_FIELDS row; ...
        'placementShape' now has 9 mod identifier(s) (baseline 0) — if T-679 landed the reader, ...
1 validation failure(s).
```

   Brief: landing a `placementRadius` reader WILL fail `UNREAD_WIRE_FIELDS` until the command center retires the row. Expected. Did not edit `schema_gates.rs` or `mission.schema.json`.

2. No cargo tests written (owns are Enfusion `.c` only). Non-vacuity is the compile perturbation above, not a Rust test.

3. SpawnManager has no separate group-prefab spawn site (group AI entities are formed in `TBD_WaypointRuntime`, not owned). Group radius is applied in `ForSlot`, which `SpawnSlotBody` calls for every slot (and therefore every group member).

## commits

- `580a5304c3e9a4fcfe5419eb7f0932484f9bde8f` — `T-679: scatter slot and group spawns by placement radius.`

Not pushed. Not merged. Tickets/registry untouched.

## manual_checklist

IN-GAME BEHAVIOUR CANNOT BE PROVEN HERE. One human-runnable line each:

1. Hand-stage `golden-missions/schema-1_3-wire-fields.json` (slot `placementRadius` 5, group `placementRadius` 25, both `circle`) onto the dedicated server mission file, boot, and confirm Alpha bodies are not stacked on (4870, 7760) — they sit inside the authored radii.
2. Same document with both radii set to `0`: bodies spawn on the exact authored `(x, z)` as a pre-T-679 build (zero-radius identity).
3. Omit `placementRadius` / `placementShape` entirely: same exact spawn as (2).
4. Author only group `placementRadius` 20 (slot radius omitted): the squad keeps relative seats and the whole group is offset together; they do not independently fill the circle.
5. Live editor `/compiled` (flatten): scatter does **not** apply until flatten emits the keys — confirm today's compiled payload still has no `placementRadius`.

## twins_confirmed

| path | on disk |
|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_PlacementScatter.c` | yes (12338 bytes, ASCII, committed) |
| `apps/mod/tbd-export/Scripts/Game/TBD/Backend/TBD_PlacementScatter.c` | yes (byte-identical twin) |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c` | yes (`ForSlot` at 1203) |
| `apps/mod/tbd-export/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c` | yes (same call site) |
