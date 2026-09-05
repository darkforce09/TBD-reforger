# T-090.9 — World-object interaction (hover, inspect, filter, legend)

## Context

Static world objects render as pixels today; mission makers cannot interrogate
them. This slice makes them read-only context: hover tooltip, click-to-inspect
panel with "Ask AI about this object", taxonomy filter/search, a legend, and a
Z-trust badge — without ever moving them (edits stay Workbench-only, N7 locked)
and without re-enabling Deck GPU picking.

## Approach

Build the interaction layer in `apps/website/frontend/src/editor/world_assets` (new `pick.rs` + `inspect.rs`, registered in `mod.rs`) +
`apps/website/frontend/src/editor/mission_editor.rs` (mount only — new code goes in the new files): CPU-side picking over the worker's
spatial index per the picking authority, tooltip + read-only inspect panel fed by
the T-090.7 resolver, filter/legend driven by the taxonomy, and the Z-trust badge
from the T-090.4/.6 audit flags. Editor FPS budget respected — no per-frame
full-catalog scans.

## Risks

T-090.5 is deferred — the live render is the T-090.12 `world_host.rs` lane; the picking authority is `WorldSpatialIndex` (`crates/map-engine-core/src/world/index.rs:35`). Packs after T-090.7 (shared `mod.rs`); picking at 1M
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
