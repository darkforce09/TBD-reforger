# T-068.10.2 census — full disposition of all 1,880 registry items (H_pred freeze)

**Date:** 2026-07-12 · **Executor:** Claude Code (operator-approved full-program session) ·
**Source:** committed `registry-items.workbench.json` (T-150 export) + offline pak reads via
`scripts/mod/mcp-call.sh game_read/asset_search` (671 leaf + ancestor-chain prefab reads, cached) ·
**Data:** [`t068_10_2_census.json`](t068_10_2_census.json) — one disposition row per item:
`resource_name, old_kind, target_kind, rule_id, evidence, abstract, weight_kg, volume_cm3`.

**Contract:** the T-068.10.2 classifier rewrite must reproduce `H_pred` below EXACTLY
(gate G1). Any per-kind diff must be justified line-item in the verify log against this
census or the pre-authorized deltas in §Caveats — otherwise FAIL.

## Conservation

- Items: **1880** (all disposed; Σ old = Σ new = 1880 = 1880)
- Abstract-flagged: **352** (filename `*_base.et`/`*_Base.et` OR display `* Base`/`* base`)
- Rule hits: `{"R0": 1209, "R2": 303, "R9": 183, "R7a": 107, "R4": 15, "R6": 10, "R3": 47, "R5": 6}` (Σ = 1880)
- Read errors: 0 · unknown areas: 0 · weapon_unsplit: 0

## H_old → H_pred (per kind)

| kind | old | pred | Δ |
|---|---:|---:|---:|
| ammo | 101 | 101 | +0 |
| attachment | 26 | 26 | +0 |
| character | 354 | 354 | +0 |
| crate | 276 | 276 | +0 |
| gear_armored_vest | 0 | 12 | +12 |
| gear_backpack | 43 | 43 | +0 |
| gear_binoculars | 0 | 7 | +7 |
| gear_boots | 0 | 6 | +6 |
| gear_explosive | 0 | 6 | +6 |
| gear_glasses | 0 | 3 | +3 |
| gear_gloves | 0 | 4 | +4 |
| gear_handgun | 4 | 5 | +1 |
| gear_helmet | 92 | 92 | +0 |
| gear_item | 0 | 55 | +55 |
| gear_jacket | 0 | 61 | +61 |
| gear_launcher | 9 | 18 | +9 |
| gear_pants | 0 | 33 | +33 |
| gear_primary | 125 | 84 | -41 |
| gear_throwable | 0 | 15 | +15 |
| gear_uniform | 100 | 0 | -100 |
| gear_vest | 46 | 34 | -12 |
| magazine | 127 | 127 | +0 |
| optic | 30 | 30 | +0 |
| other | 252 | 183 | -69 |
| vehicle | 218 | 218 | +0 |
| vehicle_weapon | 77 | 87 | +10 |
| **Σ** | **1880** | **1880** | 0 |

## Transition matrix (old → new, zero rows omitted)

| old → new | n |
|---|---:|
| `character->character` | 354 |
| `crate->crate` | 276 |
| `vehicle->vehicle` | 218 |
| `other->other` | 183 |
| `magazine->magazine` | 127 |
| `ammo->ammo` | 101 |
| `gear_helmet->gear_helmet` | 92 |
| `gear_primary->gear_primary` | 84 |
| `vehicle_weapon->vehicle_weapon` | 77 |
| `gear_uniform->gear_jacket` | 61 |
| `other->gear_item` | 55 |
| `gear_backpack->gear_backpack` | 43 |
| `gear_vest->gear_vest` | 34 |
| `gear_uniform->gear_pants` | 33 |
| `optic->optic` | 30 |
| `attachment->attachment` | 26 |
| `gear_primary->gear_throwable` | 15 |
| `gear_vest->gear_armored_vest` | 12 |
| `gear_primary->vehicle_weapon` | 10 |
| `gear_launcher->gear_launcher` | 9 |
| `gear_primary->gear_launcher` | 9 |
| `other->gear_binoculars` | 7 |
| `gear_primary->gear_explosive` | 6 |
| `gear_uniform->gear_boots` | 6 |
| `gear_handgun->gear_handgun` | 4 |
| `other->gear_gloves` | 4 |
| `other->gear_glasses` | 3 |
| `gear_primary->gear_handgun` | 1 |

## Rules (census order = plugin implementation order)

