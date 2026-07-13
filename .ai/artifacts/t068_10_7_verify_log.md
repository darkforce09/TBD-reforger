# T-068.10.7 verify log — Arsenal paper-doll (SVG soldier, clickable parts)

**Date:** 2026-07-13 · **Executor:** Claude Code (Mode D session) ·
**Spec:** `docs/specs/Mission_Creator_Architecture/t068_10_7_arsenal_paper_doll.md`

## Status

**PASS (automated).** Operator visual = the Mode D pause (checklist below).

| # | Assertion | Result |
|---|-----------|--------|
| M1 | Region completeness: `DOLL_REGIONS` (12) + `PRIMARY_SUB_REGIONS` (`optic`,`magazine`) == `EMPTY_PICKS` keys (14), each exactly once; subs are not main regions; `primary` is | PASS (vitest) |
| M2 | `loadoutWeight` on committed-envelope GUIDs: RGD5 → 0.31 kg known / 0 unknown; AK-74 → 0 known / 1 unknown (data-truth guard asserts the envelope still serializes no rifle weight); mixed → 0.31 / 1 / 2; empty → zeros; catalog-missing resource → unknown | PASS (vitest) |
| M3 | `formatLoadoutWeight`: unknown > 0 → "≥ 0.3 kg · 1 item without weight data"; all-known → "0.6 kg · 2 items" | PASS (vitest) |
| L1 | Left list = same pipeline as the pickers (`buildGroupedRowOptions`): abstract + `variant_of` rows hidden, live pick never blanks, stranded pick listed with the incompatible suffix, query filter never drops the current pick | PASS (code path — pipeline unchanged, covered by the existing `arsenalRules` suite) |
| L2 | Edge regions: no primary → "pick a primary first"; worker down → compat-down message (kind regions keep full catalog) | PASS (code path) |
| C1 | Right column precedence: rifle active+picked → optic/magazine quick-lists (click to equip via the same `onPick`); container equipped → `ContainerPanel`; else `ItemDetailPane` with variant-hop inspection reset on region change/pick | PASS (code path; visual at pause) |
| V1 | Modal expands only on the Arsenal tab (`max-w-7xl` through shared DialogContent, tailwind-merge override; other tabs untouched) | PASS (code path; visual at pause) |
| V2 | SVG state utilities compiled: `.fill-primary`(/15/25), `.stroke-primary`, `.stroke-outline-variant`, `.fill-none`, `.stroke-1`, `stroke-width:2.5`, `stroke-dasharray` all present in the production CSS bundle (Tailwind silently drops unknown classes — checked, not assumed) | PASS (grep dist) |
| F | Full suite **353/353** (346 prior + 7 new) · `npm run build` clean · `tsc --noEmit` clean · lint = pre-existing `router.tsx` only | PASS |

## Notes

- Silhouette geometry sanity-checked off-app: the SVG rendered standalone (mixed
  empty/equipped/active states) and label/hotspot collisions fixed before ship (rifle moved
  below the belt kit; pants/handgun/primary labels repositioned).
- `ArsenalPicksPanel` untouched — Faction Manager role editor still uses the compact
  dropdown mode.

## Operator visual checklist (pause)

1. Hard-refresh the editor tab (Vite full reload is blocked on the editor route). Open a
   character slot → Attributes → Arsenal: modal visibly wider; soldier silhouette centered;
   left list shows Primary (default active region); right column shows the detail/empty
   state.
2. Click the helmet region → left list becomes helmets; click a helmet → dome tints and
   shows its name.
3. Click the rifle → primaries list (21 real primaries, variant-collapsed); equip an AK-74N.
4. Click the OPTIC bump on the rifle → optics compat feed in the left list AND the right
   quick-list; equip 1P29 from the right quick-list — both update.
5. Equip 6B2 (armored vest) + Lifchik (chest rig) → BOTH torso overlays fill simultaneously.
6. Weight readout (top right): known sum with "≥ … items without weight data" — rifle
   weights are honestly unknown, never invented.
7. Hotspot states distinct: dashed empty vs tinted equipped vs thick-stroke active; Tab +
   Enter keyboard-select works.
8. Bottom bar: validation badge; Export still downloads v2 JSON with the legacy gear block.
9. Faction Manager role editor unchanged (compact dropdown panel).
