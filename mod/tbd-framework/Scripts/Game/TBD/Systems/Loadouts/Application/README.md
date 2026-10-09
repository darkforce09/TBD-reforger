# Server loadout equip pass

Dresses one slot body in the loadout its slot authors and reports whether everything arrived:
worn gear and weapons first, then the optic, magazine and attachments on the primary, then cargo,
then a check that the body is not naked. It runs on the server each time a slot body spawns.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/Systems/Loadouts/Application/
├── TBD_LoadoutApplication.c  TBD_LoadoutApplication: runs the phases, keeps the delivery ledger, writes the verdict
├── TBD_LoadoutCargoPhase.c   TBD_LoadoutCargoPhase: inserts each cargo row into its worn container
├── TBD_LoadoutGearPhase.c    TBD_LoadoutGearPhase: equips garments and weapon slots, polls until worn, swaps
├── TBD_LoadoutWeaponPhase.c  TBD_LoadoutWeaponPhase: mounts optic, magazine and attachments on the primary
└── TBD_LoadoutWornAudit.c    TBD_LoadoutWornAudit: the nakedness guard, after a pass or after a kit-only spawn
```

## How it works

`TBD_LoadoutApplication` owns one of each phase by strong reference; each phase keeps a weak
back-reference and reports through the application's `Fail` and `Degrade`. `Run()` counts what
the loadout authors and starts the phases in order:

1. gear (`TBD_LoadoutGearPhase`): each authored garment replaces the kit's (an absent one keeps
   it) through `EquipCloth`, after the incumbent is captured; a same-prefab item is skipped. The
   primary, launcher, sidearm and throwable go into engine weapon slots 0 to 3 by
   `CanInsertItemInStorage`, else `CanReplaceItem`, never `EquipWeapon`. A 500 ms poll, up to six
   ticks, verifies each item is worn; a verified item deletes the incumbent it displaced;
2. weapon (`TBD_LoadoutWeaponPhase`): `optic`, `magazine` and each `attachments` entry spawn into
   the primary's storage and are verified by a 250 ms scan; when the weapon's own magazine blocks
   the well it is swapped out once; an item that still will not mount is stowed loose (degraded);
3. cargo (`TBD_LoadoutCargoPhase`): each `cargo[]` row goes into its worn container after
   `CanInsert*` checks, falling back to any storage (degraded);
4. audit (`TBD_LoadoutWornAudit`): a body with no jacket or no pants is an ERROR naming the slot;
5. verdict: `loadout pass complete`, or `REFUSED` (ERROR, blocking) or `SHORTFALL` (WARNING),
   followed by the itemised `loadout INCOMPLETE` and `loadout DEGRADED` lines.

`RunKitWornAudit(kit)` runs step 4 alone for a slot with no loadout, polling every 250 ms for up to
one second so it never reports a body that is still dressing. `CallLater` keeps no reference, so the
owner (`TBD_SlotLoadoutSettle`, or the harness) holds the application until `IsDone()`, and
`Cancel` stands it down when the body is deleted or superseded.

## Authority

- Server: everything; the methods that spawn, move or delete entities carry `@authority server`,
  and their callers run only on the server.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_SlotLoadoutStruct`, `TBD_SlotGearStruct` and `TBD_SlotCargoStruct`;
  `TBD_LoadoutInventoryUtil`; the engine's `SCR_InventoryStorageManagerComponent`,
  `SCR_CharacterInventoryStorageComponent`, `BaseWeaponManagerComponent` and the `Loadout*Area`
  types.
- Used by: `TBD_SlotBodyMaterializer`, `TBD_SlotLoadoutSettle` and `TBD_DeployExecutor` in
  `mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/`; `TBD_LoadoutEquipComponent` in the
  parent folder.
- Rules: the `[TBD][Equip] slot=... weapon=... result=...` and `[TBD][Equip] attach=...` lines keep
  their format, because the gates grep them; only a blocking failure logs an ERROR; weapon slot ids
  match the Mission Creator's Arsenal and `loadout-export.schema.json`; `cargo xtask mod compile`
  checks that the scripts compile.
