# T-090.9 — World-object interaction (hover, inspect, filter, legend)

## Context

Static world objects render as pixels today; mission makers cannot interrogate
them. This slice makes them read-only context: hover tooltip, click-to-inspect
panel with "Ask AI about this object", taxonomy filter/search, a legend, and a
Z-trust badge — without ever moving them (edits stay Workbench-only, N7 locked)
and without re-enabling Deck GPU picking.

## Approach

Build the interaction layer in `apps/website/frontend/src/world_assets` +
`apps/website/frontend/src/mission_editor.rs`: CPU-side picking over the worker's
spatial index per the picking authority, tooltip + read-only inspect panel fed by
the T-090.7 resolver, filter/legend driven by the taxonomy, and the Z-trust badge
from the T-090.4/.6 audit flags. Editor FPS budget respected — no per-frame
full-catalog scans.

## Risks

Depends on T-090.5 render and the T-090.7 resolver being live; picking at 1M
objects can wreck frame time if the spatial index is bypassed; scope creep toward
editing terrain props is explicitly forbidden (move/delete/edit remain
Workbench-only). AI panel wiring must degrade gracefully when the AI surface is
absent.

## Verification

Hover shows the tooltip, click opens read-only inspect with the resolved fields,
filter/search and legend act on taxonomy classes, the Z-trust badge reflects
audit flags; no mutation affordance exists on world objects; editor FPS stays
within the HUD budget
(spec: `docs/specs/Mission_Creator_Architecture/t090_9_world_object_interaction.md`).
