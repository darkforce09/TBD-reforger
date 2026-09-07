# REPORT T-304 — RegistryScan weapon weight

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-304
slice/T-304
```

First action of the run, before any edit. HEAD at start: `b59b99116` (T-942 wave 252 briefs).

## defect_verified_on_main

Proved on this worktree (branched from main at the wave-252 brief commit) before writing. Registry JSON was **read only** — not edited.

| claim | evidence |
|---|---|
| `ReadPhysAttrsPass` only reads `ItemPhysAttributes` when `cls.EndsWith("InventoryItemComponent")` | `TBD_RegistryScan.c` (pre-edit): `isStorage` reads `m_fMaxWeight` / `MaxCumulativeVolume` only. `SCR_WeaponAttachmentsStorageComponent` matches `*StorageComponent`, so weapon weight is skipped. |
| 0/107 weapons have `weight_kg` | `registry-items.workbench.json`: kinds `gear_primary` (84) + `gear_launcher` (18) + `gear_handgun` (5) = **107**; `weight_kg` present on **0**. |
| AK74 has no weight (T-206 pin) | `{43497A18DD888667}Prefabs/Weapons/Rifles/AK74/Rifle_AK74_base.et` — `kind=gear_primary`, no `weight_kg`, no `volume_cm3` (only `max_volume_cm3` from storage). |
| Hash-order class foreach lets `Item_Base` 0.01 kg win | `foreach (string cls, array<BaseContainer> bucket : comps)` over a hash map; first write of `weightKg` sticks. Ticket named 32 rows; **measured 53** rows with `weight_kg == 0.01` (34 non-`other` + 19 `other` including `Item_Base.et`). |

Ticket examples still 0.01 kg (real weights from T-206 / ticket: 14.2 / 18.5 / 19.96 — not in the JSON, scanner never wrote them):

- `Part_M252_Barrel.et` — `gear_backpack` `weight_kg=0.01` `volume_cm3=100`
- `Radio_R107M.et` / `Radio_R107M_FIA.et` — `gear_backpack` `weight_kg=0.01`
- `Part_M3_Tripod.et` — `gear_backpack` `weight_kg=0.01`

Non-`other` 0.01 kg rows (34): `Part_M3_Tripod`, `Part_M252_Barrel`, `DeployablePart_Main_Base`, `Radio_R107M`, `Radio_ANPRC77`, `Part_Tripod_M122`, `Part_2B14_BasePlate`, `Radio_ANPRC77_COK`, `Part_NSV_Tripod`, `Part_M252_Bipod`, `Binoculars_base`, `Part_Sandbag_Base`, `Part_2B14_Barrel`, `BlastingMachine_base`, `Flashlight_base`, `Part_Tripod_6T5`, `Watch_Base`, `Part_Sandbag_Burlap`, `Part_2B14_Bipod`, `Radio_R107M_FIA`, `Part_NSV_Gun_Optic`, `Radio_RF10`, `BlastingMachine_M34_base`, `BlastingMachine_KPM_3U1_base`, `Part_M2_Gun`, `Radio_Deployable_Base`, `DeployablePart_Base`, `Radio_base`, `Part_M252_BasePlate`, `Part_Sandbag_Plastic`, `Radio_RF10_base`, `BlastingMachine_M34`, `Part_NSV_Gun`, `BlastingMachine_KPM_3U1`.

Not already fixed. Implemented. Registry rows stay wrong until the operator re-scans (LOCKED).

## changes

| path | line | why |
|---|---|---|
| `apps/mod/tbd-framework/Scripts/WorkbenchGame/TBD_RegistryScan.c` | 314–372 | `ClassKeysMostDerivedFirst` / `SortClassNamesMostDerivedFirst` / `ClassIsMoreDerived` / `AssertClassNamesMostDerivedFirst`. typename has no parent-walk API — pair `IsInherited`, stable insertion sort, most-derived first. Assertion prints `T-304 ASSERT` + `LogLevel.ERROR` if a strict subtype is sorted after its base (hash-order failure mode). |
| same | 538–591 | `ReadPhysAttrsPass`: iterate ordered keys, never `foreach (comps)`. Read `Attributes.ItemPhysAttributes` when `isInvItem \|\| isStorage` so `SCR_WeaponAttachmentsStorageComponent` yields weapon `Weight` / `ItemVolume`. Capacity pass unchanged (universal first, then fallback). |
| same | HasCompSuffix, CollectObjectVarClasses, CollectResourceVarValues, HasCompInheritedFrom, ItemAttachmentType, CollectVehicleWeapons, CollectLoadoutPrefabs, CollectInitialCargo, FirstUiName | Every class-keyed `comps` foreach now goes through `ClassKeysMostDerivedFirst`. Zero remaining `foreach (string cls, array<BaseContainer> bucket : comps)`. |
| `apps/mod/tbd-export/Scripts/WorkbenchGame/TBD_RegistryScan.c` | (byte-identical) | ASCII twin of the framework copy. `cmp` empty. |

No registry JSON edits. No `packages/tbd-schema`. No `flatten.rs`. No `schema_gates.rs`. No sibling wave files.

Framework copy was transliterated to ASCII (`—`→`-`, `→`→`->`, `…`→`...`, `§`→`S`, `³`→`^3`, `×`→`x`) so both twins can be identical under the export non-ASCII lock. Pre-existing twin drift was only that punctuation.

## perturbation

`cargo xtask mod compile` does **not** typecheck `Scripts/WorkbenchGame` (headless server compiles Game only). Skipping sort on **both** twins therefore stays compile-green. Observable RED for this owns set is lockstep.

Reverted framework `ClassKeysMostDerivedFirst` to hash-order `foreach (comps)` and skipped `SortClassNamesMostDerivedFirst`; left `AssertClassNamesMostDerivedFirst(outKeys)` on that hash list; export twin still sorted. `cargo xtask mod compile` RED, verbatim:

```
FAIL: tbd-framework and tbd-export are not in lockstep
      (scripts compared as code + string literals, with the ASCII rule's punctuation
       folded away; every other shared path compared byte-for-byte)
  Scripts/WorkbenchGame/TBD_RegistryScan.c: the two copies differ in code or in a string literal
      The engine compiles the tbd-framework copy (T-946.23) and the mirror ships too,
      so a divergence here is code that no gate reads. Make the two copies agree.
