# T-068.10.4 verify log — loadout document v2 (Reforger-shaped)

**Date:** 2026-07-13 · **Executor:** Claude Code (operator-approved full-program session) ·
**Spec:** `docs/specs/Mission_Creator_Architecture/t068_10_4_loadout_doc_v2.md` ·
**Engine evidence:** `.ai/artifacts/t068_10_2_census.md`

## Status

**PASS (L1–L5).** Arsenal now expresses a real Reforger kit: both simultaneous vest slots,
jacket/pants/boots, four slot-indexed weapon slots (2nd untyped primary = "Launcher / 2nd
rifle"), gloves — 14 picker rows total. Operator visual + T-068.11 unpark decision at this
pause.

## U6 resolution (mod reader tolerance → single-file emission)

`TBD_LoadoutEquipComponent.RunLoadoutTest` parses via `JsonLoadContext.ReadValue("", doc)`
into a declared-members struct — unknown JSON fields are ignored, `doc.gear` presence is the
only structural requirement (`TBD_LoadoutEquipComponent.c:103-115`). → v2 export is ONE file
carrying `wear/weapons/equipment/cargo` PLUS a **derived legacy `gear` block**
(jacket→uniform, armoredVest else vest→vest — armored wins as the visually dominant piece —
headCover→helmet, weapons slot 0→primary/optic/magazine). The Phase-1 NPC path keeps working
unchanged until T-068.12 reads v2 natively.

## What shipped

- `loadout-export.schema.json` → `oneOf` v1|v2; v2 = `wear` open map (canonical keys
  headCover/jacket/pants/boots/vest/armoredVest/backpack/handwear; pattern-open for mod
  areas), slot-indexed `weapons[]` (`slotIndex`/`slotType`/`weapon`/`optic`/`magazine`/
  `attachments`), `equipment`/`cargo` skeletons, required derived `gear`. v2 golden
  (`loadout-export.v2.sample.json`) uses the real USSR two-vest kit GUIDs. Codegen rerun.
- `SlotLoadout` union in `tactical-map/state/schema.ts`: `SlotLoadoutV1` (unchanged shape) |
  `SlotLoadoutV2` (+`LoadoutWeapon`, `isLoadoutV2`); docs keep v1 on disk, editor migrates on
  read and writes v2 only. `updateSlotLoadout`/persistence/compiler untouched (summary is a
  common member; compiler still emits the summary string only).
- `migrateLoadout.ts`: v1→v2, **AreaType-aware** — the v1 `vest` value is looked up in the
  registry and routed `gear_armored_vest`→`wear.armoredVest` else `wear.vest` (unknown items
  default to `wear.vest`, never guessed armored).
- `arsenalRules.ts`: pick keys ×14; `WEAPON_SLOTS` map (primary 0/primary, launcher
  1/primary — the second untyped slot, handgun 2/secondary, throwable 3/grenade);
  `picksToLoadout`→v2; summary appends the 2nd weapon. Rows: Primary/Optic/Magazine/
  Launcher/Handgun/Throwable/Helmet/Jacket/Pants/Boots/Vest (chest rig)/Armored vest/
  Backpack/Gloves — all on the .10.3 grouped/sorted/searchable/abstract-filtered pipeline.
  `PENDING_V2_KINDS` shrinks to glasses/binoculars/gear_item (equipment slice; glasses defer
  on unknown engine slot name — no vanilla character carries a Googles LoadoutSlotInfo).
- `loadoutExport.ts`: v2 envelope + derived gear; download = v2 single file.

## Gates

| # | Assertion | Result |
|---|-----------|--------|
| L1 | v1 golden + v2 golden validate (`oneOf`) | PASS — `npm run validate` all green |
| L2 | Migration property test, real GUIDs: `Vest_6B2 → wear.armoredVest`, `Vest_Lifchik → wear.vest` (sanity-asserts the envelope kinds first) | PASS |
| L2b | v2 expresses BOTH vests at once (the case v1 cannot) | PASS |
| L3 | v1 → migrate → picks → v2 round-trip: identical wear + weapons | PASS |
| L4 | Forge writes v2; v1 docs load via migrate-on-read; v2 pass-through identity; unknown-item vest defaults safe | PASS (vitest) + operator manual at pause |
| L5 | build clean · `npx tsc --noEmit` clean · vitest **334/334** · lint = 1 pre-existing `router.tsx` error only (a new complexity finding in `slotLoadoutToGear` was fixed) · codegen drift = the intended regen, committed | PASS |

Manual check path: Arsenal tab on a character slot → 14 rows; pick AK-74 + RPG (Launcher
row) + RGD5 (Throwable) + Jacket/Pants/Boots + BOTH vests → download →
`loadout-export.json` has `loadoutVersion "2"`, `weapons` slot-indexed, and a legacy `gear`
block; drop it as `$profile:TBD_LoadoutTest.json` and the Phase-1 NPC still dresses.
