# REPORT-T-310

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-310
slice/T-310
```

## defect_verified_on_main

Ticket `:1497` is stale. Live `mod_slot_loadout` is `flatten.rs:2012`. Before this slice it set `gear.optic` / `gear.magazine` only and never read `weapons[].attachments`.

| claim | path:line | command |
|---|---|---|
| Flatten mapper ignores SlotLoadoutV2 `attachments[]` edges | `crates/map-engine-core/src/mission/flatten.rs:2012` (pre-slice: optic/magazine assign only; no `gear.attachments`) | `rg -n "fn mod_slot_loadout|gear.optic|gear.magazine|attachments" crates/map-engine-core/src/mission/flatten.rs` |
| Compiled golden gear has no `attachments` key (empty edges omitted, non-empty never emitted) | `packages/tbd-schema/golden-missions/compiler-shaped-two-faction.json` slots `blufor:Ranger:SL:0` etc. | `python3` load golden; print `sorted(slot["loadout"]["gear"].keys())` — keys were backpack/boots/handgun/handwear/helmet/magazine/optic/pants/primary/throwable/uniform/vest; `"attachments" in g` was `False` |
| Schema `gear` is `additionalProperties: false` over string fields only | `packages/tbd-schema/schema/mission.schema.json:380-399` (pre-slice: no `attachments` property) | `sed -n '380,399p' packages/tbd-schema/schema/mission.schema.json` |
| Equip helper mounts optic/magazine only | `TBD_LoadoutEquipHelper.c:996-997` (pre-slice `IssueWeaponItem("optic"/"magazine")` only) | `rg -n "IssueWeaponItem" apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_LoadoutEquipHelper.c` |

A suppressor in Arsenal `weapons[0].attachments` therefore never reached `/compiled` and could not be mounted.

## changes

| path | line | why |
|---|---|---|
| `packages/tbd-schema/schema/mission.schema.json` | 382 (description clause), 388-393 (`attachments` property) | Surgical insert: optional `gear.attachments[]` of `wireSafeString` `minLength: 1`. No `json.dumps`. |
| `crates/map-engine-core/src/mission/flatten.rs` | 226-230 (`ModSlotGear.attachments`), 262 (`is_empty`), 2012 (`mod_slot_loadout`), 2056-2064 (emit), 5661 (`arsenal_suppressor_edge_reaches_compiled_gear`) | Emit primary (0,primary) SlotLoadoutV2 edges; skip empty strings; omit empty list (byte-identical). Golden pin uses live suppressor `{E52C9791E1554A5F}Prefabs/Weapons/Attachments/Muzzle/Suppressor_M16/Suppressor_M16.et`. |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_LoadoutEquipHelper.c` | 384, 415, 447, 985-1015, 1029-1034, 1068, 1178 | Count attachments in verdict denominator; after optic/magazine `IssueWeaponItem("attach", …)`; `[TBD][Equip] attach=<res> result=<ok\|failed>` via `LogAttachResult`. |
| `apps/mod/tbd-export/Scripts/Game/TBD/Gamemode/TBD_LoadoutEquipHelper.c` | same CODE (ASCII twin) | Lockstep. |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionSlotStruct.c` | 10 | **Required bind:** `ref array<string> attachments` on `TBD_SlotGearStruct`. Without this, `gear.attachments` is invisible to JsonLoadContext and the helper would not compile. |
| `apps/mod/tbd-export/Scripts/Game/TBD/Backend/TBD_MissionSlotStruct.c` | 10 | Export twin. |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_LoadoutEquipComponent.c` | 267 (copy), 284 (warn) | TestNPC v2 path copies primary edges onto `gear.attachments`; warns only for non-primary weapons. |
| `apps/mod/tbd-export/Scripts/Game/TBD/Gamemode/TBD_LoadoutEquipComponent.c` | same CODE | Export twin. |

Did **not** edit `xtask/src/schema_gates.rs`. Did **not** run `schema-codegen` (`mission.schema.json` is not a typify target). Did **not** emit T-216 slot deltas or other flatten holes.

## perturbation

Skip the emit at `flatten.rs:2056` (assign dropped). Restore + `touch crates/map-engine-core/src/mission/flatten.rs`.

**red_output VERBATIM:**

