# ACE → TBD kind → Reforger detect signals → Forge

**Status:** frozen with the T-068.10.2 census (2026-07-12) ·
**Sources:** ACE3 `arsenal/defines.hpp` (IDX_CAT_* 0–23) · `Prefabs/Characters/Core/Character_Base.et` ·
`scripts/Game/Components/Arsenal/SCR_EArsenalItemType.c` (22 flags) · `Configs/EntityCatalog/*` (48 per-faction
configs) · [`t068_10_2_census.md`](t068_10_2_census.md) (per-item evidence) · T-150 verify log.

Reforger engine facts this map is built on (evidence in census + plan):

- Weapon slots on vanilla characters: **2× untyped `"primary"` + 1× `"secondary"` + `"grenade"` +
  `"throwable"`** (`WeaponType None` — two rifles legal; launcher = a weapon in the 2nd primary slot,
  proven by `Character_USSR_LAT.et` AK74 + RPG22).
- Wear areas are separate simultaneous slots: Jacket / Pants / Boots / Vest / **ArmoredVest** / HeadCover /
  Backpack / Handwear (+ Googles, Binoculars, IdentityItem, Watch, Cover). "Uniform" does not exist as one item.
- Storage = **volume (cm³) + weight (kg)**, not grid (`GetMaxVolumeCapacity` / `GetMaxLoad`; char `m_fMaxWeight 80`).
- Flashlights are **gadgets** (`SCR_FlashlightComponent`), not weapon attachments — ACE's flashlight tab has no
  attachment analogue.
- Vanilla flares descend `Launcher_Base` (single-use hand launchers), not the grenade family.

| ACE category (IDX_CAT_*) | TBD kind (v3) | Detect signal (priority order) | Forge |
|---|---|---|---|
| ALL (0) | — | (search feature, not a row) | search box |
| PRIMARY_WEAPON (1) | `gear_primary` | catalog RIFLE/MACHINE_GUN/SNIPER_RIFLE → ancestor `Rifle_Base`/`MachineGun_Base`/`LongRangeRifle_Base` | row now |
| SECONDARY_WEAPON (2) = launchers | `gear_launcher` | catalog ROCKET_LAUNCHER → ancestor `Launcher_Base` → `*MuzzleInMagComponent` → `/Launchers/` | row now |
| HANDGUN_WEAPON (3) | `gear_handgun` | catalog PISTOL → ancestor `Handgun_Base` → `/Handguns/` | later (engine slot `"secondary"`) |
| OPTICS_ATTACHMENTS (4) | `optic` | `WeaponAttachmentAttributes.AttachmentType` = `AttachmentOptics*` / `SightsComponent` | row now |
| FLASHLIGHT_ATTACHMENTS (5) | `gear_item` | `SCR_FlashlightComponent` (gadget — NOT an attachment in Reforger) | later |
| MUZZLE_ATTACHMENTS (6) | `attachment` | AttachmentType `AttachmentMuzzle*` | later |
| BIPOD_ATTACHMENTS (7) | `attachment` | AttachmentType `AttachmentUnderBarrel*` | later |
| ITEMS_ALL (8) | `gear_item` | `SCR_GadgetComponent` family; catalog HEAL/EQUIPMENT | later |
| HEADGEAR (9) | `gear_helmet` | AreaType `LoadoutHeadCoverArea` (+`LoadoutCoverArea`) | row now |
| UNIFORM (10) | `gear_jacket` / `gear_pants` / `gear_boots` | AreaType `LoadoutJacketArea` / `LoadoutPantsArea` / `LoadoutBootsArea` | rows now (3) |
| VEST (11) | `gear_vest` / `gear_armored_vest` | AreaType `LoadoutVestArea` / `LoadoutArmoredVestSlotArea` — worn simultaneously | rows now (2) |
| BACKPACK (12) | `gear_backpack` | AreaType `LoadoutBackpackArea` (incl. deployable weapon parts — engine-true) | row now |
| GOGGLES (13) | `gear_glasses` | AreaType `LoadoutGooglesArea` (engine spelling) | later |
| NVG (14) | `gear_glasses`/`gear_item` | no vanilla NVG (1989); decide on first NVG modset | hide |
| BINO (15) | `gear_binoculars` | `SCR_BinocularsComponent`; equipment slot BINOCULARS | later |
| MAP (16) | `gear_item` | `SCR_MapGadgetComponent` | later |
| COMPASS (17) | `gear_item` | `SCR_CompassComponent` | later |
| RADIO (18) | `gear_item` | `SCR_RadioComponent` (handhelds; manpacks are backpacks / RADIO_BACKPACK) | later |
| WATCH (19) | `gear_item` | AreaType `LoadoutWatchArea`; equipment slot WATCH | later |
| COMMS (20) | — | no Reforger equivalent (ACE earplugs) | hide |
| GRENADES (21) | `gear_throwable` | `GrenadeMoveComponent` (grenades + smokes; catalog LETHAL/NON_LETHAL_THROWABLE) | row now |
| EXPLOSIVES (22) | `gear_explosive` | `SCR_ExplosiveCharge*` / `SCR_ExplosiveTriggerComponent` | later |
| MISC_ITEMS (23) | `other` | escape hatch — count reported every export | hide |
| (right panel) magazines | `magazine` | `MagazineComponent` + !`WeaponComponent` | row now |
| (right panel) ammo | `ammo` | `/Ammo/` + ammo signals | hide in Forge |
| (ACE unique items IDX_VIRT_UNIQUE_*) | — | no analogue | never |
| (no ACE analogue) gloves | `gear_gloves` | AreaType `LoadoutHandwearSlotArea` | later |
| (no ACE analogue) statics | `vehicle_weapon` | `*CompartmentManagerComponent` / `SCR_RocketEjectorMuzzleComponent` / weapon w/o `ItemPhysAttributes` | never (not infantry) |
| abstract templates | any kind + `abstract: true` | filename `*_base.et` OR display `* Base` | always hidden |

**Tier contract (mod-agnosticism):** Tier A = engine-required components (hold for every functioning mod,
inheritance-aware matching in the plugin); Tier B = EntityCatalog `SCR_ArsenalItem` types (coverage measured
per modset, printed each export); Tier C = conventions (`/Handguns/` path, `*_base` suffix — failure is
cosmetic + counted, never silent `gear_primary` pollution; fallthrough → `other` + counted).
