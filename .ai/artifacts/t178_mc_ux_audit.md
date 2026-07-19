# Mission Creator UX/UI audit (post T-177) — 2026-07-19

Agent pass (read-only). Operator also filed forest tiling + guide gaps + OUTLINER label → **T-178**.

## Verdict

Usable Eden shell after T-172–T-177. Still half-authored: ORBAT Manager is browse-only until **T-071.1**. Several chrome controls look live but do nothing.

## Working

- Menus above docks; YouTube-ish guides; grab on palette leaves  
- Left = Editor Layers; top **ORBAT Manager** browse/select/Attributes  
- Place ghost + drag preview; boot overlay; 8 m canopy forest; sat progressive  

## Issues (audit)

| Sev | Problem | Fix direction |
|-----|---------|---------------|
| P0 | ORBAT Manager browse-only | **T-071.1** CRUD |
| P1 | Dual headers Outliner + Editor Layers | Drop “Outliner” → **T-178** |
| P1 | Guide gaps / not clickable | Continuous spine + click-to-toggle → **T-178** |
| P1 | Forest mass inconsistent / chunky tiles | Progressive chunk seams → **T-178** |
| P1 | Dead chrome (Vehicles/Markers, Ruler/LoS, stubs) | Hide or mark disabled |
| P2 | Ticket jargon in ORBAT footer; false “soon” rows | Copy cleanup |

## Recommended product order

1. **T-178** — forest load + guide polish + Outliner label (operator eye-pass)  
2. **T-071.1** — squad CRUD  
3. **T-071.2** → **T-068.11** — export order then compiler loadout  
