# T-177 — Claude Code handoff

**Start on `main` after T-176 @ `a5940fad`.** Do not touch `apps/mod/` or docs/registry.

## Operator word

- YouTube-style comment **lines** for tree hierarchy.  
- Grab cursor (or clear affordance) when hovering draggable assets.  
- Left Outliner: **remove ORBAT** (bad split with Editor Layers); keep Editor Layers.  
- Top strip: **ORBAT Manager** button (e.g. by Environment) → **modal** (= **T-071.0**).  
- Environment (etc.) menu opens **behind** the left panel — fix stacking.  
- T-176 forest: significantly better (leave alone unless regress).

## Authority

1. [`docs/platform/t177_mc_chrome_orbat_cutover.md`](../../docs/platform/t177_mc_chrome_orbat_cutover.md)  
2. [`t071_orbat_manager_program.md`](../../docs/specs/Mission_Creator_Architecture/t071_orbat_manager_program.md) — **T-071.0 only**  
3. Screens in [`t177_operator_screens/`](t177_operator_screens/)

## Leads

| ID | Lead |
|----|------|
| A1 | `guide_spans` → stem + elbow |
| A2 | palette leaf `cursor-grab` |
| A3 | menu `z-50` under dock stacking — portal / raise |
| B1–B2 | remove ORBAT dock block; ORBAT Manager → modal shell |

## Return

Tag **T-177** · note T-071.1+ remaining · Cursor list.
