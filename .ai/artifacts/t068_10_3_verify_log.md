# T-068.10.3 verify log — Forge picker UX on clean kinds

**Date:** 2026-07-12 · **Executor:** Claude Code (operator-approved full-program session) ·
**Spec:** `docs/specs/Mission_Creator_Architecture/t068_10_3_forge_picker_ux.md` ·
**Data基:** T-068.10.2 census-gated export (`be85a26e`, tag T-068.10.2)

## Status

**PASS (automated F1–F5).** F6 = operator visual check (this pause).

## What shipped

- `arsenalRules.ts`: row set v2 within SlotLoadout-v1 persistence limits — implementation
  decision per spec Goal 5: the `uniform` field row relabels **Jacket** and sources
  `gear_jacket` (v1→v2 migration maps `uniform → wear.jacket`); `vest` row sources
  `gear_vest` (chest rigs); armored vest / pants / boots / backpack / launcher / handgun /
  throwable / equipment rows land in **T-068.10.4** with the v2 fields that can store them
  (`PENDING_V2_KINDS` hint in the tab). No dead/disabled picker rows shipped.
- Option pipeline: `abstract` templates excluded everywhere (a live abstract pick never
  blanks); locale-alpha sort; case-insensitive display-name search that never removes the
  current pick; category-derived `<optgroup>` buckets (addon segment dropped, single-bucket
  collapse); stranded-pick retention unchanged.
- `SelectField` gains optional `groups` (optgroups); `ArsenalTab` renders search box +
  grouped pickers + pending-kinds hint; degrade paths untouched.
- `registryGraph.test.ts` envelope stats updated to the .10.2 export (4,685 edges,
  6 families; 16 mag edges moved family with the statics reclassification — commented).

## Gates

| # | Assertion | Result |
|---|-----------|--------|
| F1 | Row option counts == envelope non-abstract kind counts (numbers inline: primary 58, jacket 46, vest 28, helmet 68) | PASS (vitest, real committed envelope via fs read) |
| F2 | Locale-sorted options (sorted-copy equality) | PASS |
| F3 | Abstract exclusion (`Rifle M16A2 base` named; exactly 26 abstract primaries) | PASS |
| F4 | `Grenade RGD5` + `Smoke M18 Red` are gear_throwable and absent from the primary row; no smoke/grenade/pod/mortar/cannon label in primary options | PASS |
| F5 | build + lint + full vitest | PASS — build clean; vitest **325/325**; lint = 1 pre-existing `router.tsx` react-refresh error, reproduced on clean HEAD via stash (same disposition as T-150) |
| F6 | Operator visual (grouped pickers + search + hint) | **PENDING — this pause** |

Manual check path: `make web` → `/missions/:id/edit` → place/select a character slot →
Attributes → Arsenal. Expect: search box; Primary shows 58 rifles/MGs/snipers grouped by
`Weapons/…` buckets; Jacket/Vest/Helmet grouped + sorted; zero `* base` rows; grenades only
absent (their row arrives with .10.4); hint line about pending kinds.