| id | rule | evidence recorded |
|---|---|---|
| R0 | family unchanged (character/vehicle/vehicle_weapon/crate/magazine/ammo/optic/attachment keep T-150 rules) | — |
| R2 | `BaseLoadoutClothComponent.AreaType` exact class → wear kind (Jacket/Pants/Boots/Vest/**ArmoredVestSlot**/HeadCover/Cover/Backpack/Googles/**HandwearSlot**/Binoculars/IdentityItem/**Watch**) | area class |
| R3 | `SCR_GadgetComponent` family (Binoculars/Compass/MapGadget/Radio/Flashlight/ConsumableItem/DetonatorGadget/BallisticTable/MortarShellGadget) → gear_binoculars / gear_item | component |
| R4 | `GrenadeMoveComponent` → gear_throwable (before weapon rules — throwables carry WeaponComponent) | component |
| R5 | `SCR_ExplosiveCharge*/SCR_ExplosiveTriggerComponent` → gear_explosive (before weapon rules — DemoBlocks carry WeaponComponent) | components |
| R7a | weapon + vanilla family ancestor (`Rifle_Base`/`MachineGun_Base`/`LongRangeRifle_Base`/`Handgun_Base`/`Launcher_Base`; `Weapon_Base` deliberately unmapped) | ancestor file |
| R6 | statics: `*CompartmentManagerComponent` (crewed) OR `SCR_RocketEjectorMuzzleComponent` (pods) OR weapon without `ItemPhysAttributes`/`*InventoryItemComponent` (not carryable) → vehicle_weapon | discriminator |
| R7b/c | weapon fallback: `*MuzzleInMagComponent` → gear_launcher; path `/Handguns/`,`/Launchers/` | component/path |
| R7d | weapon_unsplit → gear_primary + counted (0 in vanilla) | — |
| R9 | no signal → other + counted | — |

Plugin note: census matches component classes by NAME (pak text); the plugin matches by
class-ancestry (`ToType().IsInherited`) — a strict superset. Identical on vanilla (exact
class names in use); mod subclasses classify in the plugin that a text census would miss.

## Caveats / pre-authorized G1 deltas

1. **Flares → gear_launcher (8)** — engine ancestry (`Launcher_Base`): `Flare RSP30 red`, `FlareStarParachute M126A1 red`, `FlareStarParachute base`, `Flare RSP30 white`, `FlareStarParachute M195 green`, `FlareStarParachute M127A1 white`, `Flare RSP30 Base`, `Flare RSP30 green`.
   If the live EntityCatalog pass (R1) types them `NON_LETHAL_THROWABLE`, refinement to `gear_throwable` for exactly these rows is pre-authorized (G1 diff ≤ 8, names above).
2. **Deployable weapon parts & sandbags wear as backpacks (18)** — engine truth (`LoadoutBackpackArea` on `DeployablePart` ancestry): tripod/barrel/baseplate parts are back-carried. They stay `gear_backpack`; Forge groups them by `category` so real packs and parts don't interleave.
3. **`Weapon_Base` → vehicle_weapon (abstract)** — the root weapon template matches no family and has no phys attrs; it is UI-hidden via `abstract`. Mapping it to a family would bypass the statics rule for cannons/mortars.
4. **Unresolved ancestor refs (35 refs / 6 fragments)** — pak header garble ate the fragment starts: `{"structible_Props_Base.et": 21, "Base.et": 8, "ear/Wool_Gloves/Wool_Gloves_01_base.et": 1, "estructible_Props_Base.et": 3, "ear/Pilot_Gloves/Pilot_Gloves_Base.et": 1, "eldDressing_Base.et": 1}`. All inert: affected items already classified from leaf/mid-chain signals (gloves via HandwearSlotArea, props roots carry no cloth/weapon/gadget signal). Zero dispositions depend on them.
5. **`gear_uniform` retires to 0 rows** (split into jacket/pants/boots) but stays in the schema enum for Phase-1 payload compatibility.
6. **`LoadoutWatchArea` discovered** (8 rows → gear_item) — U4 now 13/15 LoadoutAreaType subclasses enumerated; remaining 2 resolved at the live Workbench spike.

## New-kind populations (v3 enum additions)

- `gear_throwable`: 15 rows (5 abstract)
- `gear_explosive`: 6 rows (4 abstract)
- `gear_jacket`: 61 rows (15 abstract)
- `gear_pants`: 33 rows (10 abstract)
- `gear_boots`: 6 rows (1 abstract)
- `gear_armored_vest`: 12 rows (6 abstract)
- `gear_glasses`: 3 rows (1 abstract)
- `gear_gloves`: 4 rows (3 abstract)
- `gear_binoculars`: 7 rows (4 abstract)
- `gear_item`: 55 rows (22 abstract)

## Weight/volume capture (offline, prefab-text level)

- `weight_kg` present in prefab text for 612 items; `volume_cm3` for 547 (units per ItemPhysicalAttributes API docs: kg / cm3).
- Absent values = engine class defaults not serialized in the prefab — exported as null, NEVER guessed (U1 probes the default source at the live spike).

