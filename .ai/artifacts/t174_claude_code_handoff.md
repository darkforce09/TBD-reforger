# T-174 — Claude Code handoff

**Start on `main` after T-173.** Do not touch `apps/mod/` or docs/registry.

## Operator word

> “the sat map is way too low res, also I don't want the tree heat map (the green glow) Just make a new ticket… Also there's a long line that goes from top to bottom on both the left and right panel. Otherwise generally it is pretty good.”

## Authority

1. [`docs/platform/t174_mc_sat_heatmap_guides.md`](../../docs/platform/t174_mc_sat_heatmap_guides.md)  
2. Screens in [`.ai/artifacts/t174_operator_screens/`](t174_operator_screens/)

## Leads

| S | Fix direction |
|---|----------------|
| S1 | `satellite.rs` — stop treating localhost as forever-preview; full TBDS (or preview→full progressive) on `make leptos` |
| S2 | Density-heatmap glow **removed** (operator: “Remove the heatmap…”); LOD rung kept |
| S3 | `guide_spans` — stop full-dock-height rails; clip to tree rows only |

## Return

Tag **T-174** @ sha · inventory + verify · S1–S3 PASS · Cursor list if any.
