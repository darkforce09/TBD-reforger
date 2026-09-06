# REPORT T-689 — play-area vehicleClasses bind + apply

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-689
slice/T-689
```

## defect_verified_on_main

| claim | path:line | command |
|---|---|---|
| No `.c` reader of the wire key `vehicleClasses`. Schema 1.3 already declares it (T-706). `TBD_MissionZoneRulesStruct` ended at `startingOwner`; play-area apply treated every occupant alike. | `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionLoader.c` struct `TBD_MissionZoneRulesStruct` (pre-change last field `startingOwner`; parent `b422aac85`) | `rg -n 'vehicleClasses' apps/mod --glob '*.c'` → only `TBD_ZoneVolume.c:49` comments about AGL excluding aircraft overhead, **zero identifiers**. `git grep -n vehicleClasses b422aac85 -- 'apps/mod/**/*.c'` → empty. |
| UNREAD baseline 0 is still the gate's expectation until command center retires the row. | `xtask/src/schema_gates.rs` `UnreadField { name: "vehicleClasses", expected: 0, ticket: "T-689" }` | `cargo xtask schema validate` **after** the reader: FAIL, 6 identifiers (see perturbation). |

## changes

| path | line | why |
|---|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/Zones/TBD_PlayAreaVehicleAxis.c` (new, ASCII) | 1–231 | Bind `vehicleClasses`, classify occupant (`infantry` / `ground` / `aircraft` / `sea`), `EffectivePenalty`, apply helper `OccupantConfinedByZone`. |
| `apps/mod/tbd-export/Scripts/Game/TBD/Zones/TBD_PlayAreaVehicleAxis.c` | same | Export twin; byte-identical to framework. |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionLoader.c` | 120–124 | `ref array<string> vehicleClasses` on `TBD_MissionZoneRulesStruct`. Presence is `Count()` (allocated empty array when the key is absent). |
| `apps/mod/tbd-export/Scripts/Game/TBD/Backend/TBD_MissionLoader.c` | 120–124 | Same field (lockstep). Pre-existing emdash-vs-hyphen comments untouched. |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Zones/TBD_ZoneRegistry.c` | 97, 111 | `TBD_PlayAreaVehicleAxis.Clear()` on registry `Clear()` / `Build()` so statics do not leak across in-process mission restarts. |
| same | 452–454 | `Bind` + count a non-empty list as a legible zoneRules key. |
| same | 510–515 | Boundary apply: occupant off the governing zone's axis is treated as inside (not in violation). |
| same | 541 | `base_protection` apply: skip the zone when the occupant is off its axis. |
| `apps/mod/tbd-export/Scripts/Game/TBD/Zones/TBD_ZoneRegistry.c` | same code lines | Export twin. |

Semantics (schema + FNF v4):

- Absent / `Count()==0`: confine **everyone** (today).
- Non-empty list: confine exactly those classes. Omit `aircraft` → aircraft may leave; a player who then gets out is `infantry` and is confined again.
- On-foot → `infantry`. Heli/plane controllers or simulations → `aircraft`. `VehicleBuoyancyComponent` → `sea`. Any other vehicle → `ground`. Unknown vehicle → `ground`.
- `graceSeconds` / `warnEverySeconds` / `penalty` defaults unchanged.
- Flatten clones `zones[].rules` as `Value` (`flatten.rs:3168`); an authored list reaches `/compiled` without a flatten.rs edit.

`TBD_PlayAreaComponent.c` is **not** in owns. Apply is the registry query path that component already calls (`IsInsideBoundary` / `FindViolatedProtection`), with occupant reverse-looked-up from the XZ PlayArea just sampled.

## perturbation

No new `cargo test` was added (owns are Enfusion `.c` only). Vacuity proof is the unread gate, which is specified to go red the moment a real identifier appears.

**RED** (`cargo xtask schema validate`, verbatim):

```
T-706 unread 1.3 wire fields (each must stay reader-free until its ticket lands):
  FAIL  a 1.3 wire field gained a reader — update mission.schema.json
        'vehicleClasses' now has 6 mod identifier(s) (baseline 0) — if T-689 landed the reader, DROP the field's "no reader on any shipped build" wording in mission.schema.json and remove/repin its UNREAD_WIRE_FIELDS row; if this is an unrelated change to the pre-existing 'clean' identifier, re-pin the baseline here on purpose
```

