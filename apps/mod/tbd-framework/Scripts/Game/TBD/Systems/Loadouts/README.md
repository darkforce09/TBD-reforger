# Slot loadout equipping and kit preview

Dresses characters in the loadout a [slot](/documentation_v2/glossary.md#slot) authors: on the
server, the equip pass that puts a slot body's gear, weapons and cargo on it and verifies they
arrived; on the client, the lobby's kit preview doll wearing the same kit and loadout; and a
development harness that equips an [arsenal](/documentation_v2/glossary.md#arsenal) export file.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Loadouts/
├── TBD_LoadoutEquipComponent.c  dev harness equipping $profile:TBD_LoadoutTest.json, and the loadout-export structs
├── TBD_LoadoutEquipHelper.c     TBD_LoadoutApplication: the server equip pass, its verify and its verdict
└── TBD_LoadoutPreviewDresser.c  dresses the lobby's local preview character with a slot's kit and loadout
```

## How it works

### Server equip pass

`TBD_SpawnManager.SpawnSlotBody` builds one `TBD_LoadoutApplication` per slot body, from the body
and the slot's `TBD_SlotLoadoutStruct`, and keeps it until it settles. A slot that authors a
loadout gets `Run()`; a kit-only slot gets `RunKitWornAudit(kit)`, the nakedness check alone.
`Run()` works in phases:

1. wear: each authored garment replaces the kit's (an absent one keeps it); incumbents are
   captured first, and a same-prefab item is skipped;
2. verify: a 500 ms poll, up to six ticks, until each equipped item is worn; a verified swap
   deletes the displaced incumbent with its contents;
3. weapons: the primary, launcher, sidearm and throwable go to their weapon slots, and `optic`,
   `attachments` and `magazine` mount into the primary's own storage, re-verified by a scan;
4. cargo: each `cargo[]` row is inserted into its container after `CanInsert*` checks, falling
   back to any storage;
5. verdict: every failure names its slot and item on one `[TBD]` line. A blocking failure (the
   slot is unplayable, such as a prefab that does not exist) logs an ERROR and is what
   `HasBlockingFailure()` reports to the spawn boundary; a shortfall (the item reached the body in
   the wrong place, or did not fit) logs a WARNING. A pass that ends with no jacket or no pants on
   the body is an ERROR naming the slot.

### Kit preview

`TBD_LoadoutPreviewDresser.Dress` follows the same composition rule on the preview entity of an
`ItemPreviewManagerEntity` (it spawns vanilla's `ItemPreviewManager.et` locally when the world has
none, `GetOrSpawnManager`). It is a separate class, not a subclass, because it spawns local
entities (`SpawnEntityPrefabLocal`) and attaches synchronously, and a miss is one WARNING per
reason rather than an ERROR. The manager returns one preview entity per prefab, and baked weapon
slots have no template, so the first time a prefab resolves its baseline (what every garment and
weapon slot holds) is recorded; each pass then sets every slot to the authored prefab, else the
baseline, and clears a slot whose baseline is empty. Attachments mount through
`AttachmentSlotComponent.CanSetAttachment`, the magazine falls back to the inventory manager, and
the primary goes in the doll's hands.

### Development harness

`TBD_LoadoutEquipComponent` is a `SCR_BaseGameModeComponent` on
`apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`. With its `m_bRunLoadoutTest` attribute
on (off by default), it reads `$profile:TBD_LoadoutTest.json`, the web arsenal's loadout export
(`loadoutVersion` `1` or `2`; another `modpackId` than the expected one is a WARNING), spawns
`m_sTestCharacter` at `m_vSpawnOrigin` three seconds after start, and dresses it with a
`TBD_LoadoutApplication` whose lines carry the `[TBD][Loadout][TestNPC]` tag.

| Attribute | Default | Meaning |
|---|---|---|
| `m_bRunLoadoutTest` | `0` | run the equip test on play; development only |
| `m_sTestCharacter` | `Character_US_Base.et` | the minimal body to equip |
| `m_vSpawnOrigin` | `6400 0 6400` | where the test body spawns |

## Authority

- Server: the equip pass and the harness. `TBD_LoadoutEquipHelper.c` carries `@authority server`
  on its header and on `RunKitWornAudit`; the harness's `OnPostInit` carries it and returns on
  `RplMode.Client`. The engine replicates the entities the pass spawns and moves.
- Client: `TBD_LoadoutPreviewDresser`, menu-time code with local entities only.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_SlotLoadoutStruct` and its gear and cargo structs in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Data/`; `TBD_Registry` (kit aliases to
  prefabs); `TBD_Log`; the engine's `SCR_InventoryStorageManagerComponent`,
  `EquipedLoadoutStorageComponent`, `EquipedWeaponStorageComponent`, `AttachmentSlotComponent` and
  `ItemPreviewManagerEntity`; the harness's file shape in
  `contracts_v2/definitions/loadout-export.schema.json`.
- Used by: `TBD_SpawnManager` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/`
  (`TBD_LoadoutApplication`, `HasBlockingFailure`); `TBD_KitPreviewComponent` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/UI/` (`GetOrSpawnManager`, `Dress`);
  `TBD_FrameworkManager`'s component roll call; `apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`,
  which attaches `TBD_LoadoutEquipComponent`.
- Rules: the preview and the server pass apply the same composition rule, so the doll wears what
  the slot spawns; only a blocking failure stops a session, and a shortfall stays a WARNING; the
  harness stays off on the shipped game mode; lines added stay ASCII, and `cargo xtask mod compile`
  checks that the scripts compile, while what a body wears is checked in a round.