red_exit=1
```

Restored by copying the unperturbed export twin over framework. `touch`ed both `TBD_RegistryScan.c`. Twins identical again. Assertion + sort path restored.

**restored_green:**

```
OK: compiled clean
    Module: Game; loaded 5755x files; 11434x classes
    Compiling Game scripts took: 1085.706000 ms
    0 warning(s) in TBD sources
green_exit=0
```

(First green, before perturbation, was the same verdict at 877.851000 ms, 5755 files / 11434 classes.)

Runtime: if sort is skipped inside Workbench, `AssertClassNamesMostDerivedFirst` prints `T-304 ASSERT: most-derived bucket must win, never hash order` on any prefab that carries both a base and a derived component class. No Enfusion unit harness exists here.

## gate_verdict_tail

`cargo xtask platform wave gate --slice T-304` — last 15 lines (lock wait ~600s, then):

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

  gate verdict PASS @ b59b99116a09 recorded: .ai/artifacts/verdicts/T-304.json
SLICE GATE: PASS
gate_exit=0
```

`cargo xtask schema validate` — All contracts valid (registry-items.workbench.json PASS; JSON not modified).

## mod_compile_verdict

```
OK: compiled clean
    Module: Game; loaded 5755x files; 11434x classes
    Compiling Game scripts took: 1085.706000 ms
    0 warning(s) in TBD sources
green_exit=0
```

## files_outside_owns

[]

Only:

- `apps/mod/tbd-framework/Scripts/WorkbenchGame/TBD_RegistryScan.c`
- `apps/mod/tbd-export/Scripts/WorkbenchGame/TBD_RegistryScan.c`

plus this report. EnfusionMCP copy is gitignored (19 files, not committed).

## found_not_fixed

- **Registry JSON still has 0/107 weapon weights and the 0.01 kg rows.** Locked: do not hand-patch; the next Workbench scan is the operator's step.
- Ticket said **32** poisoned rows; this checkout measures **53** at `0.01 kg` (34 non-`other`). The named examples match. The extra rows are the same `Item_Base` 0.01 kg class, not a second bug.
- `AssertClassNamesMostDerivedFirst` cannot go RED under `mod compile` because WorkbenchGame is never compiled headless. Workbench scan is the runtime for that Print.

## deviations

- Framework comments were ASCII-folded to match the export non-ASCII gate so the twins can be byte-identical (T-304 acceptance). Mapping matches the pre-existing export twin.
- Every `comps` class foreach uses ordered keys, not only the four loops at old 324–432 + `ReadPhysAttrsPass`.
- Perturbation RED is lockstep (one twin hash-order), not a Game-module undefined-function compile error. WorkbenchGame has no headless typecheck.

## commits

Code + this report, one commit on `slice/T-304` (see git log). No push.

## manual_checklist

Operator, after merge, in Workbench:

1. Plugins → TBD → Export TBD Registry Items (loaded addons including vanilla).
2. Copy `$profile:TBD_RegistryItems.json` over `packages/tbd-schema/registry/registry-items.workbench.json` (operator owns that file; this slice did not touch it).
3. Confirm **107/107** `gear_primary`+`gear_launcher`+`gear_handgun` rows have `weight_kg`.
4. Confirm the 0.01 kg set is gone for the named rows: `Part_M252_Barrel` (~14.2), `Radio_R107M` (~18.5), `Part_M3_Tripod` (~19.96), and the rest of the 34 non-`other` list above.
5. Confirm `Rifle_AK74_base` has Weight **3.07** / ItemVolume **1500** (T-206 pak evidence).

## twins_confirmed

`cmp` of the two `TBD_RegistryScan.c` copies: **identical**. Both ASCII (`non_ascii=0`). CRLF preserved. `mirror_lockstep` green on restored compile and on the slice gate.
