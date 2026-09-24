# T-068.10.4 — Loadout document v2 (Reforger-shaped)

**Ticket:** T-068 · **Slice:** T-068.10.4 · **Status:** queued (starts after the T-068.10.3
operator pause) · **Executor:** claude-code ·
**Authority:** [`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md) ·
**Engine evidence:** [`.ai/artifacts/t068_10_2_census.md`](../../../.ai/artifacts/t068_10_2_census.md) §sources

---

## In one sentence

Replace the ACE-shaped 6-field `SlotLoadout` with a Reforger-shaped v2 document — wear areas
as an open map keyed by engine slot name, slot-indexed `weapons[]`, equipment/cargo skeletons —
so a real kit (two vests, jacket+pants+boots, rifle+launcher) is expressible before the
compiler work (T-068.11) unparks.

## Problem

`SlotLoadout` (`tactical-map/state/schema.ts:55-65`) and `loadout-export.schema.json` model
`{primary, uniform, vest, helmet, optic, magazine}`. Vanilla characters wear Jacket+Pants+
Boots (three areas) and ArmoredVest+Vest (simultaneously); characters have two untyped primary
weapon slots. The v1 document cannot express a vanilla USSR rifleman.

## Goal

1. **SlotLoadout v2** (editor doc + contract):
   - `wear`: open map keyed by engine `LoadoutSlotInfo` name — canonical keys
     `headCover, jacket, pants, boots, vest, armoredVest, backpack, handwear` documented,
     pattern-validated extras allowed (mod areas representable without schema change).
   - `weapons[]`: `{ slotIndex, slotType, weapon, optic, magazine, attachments[] }`.
   - `equipment` (binoculars, wristwatch — skeleton), `cargo[]` (`{container, item, qty}` —
     skeleton; UI in the later cargo slice).
2. **loadout-export.schema.json v2** with `loadoutVersion: "2"`; U6 outcome (mod JSON reader
   tolerance, `TBD_LoadoutEquipComponent.c` read paths) decides single-file vs dual-file
   emission — v1 consumers must keep working until T-068.12.
3. **Migration** v1 → v2: `uniform → wear.jacket`; `vest → wear.vest` OR `wear.armoredVest`
   by looking the item up in the registry (area-derived kind — never string-guessed);
   `primary/optic/magazine → weapons[0]`.
4. Forge writes v2; compiler keeps emitting the summary string only (T-068.11 PARKED).
5. Landmines honored (from the program review): `EquipCloth` routes by the item's own
   AreaType; two primaries need slot-indexed equip at T-068.12; round-count is impossible
   (`ammo_in_mag` OPEN) and must not appear in UI; grenade slot ≠ cargo spares.

## Out of scope

Mod reader changes (T-068.12) · compiler loadout block (T-068.11, PARKED) · cargo/equipment
UI (fields exist, UI later) · favorites/library.

## Verify (gates)

| # | Assertion | Method |
|---|-----------|--------|
| L1 | v1 goldens still validate; v2 goldens validate | `npm run validate` |
| L2 | Migration property test with real GUIDs: USSR rifleman kit (Vest_6B2 → `armoredVest` AND Vest_Lifchik → `vest`) — the case v1 cannot express | vitest |
| L3 | Round-trip: v1 doc → migrate → v2 → resolved equip-set identical | vitest |
| L4 | Forge writes v2; existing missions load (migration on read) | vitest + manual |
| L5 | build + lint + vitest green; contract codegen zero-drift | exit 0 |

## Acceptance

- [ ] v2 schema + contract + migration + Forge write path shipped; L1–L5 PASS.
- [ ] Verify log + tag **T-068.10.4**.
- [ ] PAUSE — operator decides whether Arsenal is "proper" → unpark T-068.11.
