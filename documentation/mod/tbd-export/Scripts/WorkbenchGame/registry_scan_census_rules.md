**Status:** live

# Registry scan census rules

The classification rules the registry items export applies to every prefab it scans, in the order
`ClassifyAndCollect` in `mod/tbd-export/Scripts/WorkbenchGame/TBD_RegistryScan.c` applies them,
and the kind distribution a census of the vanilla registry predicts from them. Each exported item
carries the id of the rule that classified it (`ruleId`), and the plugin's debug sidecar writes
one `resource_name;ruleId` line per item (`TBD_RegistryItemsExportPlugin.c`), so every kind in an
export traces back to one rule below.

## Rules

The first matching rule wins.

| Rule | Signal | Kind |
|---|---|---|
| DENY | the path holds `/Structures/`, `/Rocks/`, `/Trees/`, `/Debris/`, `/Foliage/` or `Prefabs/Editor/` (`DENY_HARD`) | not exported |
| R0 | a vehicle root or vehicle simulation component | `vehicle` |
| R0 | a character root or `CharacterControllerComponent` | `character` |
| R0 | a `MagazineComponent` without a weapon | `magazine` |
| R2 | a `LoadoutClothComponent` with a mapped `AreaType` | the area's `gear_*` kind |
| R2_UNKNOWN_AREA | a named but unmapped `AreaType` (counted and logged) | `other` |
| R3 | `SCR_BinocularsComponent` | `gear_binoculars` |
| R3 | any other `SCR_GadgetComponent` (compass, map, radio, flashlight, consumable, detonator) | `gear_item` |
| R4 | `GrenadeMoveComponent` (before weapons: grenades carry a `WeaponComponent`) | `gear_throwable` |
| R5 | an explosive charge or trigger component (before weapons) | `gear_explosive` |
| R7a | a weapon whose ancestry is a vanilla weapon family (before R6, so abstract templates keep their family) | the family's kind |
| R6 | a weapon with a compartment manager or rocket ejector muzzle, or one that cannot be carried | `vehicle_weapon` |
| R7b | a carryable weapon with `MuzzleInMagComponent` | `gear_launcher` |
| R7c | a carryable weapon under `/Handguns/` or `/Launchers/` | `gear_handgun`, `gear_launcher` |
| R7d | any other carryable weapon (counted as unsplit) | `gear_primary` |
| R0 | an inventory item declaring an attachment type | `optic` or `attachment` |
| R0 | a path under `/Ammo/` | `ammo` |
| R0 | storage without an inventory item, hosting a weapon slot | `vehicle_weapon` |
| R0 | storage without an inventory item | `crate` |
| R9 | an inventory item with no stronger signal (quarantined, never `gear_primary`) | `other` |

A prefab no rule matches is dropped. After the scan, the faction `Configs/EntityCatalog`
entries refine three cases as R1: an `other` row takes the catalog's kind, an R7d weapon takes
the catalog's weapon family, and a launcher the catalog lists as a throwable becomes
`gear_throwable`.

## Census of the vanilla registry

The census reads the vanilla prefabs offline and predicts each item's kind from the rules above;
an export over the same game data matches it item for item, apart from rows the plugin keeps
through class inheritance that a text census cannot see.

| Kind | Items |
|---|---:|
| `ammo` | 101 |
| `attachment` | 26 |
| `character` | 354 |
| `crate` | 314 |
| `gear_armored_vest` | 12 |
| `gear_backpack` | 43 |
| `gear_binoculars` | 7 |
| `gear_boots` | 6 |
| `gear_explosive` | 6 |
| `gear_glasses` | 3 |
| `gear_gloves` | 4 |
| `gear_handgun` | 5 |
| `gear_helmet` | 92 |
| `gear_item` | 55 |
| `gear_jacket` | 61 |
| `gear_launcher` | 18 |
| `gear_pants` | 33 |
| `gear_primary` | 84 |
| `gear_throwable` | 15 |
| `gear_vest` | 34 |
| `magazine` | 125 |
| `optic` | 30 |
| `other` | 110 |
| `vehicle` | 218 |
| `vehicle_weapon` | 87 |
| dropped (no rule matched) | 35 |
| denied | 2 |
| **Total** | **1880** |

The predicted export set is 1,843 items (1,880 less 35 dropped and 2 denied); the export holds
1,857, the prediction plus 14 rows kept through inheritance. Flares stay `gear_launcher` through
their engine ancestry unless the catalog refinement moves them.

## Open work

- In the census export the EntityCatalog refinement found the catalog roots
  (`m_eEntityCatalogType`, `m_aEntityEntryList`) but parsed no entries, so no row took an R1 kind
  and `arsenal_type` stayed empty; no ticket covers it yet.
