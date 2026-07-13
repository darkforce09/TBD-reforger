# T-068.10.6 verify log — Arsenal expanded modal (ACE panes)

**Date:** 2026-07-13 · **Executor:** Claude Code (Mode C session) ·
**Spec:** `docs/specs/Mission_Creator_Architecture/t068_10_6_arsenal_expanded_modal.md`

## Status

**PASS (automated).** Operator visual = the Mode C close-out pause.

| # | Assertion | Result |
|---|-----------|--------|
| D1 | `itemDetail` selector vs committed envelope (real GUIDs): weapons expose **null** phys attrs (no vanilla primary serializes Weight/ItemVolume — data-truth discovered in test, never-guess rule holds) and are NOT containers despite their weapon-attachment storage capacity (semantic exclusion, commented) | PASS |
| D2 | RGD5 phys flow (0.31 kg / 100 cm³); concrete backpack container detection via exported max fields | PASS |
| D3 | Variant relations: `AK74N 1P29 → AK74N` back-link; reverse configurations list contains 1P29 + GP25, locale-sorted | PASS |
| D4 | Unknown resource → null; abstract flag surfaces | PASS |
| V3 | Modal expands only on the Arsenal tab (`max-w-6xl` through shared DialogContent, tailwind-merge override; other tabs untouched) | PASS (code path; visual at pause) |
| F | Full suite **346/346** · build clean · `tsc --noEmit` clean · lint = pre-existing `router.tsx` only (two new-code lint findings fixed during the slice: non-null assertions in the test → `must()` guard; nested non-null in the pane → scoped local) | PASS |

## Operator visual checklist

Open a character slot → Attributes → Arsenal: modal visibly wider; left pane fills on
first pick (silhouette box, name, kind/addon chips, weight/volume "—" for rifles — engine
defaults are not invented); pick a backpack/vest → right Container panel with capacity +
cargo placeholder; on an AK-74N the left pane lists its factory configurations as links,
and a 1P29 config links back ("Configuration of Rifle AK74N").