`1 validation failure(s).`

**restored_green:** not restored. Brief: do **not** edit `xtask/src/schema_gates.rs`; command center retires the row after merge. The fire-once scratch test `unread_gate_fires_when_a_reader_appears` was left alone.

`cargo xtask mod compile` after the reader: **green** (see mod_compile_verdict).

## gate_verdict_tail

Last 15 lines of `cargo xtask platform wave gate --slice T-689`:

```
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

  gate verdict FAIL @ bda12038dd89 recorded: .ai/artifacts/verdicts/T-689.json
SLICE GATE: FAIL
```

Everything except `schema` / `validate` passed. Schema FAIL is **only** the expected `vehicleClasses` unread row (height-labels SKIP is the worktree LFS pointer, environmental).

## mod_compile_verdict

```
OK: compiled clean
    Module: Game; loaded 5753x files; 11419x classes
    Compiling Game scripts took: 849.317000 ms
    0 warning(s) in TBD sources
```

## files_outside_owns

[]

(Report path is the required deliverable, not an owns edit.)

## found_not_fixed

| path:line | repro |
|---|---|
| `xtask/src/schema_gates.rs` `UNREAD_WIRE_FIELDS` `vehicleClasses` expected 0; `packages/tbd-schema/schema/mission.schema.json` `$defs/zoneRules.vehicleClasses` still says no reader on any shipped build | `cargo xtask schema validate` → FAIL 6 identifiers / T-689. **Expected. Do not fix here.** |
| `JsonLoadContext` + `ref array<string> vehicleClasses`: absent key and authored `[]` both `Count()==0` | Schema defines `[]` as "confine nobody". Typed reader cannot tell them apart; both bind as today's apply-all so defaults stay exact. Hand-staged `[]` will **not** invert the axis. |
| `TBD_PlayAreaComponent.c` EvaluatePlayer/FindViolation (not in owns) | Apply is ZoneRegistry reverse-lookup of the body at that XZ. Two living players on the exact same XZ could classify the wrong body; a lookup miss fails toward confine-all (today). |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Zones/TBD_PlayAreaVehicleAxis.c` `IsAircraft` | Prefab-only / modded air with none of `HelicopterControllerComponent`, `VehicleHelicopterSimulation`, `AirplaneControllerComponent`, `VehicleFixedWingSimulation` classifies as `ground` (plan: unknown vehicle → ground). |

## deviations

- Slice gate ends `SLICE GATE: FAIL`, not `PASS`. Sole cause is the mandated unread fail. Did not touch `schema_gates.rs` / `packages/tbd-schema/**`.
- Did not edit `TBD_PlayAreaComponent.c` (outside owns). Apply lives on the existing registry query path plus XZ occupant lookup.

## commits

- `bda12038dd89c44e7a13b89f7e2b5095fcaf1154` `T-689: bind and apply play-area vehicleClasses axis.`
- `075ac9f33e6f1371da60de7148c1ada694289dfd` `T-689: record slice report (unread fail expected, mod compile clean).`

## manual_checklist

- Author a `boundary` with `rules.vehicleClasses: ["infantry","ground"]` (no `aircraft`). Fly a helicopter out of the AO: no grace/warn/kill while seated.
- Get out of that helicopter outside the AO: infantry now; grace/warn/kill run as authored.
- Same mission with `vehicleClasses` omitted: helicopter is confined like today.
- `base_protection` with air omitted: aircraft inside the protected base do not start the protection penalty.
- Boat with `VehicleBuoyancyComponent` and `aircraft`/`sea` omitted: sea occupant not confined; wheeled vehicle still is.
- Confirm server log `PlayAreaAxis vehicleClasses id=… aircraft=0` on load for the exempt AO.

## twins_confirmed

| path | exists |
|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/Zones/TBD_PlayAreaVehicleAxis.c` | yes (7449 B, ASCII, identical to export) |
| `apps/mod/tbd-export/Scripts/Game/TBD/Zones/TBD_PlayAreaVehicleAxis.c` | yes |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionLoader.c` | yes |
| `apps/mod/tbd-export/Scripts/Game/TBD/Backend/TBD_MissionLoader.c` | yes |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Zones/TBD_ZoneRegistry.c` | yes |
| `apps/mod/tbd-export/Scripts/Game/TBD/Zones/TBD_ZoneRegistry.c` | yes |
