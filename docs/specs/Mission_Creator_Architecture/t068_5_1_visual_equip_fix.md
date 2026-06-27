# T-068.5.1 — Visual loadout equip fix (make-or-break)

**Ticket:** T-068 · **Slice:** T-068.5.1  
**Status:** **shipped** @ `b233b11` (git tag **T-068.5.1**)  
**Executor:** claude-code (targets `mod`)
**Authority:** [`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md) ·
[`t068_5_mod_equip_loadout.md`](t068_5_mod_equip_loadout.md) ·
[`t068_6_phase1_e2e_gate.md`](t068_6_phase1_e2e_gate.md) (E11)

---

## In one sentence

`TBD_LoadoutEquipComponent` logs `equip OK` but the test character spawns **visually naked** —
fix the **wear** path so the kit is actually worn, with a post-equip worn-verify gate.

---

## Problem

`EquipSlot` used `SCR_InventoryStorageManagerComponent.TryInsertItem(item, PURPOSE_LOADOUT_PROXY/ANY)`
and logged `equip OK` on a `true` return. A generic insert lands the item in **storage**, not the
body loadout area → the character is naked despite the OK log (Samuel screenshot @ TBD_Dev_POC play).
E11 / T-068.6 Phase 1 sign-off is blocked.

---

## Plan audit (root cause)

| Assumption | Valid? | Evidence |
|---|---|---|
| Wrong GUID on `Character_US_Base.et` is the bug | **NO** | `game_read` → `.et` `ID "520EC961A090BBD5"` = the `m_sTestCharacter` default; spawn succeeds both runs (`Resource.Load` could not resolve an unresolvable GUID). GUID is canonical — **keep it**. |
| `TryInsertItem` ≠ wear | **YES** | generic insert routes to storage, not the loadout area. The real fix. |

**Decision:** do not change the character GUID. Fix the wear path only.

---

## Vanilla API (confirmed via `api_search`)

- `SCR_InventoryStorageManagerComponent.EquipCloth(IEntity item)` → **void** (no bool; must verify worn).
- `SCR_InventoryStorageManagerComponent.EquipWeapon(IEntity item, SCR_InvCallBack cb=null, bool bFromVicinity=true)` → bool.
- `SCR_CharacterInventoryStorageComponent.GetClothFromArea(TypeName areaType)` → `IEntity` (worn cloth in area).
- `SCR_CharacterInventoryStorageComponent.GetCurrentWeapon()` → `BaseWeaponComponent` (worn weapon).
- LoadoutArea typenames (subclasses of `LoadoutAreaType`): uniform/jacket = **`LoadoutJacketArea`**,
  vest = **`LoadoutVestArea`**, helmet = **`LoadoutHeadCoverArea`**.

---

## Implementation (mod only — `TBD_LoadoutEquipComponent.c`)

- Spawn item prefab (unchanged).
- **Clothing** (uniform/vest/helmet): `mgr.EquipCloth(item)` (drop `TryInsertItem`).
- **Weapon** (primary): `mgr.EquipWeapon(item)`.
- **Deferred worn-verify** (Amendment 2 — `EquipCloth` void / `EquipWeapon` may callback async): after
  issuing all equips, `CallLater` one tick, then per slot read
  `SCR_CharacterInventoryStorageComponent` on the character:
  - cloth → `GetClothFromArea(<area>)` returns the spawned item (compare entity).
  - weapon → `GetCurrentWeapon()` non-null and its owner == the spawned item (Amendment 4 — not the
    `EquipWeapon` return alone, not a `PURPOSE_ANY` backpack insert).
- Log `equip OK <ResourceName>` **only** on worn-verify success; else
  `[TBD][Loadout] <slot> FAILED (not worn) <ResourceName>` and delete the orphan item.
- Hardening: log resolved character ResourceName (full GUID) on spawn; per-slot verify detail
  (area typename + entity id).
- **Out of scope:** web/schema, `gear_pants`, DeployPlayer/T-068.11, swapping to Rifleman.

---

## Acceptance (A0–A9)

| ID | Gate |
|----|------|
| A0 | **(Amendment 1)** PASS = spawn succeeds (`test spawn` + entity id @ ~6400) AND all non-null slots pass worn-verify. A lingering `RESOURCES … Wrong GUID … Character_US_Base` with spawn+wear OK = **WARNING**, not FAIL; do not revert GUID for editor noise. FAIL only if Wrong GUID ↔ spawn failure / wrong body. |
| A1 | `[TBD][Loadout] Loaded TBD_LoadoutTest.json` present |
| A2 | primary `equip OK` after weapon worn-verify (`GetCurrentWeapon`) |
| A3 | uniform `equip OK` after `GetClothFromArea(LoadoutJacketArea)` |
| A4 | vest `equip OK` after `GetClothFromArea(LoadoutVestArea)` |
| A5 | helmet `equip OK` after `GetClothFromArea(LoadoutHeadCoverArea)` |
| A6 | (optional) forced bad ResourceName → `FAILED`, not OK |
| A7 | **VISUAL (human)** — play `TBD_Dev_POC.conf`; pawn @ spawn shows primary + jacket + vest + helmet; Samuel screenshot for E11 |
| A8 | logged ResourceNames EXACT-match `jq -r '.gear'` of `$profile:TBD_LoadoutTest.json` |
| A9 | swap one slot in profile JSON → replay shows new GUID, old gone |

**Phase 1 / E11 signs off only after A7 (Samuel screenshot).**

---

## Phase 1 boundary

Equips kit on the **non-player test NPC** spawned by `TBD_LoadoutEquipComponent` @ game-mode coords. Does **not** equip the **joining human player** or mission `DeployPlayer` bodies — that is **T-068.11**.

---

## Shipped @ T-068.5.1 (`b233b11`)

| Change | Detail |
|--------|--------|
| Wear API | `EquipCloth` / `EquipWeapon` replace `TryInsertItem`-first path |
| Verify | Deferred worn-check via `GetClothFromArea` (jacket/vest/head) + `GetCurrentWeapon` |
| Character GUID | Unchanged `{520EC961A090BBD5}…Character_US_Base.et` (canonical) |
| A7 | Samuel screenshots — M60, BDU jacket, PASGT vest, helmet+goggles on test NPC |

---

## Depends on / Unblocks

- **Depends on:** T-068.5 (component scaffold)
- **Unblocks:** T-068.6 E11 → Phase 1 sign-off (**PASS** @ 2026-06-27)
