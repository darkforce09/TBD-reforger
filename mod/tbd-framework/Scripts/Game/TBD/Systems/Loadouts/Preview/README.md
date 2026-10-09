# Lobby kit preview doll

Dresses the lobby's 3D preview character in a slot's kit and loadout, so the player sees what the
slot spawns with. It runs on the client whenever the kit preview shows a slot.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/Systems/Loadouts/Preview/
├── TBD_LoadoutPreviewDresser.c  TBD_LoadoutPreviewDresser: resolves the doll, dresses garments and weapon slots
└── TBD_LoadoutPreviewMount.c    TBD_LoadoutPreviewMount: mounts optic and magazine, puts the primary in hand
```

## How it works

`TBD_LoadoutPreviewDresser.Dress` resolves the preview entity of the kit prefab from the world's
`ItemPreviewManagerEntity` (`GetOrSpawnManager` spawns vanilla's `ItemPreviewManager.et` locally
when the world has none) and applies the server pass's composition rule: an authored garment or
weapon replaces the kit's, an absent one keeps it. It spawns local entities
(`SpawnEntityPrefabLocal`) and attaches them synchronously to the `EquipedLoadoutStorageComponent`
and `EquipedWeaponStorageComponent` slots.

The manager returns one shared preview entity per prefab, and baked weapon slots have no template,
so the first time a prefab resolves its baseline (what every garment and weapon slot holds) is
recorded in a `TBD_PreviewBaseline`; each pass then sets every slot to the authored prefab, else
the baseline, and clears a slot whose baseline is empty. `TBD_LoadoutPreviewMount.Mount` puts the
optic and magazine into the attachment slot whose `AttachmentSlotComponent.CanSetAttachment`
accepts them, the magazine falling back to the inventory manager, and `HoldWeapon` selects the
primary. A miss is one WARNING per reason on the `lobby` channel, through `TBD_WarnOnce`.

## Authority

- Server: nothing.
- Client: everything; menu-time code with local entities only, no replication.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_SlotLoadoutStruct`, `TBD_LoadoutInventoryUtil`, `TBD_WarnOnce`; the engine's
  `ItemPreviewManagerEntity`, `EquipedLoadoutStorageComponent`, `EquipedWeaponStorageComponent`,
  `WeaponAttachmentsStorageComponent` and `AttachmentSlotComponent`.
- Used by: `TBD_KitPreviewComponent` in
  `mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/UI/` (`GetOrSpawnManager`, `Dress`).
- Rules: the doll follows the same composition rule as the server pass in `../Application/`; a
  preview miss never logs an ERROR; `cargo xtask mod compile` checks that the scripts compile.
