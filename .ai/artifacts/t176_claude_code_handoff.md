# T-176 — Claude Code handoff

**Start on `main` after T-175 @ `b90deac8`.** Do not touch `apps/mod/` or docs/registry.

## Operator word

> Contours much better. Forest highlighting doesn’t work properly — zoom all the way in then out seems to reset it. Asset drag from right menu still not visible (visible after placed). Zooming while panning stutters; zoom alone OK. Forest outline too much outside — needs to be a lot closer to where the trees actually are; openings painted as forest (see third screenshot — darker green = real trees). Names enough. Just the stuff mentioned.

## Screens

[`.ai/artifacts/t176_operator_screens/`](t176_operator_screens/) — `01` loose wash · `02` island overwash · `03` clearing inside hull.

## Hottest leads

| ID | Lead |
|----|------|
| A1 | Progressive forest push + memo / landcover vs TBDD gate mismatch |
| A2 | Soft landcover forest hulls (α≈38, Path B mega-regions) + TBDD iso=2 full-cell MS — tighten to canopy |
| B1 | Palette button owns pointer → map `pointermove` never runs `set_place_preview`; also `atlas_ready` early-return |
| B2 | Pan-stream settle + wheel mid-pan + forest/world fetch stacked |

## Authority

[`docs/platform/t176_forest_place_ghost_zoompan.md`](../../docs/platform/t176_forest_place_ghost_zoompan.md)

## Return

Tag **T-176** @ sha · inventory + verify · A/B PASS · Cursor list · ASK if blocked.
