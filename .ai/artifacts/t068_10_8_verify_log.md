# T-068.10.8 verify log — Arsenal UX pass 2 (no-scroll, A3 rail, clean doll)

**Date:** 2026-07-13 · **Executor:** Claude Code (Mode D session) ·
**Spec:** `docs/specs/Mission_Creator_Architecture/t068_10_8_arsenal_ux_pass2.md`

## Status

**PASS (automated).** Operator visual = the Mode D pause (checklist below).

| # | Assertion | Result |
|---|-----------|--------|
| R1 | `RAIL_REGIONS` covers every `EMPTY_PICKS` key exactly once (14 keys, no dupes) | PASS (vitest) |
| R2 | Rail entries map 1:1 to lucide icons (`RAIL_ICONS` is `Record<LoadoutKey, LucideIcon>` — a missing key is a `tsc` error, checked, not assumed) | PASS (tsc) |
| S1 | Height chain: popup `h-[85vh]` (== shared `max-h-[85vh]`) → body `flex-1` → wrapper `h-full min-h-0` (Arsenal only) → tab root `h-full` → grid `min-h-0 flex-1`; list/rail/context columns own their overflow | PASS (code path; the no-scroll assert itself is operator visual V1) |
| S2 | Non-Arsenal tabs: wrapper stays auto-height, modal stays `max-w-lg` compact | PASS (conditional `cn`) |
| D1 | Doll renders zero `<text>` nodes; `<title>` tooltips + aria labels + 3 states + sub-hotspots + keyboard handling retained | PASS (code review of rewrite; vitest region gates unchanged) |
| D2 | Caption under doll = active region label + equipped item name (falls back to resource_name, 'empty' when bare) | PASS (code path) |
| F | Full suite **354/354** (353 + rail completeness) · `npm run build` clean · `tsc --noEmit` clean · lint = pre-existing `router.tsx` only | PASS |
| T | Registry: `T-068.10.8` slice shipped; **T-154** idea row (Rust/wgpu 3D doll — operator decision, no three.js); `active_slice` stays `T-068.11` | PASS (post-commit docs-sync) |

## Operator visual checklist (pause)

1. Hard-refresh the editor tab. Open a character slot → Attributes → Arsenal at ~1080p:
   **everything visible immediately — no scrolling anywhere**: top strip (compat badge +
   weight), rail, list, doll, context column, bottom bar (validation + Export), hint line.
2. Far-left rail: 14 slot icons, A3-style. Click the helmet icon → list swaps to helmets
   AND the doll helmet shows the active ring. Click the doll rifle → the rail Primary
   entry highlights (shared activeKey, two-way).
3. Equipped regions show the small dot on their rail icon; hover a rail icon → tooltip
   "Region — item name".
4. Doll: **no text on the model** — clean shapes only; the caption line under it reads
   the active region + item; hovering a part still shows its native tooltip.
5. Regression (.10.7 checklist still green): optic bump click → optics feed left + right
   quick-list; 6B2 + Lifchik both torso overlays; weight readout honest ("≥ … without
   weight data"); Export downloads v2 JSON; Faction Manager untouched.
