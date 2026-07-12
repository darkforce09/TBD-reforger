# T-068.10.2 census v5 — full disposition of all 1,880 registry items (H_pred freeze)

**Date:** 2026-07-12 (v5; v4 @ `52e938fd`) · **Executor:** Claude Code, operator-approved
full-program session · **Source:** committed T-150 envelope + offline pak reads (census cache) ·
**Data:** [`t068_10_2_census.json`](t068_10_2_census.json) · per-item plugin evidence:
[`t068_10_2_rules_sidecar.txt`](t068_10_2_rules_sidecar.txt) · gates:
[`t068_10_2_gates_output.txt`](t068_10_2_gates_output.txt) · G5 sample:
[`t068_10_2_g5_sample.txt`](t068_10_2_g5_sample.txt)

**v4 → v5** (both census-side rule fixes, discovered by gate G1 against the first live export —
the census loop working as designed):
1. v4 shortcut sent storage-bearing accessories (pouches/holsters/belts) to `other`; v5 mirrors
   the plugin's full tail: storage-without-inventory → `crate`, no-signal → dropped.
2. v5 adds the `Prefabs/Editor/` deny (2 nonsense rows: editor lightning 'magazines') and
   `__dropped__` accounting for decorative no-inventory cloth nodes.

**Plugin-superset corrections** (text census cannot see class inheritance; the plugin can —
each such row is justified per-item in the gates output via the plugin's ruleId sidecar):
- 25 justified moves (medical/repair/rearming kits, mortar rearm boxes → `gear_item` via
  `SCR_GadgetComponent` descendants; 2 concrete gloves via live ancestry).
- 14 inheritance-kept rows census had marked dropped.

## Conservation

- Census rows: **1880** (Σ old = Σ new = 1880 = 1880)
- Predicted export set: 1880 − 35 dropped − 2 denied = 1843 (live export: 1,857 = prediction + 14 inheritance-kept ✓ gates G4)
- Abstract-flagged: **352** (346 visible in export + 6 on dropped/denied rows)
- Rule hits: `{"R0": 1207, "R2": 303, "R8s": 38, "R9": 110, "R7a": 107, "DROP": 35, "R4": 15, "R6": 10, "R3": 47, "R5": 6, "DENY": 2}`

## H_old → H_pred (per kind)

| kind | old | pred | Δ |
|---|---:|---:|---:|
| __denied__ | 0 | 2 | +2 |
| __dropped__ | 0 | 35 | +35 |
| ammo | 101 | 101 | +0 |
| attachment | 26 | 26 | +0 |
| character | 354 | 354 | +0 |
| crate | 276 | 314 | +38 |
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
| magazine | 127 | 125 | -2 |
| optic | 30 | 30 | +0 |
| other | 252 | 110 | -142 |
| vehicle | 218 | 218 | +0 |
| vehicle_weapon | 77 | 87 | +10 |
| **Σ** | **1880** | **1880** | 0 |

## Live export result (run 6, session logs_2026-07-12_21-02-59+)

- Items **1,857**, edges **4,685** (histogram in gates output); tiers A=1757 B=0 C=0 other=100;
  weaponUnsplit=0; unknownArea=0; two consecutive exports byte-identical modulo `generatedAt` (G7).
- **All gates PASS** — G1 per-item (0 unexplained), G1b exact, G2 pollution 0, G3 abstract exact,
  G4 totals reconcile, G4b referential integrity, G5 sample 20/20 slot-exact (seed 68102),
  G6 DB == envelope + idempotent re-import (0 rows) + prune removed exactly the 23 predicted rows.
- **OPEN (non-blocking):** EntityCatalog Tier-B parse = 0/30 confs — root vars discovered
  (`m_eEntityCatalogType, m_aEntityEntryList`), entry iteration debug deferred; `arsenal_type`
  absent this pass; flares stay `gear_launcher` (engine ancestry) as census predicts.

## Transition matrix (old → new, zero rows omitted)

| old → new | n |
|---|---:|
| `character -> character` | 354 |
| `crate -> crate` | 276 |
| `vehicle -> vehicle` | 218 |
| `magazine -> magazine` | 125 |
| `other -> other` | 110 |
| `ammo -> ammo` | 101 |
| `gear_helmet -> gear_helmet` | 92 |
| `gear_primary -> gear_primary` | 84 |
| `vehicle_weapon -> vehicle_weapon` | 77 |
| `gear_uniform -> gear_jacket` | 61 |
| `other -> gear_item` | 55 |
| `gear_backpack -> gear_backpack` | 43 |
| `other -> crate` | 38 |
| `other -> __dropped__` | 35 |
| `gear_vest -> gear_vest` | 34 |
| `gear_uniform -> gear_pants` | 33 |
| `optic -> optic` | 30 |
| `attachment -> attachment` | 26 |
| `gear_primary -> gear_throwable` | 15 |
| `gear_vest -> gear_armored_vest` | 12 |
| `gear_primary -> vehicle_weapon` | 10 |
| `gear_launcher -> gear_launcher` | 9 |
| `gear_primary -> gear_launcher` | 9 |
| `other -> gear_binoculars` | 7 |
| `gear_primary -> gear_explosive` | 6 |
| `gear_uniform -> gear_boots` | 6 |
| `gear_handgun -> gear_handgun` | 4 |
| `other -> gear_gloves` | 4 |
| `other -> gear_glasses` | 3 |
| `magazine -> __denied__` | 2 |
| `gear_primary -> gear_handgun` | 1 |

## Rules (census v5 order = plugin implementation order)

See v4 table plus: R8s storage tail (crate / vehicle_weapon host / other / dropped),
DENY `Prefabs/Editor/`. Full per-item rule + evidence in census.json rows.
