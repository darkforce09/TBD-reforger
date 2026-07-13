# T-068.10.7 — Arsenal paper-doll (ACE layout, 2D SVG soldier, clickable parts)

**Ticket:** T-068 · **Slice:** T-068.10.7 · **Status:** shipped ·
**Executor:** claude-code (Mode D session, operator-approved plan) ·
**Verify:** [`.ai/artifacts/t068_10_7_verify_log.md`](../../../.ai/artifacts/t068_10_7_verify_log.md)

## In one sentence

The Arsenal tab becomes the real ACE Arsenal: LEFT the active region's item **list** (click
to equip — no dropdowns), CENTER a hand-drawn **SVG soldier silhouette** where every loadout
part is itself a clickable hotspot (including the optic and magazine ON the rifle), RIGHT a
contextual column (attachment quick-lists / container capacity / item detail), with an
honest weight readout — replacing the .10.6 dropdown-rows layout inside this tab only.

## Shipped shape

- `arsenalDollModel.ts` — pure region config + weight math. `DOLL_REGIONS` (12 main
  hotspots) + `PRIMARY_SUB_REGIONS` (`optic`/`magazine` ride the rifle) cover every
  `EMPTY_PICKS` key exactly once (vitest-asserted). `loadoutWeight` sums serialized
  `weight_kg` and counts null-weight items as unknown — never guessed;
  `formatLoadoutWeight` renders "≥ 0.3 kg · 1 item without weight data".
- `SoldierSilhouette.tsx` — single-viewBox schematic soldier (Aegis tokens via SVG
  fill/stroke utilities). Per region a keyboard-accessible `<g role="button">` hotspot with
  three states: empty (dashed, dim region label), equipped (primary-tinted fill + item
  label), active (bright fill + thick stroke). Rifle carries optic-bump + magazine
  sub-hotspots (counter-rotated labels); backpack offset behind the torso; BOTH vest slots
  render simultaneously (chest-rig panel + plate-carrier rim).
- `SlotItemList.tsx` — left column: region header + count, search (Esc clears, mount keyed
  on activeKey), grouped rows from `buildGroupedRowOptions` (same abstract/variant filtering
  + never-drop-current + stranded-incompatible semantics as the pickers), explicit
  "— None —" unequip row, click = equip. Edge regions explain themselves: "pick a primary
  first" / compat-down.
- `ArsenalTab` — `[270px | 1fr | 240px]` grid @ `h-[72vh]`; top strip compat badge + weight
  readout; bottom strip validation badge + the existing v2 export download. Right column:
  rifle active → optic/magazine compat quick-lists (click to equip); container equipped →
  `ContainerPanel`; else `ItemDetailPane` (variant links still hop via inspection override).
  Picks persist through the unchanged `updateSlotLoadout` one-undo-step path.
- `AttributesModal` — Arsenal tab `max-w-6xl` → `max-w-7xl`.
- **Untouched:** `ArsenalPicksPanel` (Faction Manager role editor keeps the compact
  dropdown mode), export/migrate/compat plumbing, all other tabs.

## Gates

vitest **353/353** (new `arsenalDollModel` suite on the committed envelope: region
completeness vs `EMPTY_PICKS`; sub-region modeling; RGD5 0.31 kg known / AK-74 null →
unknown / mixed / empty / missing-item weight math; readout formatting) · build +
`tsc --noEmit` clean · lint = pre-existing `router.tsx` only · operator visual at the
Mode D pause (screenshot checklist in the verify log).
