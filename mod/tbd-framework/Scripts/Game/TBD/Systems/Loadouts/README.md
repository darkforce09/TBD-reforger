# Slot loadout equipping and kit preview

Dresses characters in the loadout a [slot](/documentation/glossary/n_to_z.md#slot) authors: on the
server, the equip pass that puts a slot body's gear, weapons and cargo on it and verifies they
arrived; on the client, the lobby's kit preview doll wearing the same kit and loadout; and a
development harness that equips an [arsenal](/documentation/glossary/a_to_f.md#arsenal) export file.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/Systems/Loadouts/
├── Application/                 the server equip pass: TBD_LoadoutApplication and its phases
├── Preview/                     the lobby kit-preview doll: dresser and weapon mounting
├── TBD_LoadoutEquipComponent.c  dev harness equipping $profile:TBD_LoadoutTest.json
├── TBD_LoadoutExportStruct.c    the loadout-export document structs the harness reads
└── TBD_LoadoutInventoryUtil.c   prefab names, gear counts, landing areas, parent chains, weapon storage
```

## How it works

### Server equip pass

`TBD_SlotBodyMaterializer` builds one `TBD_LoadoutApplication` per slot body, from the body and
the slot's `TBD_SlotLoadoutStruct`, and `TBD_SlotLoadoutSettle` keeps it until it settles. A slot
that authors a loadout gets `Run()`; a kit-only slot gets `RunKitWornAudit(kit)`, the nakedness
check alone. The phases, the verify polls and the verdict are described in
[Application](Application/README.md). Every failure names its slot and item on one `[TBD]` line;
a blocking failure (the slot is unplayable, such as a prefab that does not exist) logs an ERROR
and is what `HasBlockingFailure()` reports to the spawn boundary; a shortfall logs a WARNING.

### Kit preview

`TBD_LoadoutPreviewDresser.Dress` applies the same composition rule to the lobby's preview doll
with local entities; see [Preview](Preview/README.md).

### Development harness

`TBD_LoadoutEquipComponent` is a `SCR_BaseGameModeComponent` on
`mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`. With its `m_bRunLoadoutTest` attribute
on (off by default), it reads `$profile:TBD_LoadoutTest.json`, the web arsenal's loadout export,
into `TBD_LoadoutExportStruct` (`loadoutVersion` `1` or `2`; another `modpackId` than the expected
one is a WARNING), spawns `m_sTestCharacter` at `m_vSpawnOrigin` three seconds after start, and
dresses it with a `TBD_LoadoutApplication` whose lines carry the `[TBD][Loadout][TestNPC]` tag.
A v2 document is read from its own `wear`, `weapons` and `cargo` fields, not its derived `gear`
block.

| Attribute | Default | Meaning |
|---|---|---|
| `m_bRunLoadoutTest` | `0` | run the equip test on play; development only |
| `m_sTestCharacter` | `Character_US_Base.et` | the minimal body to equip |
| `m_vSpawnOrigin` | `6400 0 6400` | where the test body spawns |

`TBD_LoadoutInventoryUtil` holds the stateless inventory queries the equip pass, the preview, the
gadget flags and the mission slot checks share: `PrefabOf`, `CountGear` (the equip verdict's
denominator), `AreasForLabel` (a vest may land in the armored vest area), `IsRootedOn`,
`WeaponStorageOf` and `WeaponStorageHas`.

## Authority

- Server: the equip pass and the harness. The `Application/` methods that spawn or move entities
  carry `@authority server`; the harness's `OnPostInit` carries it and returns on
  `TBD_Authority.IsClient()`. The engine replicates the entities the pass spawns and moves.
- Client: the `Preview/` scripts, menu-time code with local entities only.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_SlotLoadoutStruct` and its gear and cargo structs in
  `mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Data/`; `TBD_Registry` (kit aliases to
  prefabs); `TBD_Log` and `TBD_WarnOnce`; the engine's `SCR_InventoryStorageManagerComponent`,
  `EquipedLoadoutStorageComponent`, `EquipedWeaponStorageComponent`, `AttachmentSlotComponent` and
  `ItemPreviewManagerEntity`; the harness's file shape in
  `contracts/definitions/loadout-export.schema.json`.
- Used by: `TBD_SlotBodyMaterializer`, `TBD_SlotLoadoutSettle` and `TBD_DeployExecutor` in
  `mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/` (`TBD_LoadoutApplication`,
  `HasBlockingFailure`, `ShortfallBrief`); `TBD_KitPreviewComponent` in
  `mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/UI/` (`GetOrSpawnManager`, `Dress`);
  `TBD_GadgetFlags` and `TBD_MissionSlotChecks` (`TBD_LoadoutInventoryUtil`);
  `mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, which attaches
  `TBD_LoadoutEquipComponent`.
- Rules: the preview and the server pass apply the same composition rule, so the doll wears what
  the slot spawns; only a blocking failure stops a session, and a shortfall stays a WARNING; the
  harness stays off on the shipped game mode; lines added stay ASCII, and `cargo xtask mod compile`
  checks that the scripts compile, while what a body wears is checked in a round.