```
thread 'mission::flatten::tests::arsenal_suppressor_edge_reaches_compiled_gear' (2144360) panicked at crates/map-engine-core/src/mission/flatten.rs:5669:9:
assertion `left == right` failed
  left: []
 right: ["{E52C9791E1554A5F}Prefabs/Weapons/Attachments/Muzzle/Suppressor_M16/Suppressor_M16.et"]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test mission::flatten::tests::arsenal_suppressor_edge_reaches_compiled_gear ... FAILED

failures:

failures:
    mission::flatten::tests::arsenal_suppressor_edge_reaches_compiled_gear

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 997 filtered out; finished in 0.00s
```

**restored_green:** after restore + `touch`, same command:

```
test mission::flatten::tests::arsenal_suppressor_edge_reaches_compiled_gear ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 997 filtered out; finished in 0.00s
```

Also green: `compiler_shaped_golden_is_a_fresh_emitter_output` (empty `attachments: []` still omits the compiled key). `cargo test -p map-engine-core --all-features --lib mission::flatten`: 92 passed, 2 ignored.

## gate_verdict_tail

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

  gate verdict PASS @ d6197b0bcb1e recorded: .ai/artifacts/verdicts/T-310.json
SLICE GATE: PASS
```

`cargo xtask schema validate`: All contracts valid.

## mod_compile_verdict

```
OK: compiled clean
    Module: Game; loaded 5752x files; 11415x classes
    Compiling Game scripts took: 850.586000 ms
    0 warning(s) in TBD sources
```

EnfusionMCP copy already present (19 `.c`) — not committed.

## files_outside_owns

Brief owns list omitted the JsonLoadContext struct and the TestNPC copy path. Helper `gear.attachments` does not compile without the struct field. Reported, not silent.

- `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionSlotStruct.c` — bind `TBD_SlotGearStruct.attachments`
- `apps/mod/tbd-export/Scripts/Game/TBD/Backend/TBD_MissionSlotStruct.c` — twin
- `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_LoadoutEquipComponent.c` — copy primary edges for `$profile` TestNPC; stop lying T-197 "NOT mounted" warning on primary
- `apps/mod/tbd-export/Scripts/Game/TBD/Gamemode/TBD_LoadoutEquipComponent.c` — twin

## found_not_fixed

| path:line | repro |
|---|---|
| `flatten.rs:2056` reads `(0,primary)` only | A suppressor on `weapons[]` with `(slotIndex 2, slotType secondary)` (or launcher/throwable) is still dropped from compiled `gear.attachments`. Same class as optic/magazine riding the primary alone. Schema has one list; helper mounts onto the primary storage. |
| In-game mount | Mechanical proof is compile + flatten golden + `mod compile`. Live spawn inspect is human. |

## deviations

- Extra files above (`files_outside_owns`). Ticket cannot mount without the struct field.
- Ticket `verify` names `cargo xtask ci schema-validate` / `schema-codegen`. Ran `cargo xtask schema validate` per brief. Skipped codegen: `mission.schema.json` is not in `xtask/src/codegen_schema.rs` TARGETS.
- Did not add `attachments` to `UNREAD_WIRE_FIELDS` (forbidden `schema_gates.rs` edit; field now has a reader).

## commits

- `d6197b0bcb1e22ef816f960f8d79e51a82a42dfc` T-310: emit and mount Arsenal gear.attachments[].
- (this report commit follows)

## manual_checklist

1. In the Arsenal, pick suppressor `{E52C9791E1554A5F}Prefabs/Weapons/Attachments/Muzzle/Suppressor_M16/Suppressor_M16.et` on the primary rifle, compile, spawn that slot; the suppressor is on the rifle (not irons only).
2. Server log contains `[TBD][Equip] attach={E52C9791E1554A5F}Prefabs/Weapons/Attachments/Muzzle/Suppressor_M16/Suppressor_M16.et result=ok` for that spawn.
3. A loadout with no attachments still equips as before (no new `INCOMPLETE` from a phantom attachments count).

## twins_confirmed

On disk in this worktree:

- `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_LoadoutEquipHelper.c`
- `apps/mod/tbd-export/Scripts/Game/TBD/Gamemode/TBD_LoadoutEquipHelper.c`
- `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionSlotStruct.c`
- `apps/mod/tbd-export/Scripts/Game/TBD/Backend/TBD_MissionSlotStruct.c`
- `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_LoadoutEquipComponent.c`
- `apps/mod/tbd-export/Scripts/Game/TBD/Gamemode/TBD_LoadoutEquipComponent.c`

Export copies are pure ASCII. `cargo xtask mod compile` lockstep clean.
