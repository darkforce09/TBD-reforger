# T-702 — Plan

## Context

3DEN-MISC-001 E11: every mission needs a whole-terrain play-area zone and authors draw a 12.8 km polygon by hand. The `begin_zone_draw` path (`zones_panel.rs:40-42`, `operations/entity.rs:3453`) is the only way today; the 2026-08-02 attempt is recoverable at `salvage/t853-dropped/T-702` (4e4eefd7).

## Approach

1. `operations/entity.rs`: `terrain_rect_ring(terrain_bounds)` + `terrain_rect_is_authorable` (native tests, golden vs both terrains) and `add_whole_terrain_zone()` — one undo step, labelled "Play Area", selects the zone so Attributes open.
2. `zones_panel.rs`: one button calling it; reuse the salvage diff where it matches.

## Risks

- `terrain_bounds` must be the same source the flatten/export path reads; assert it in the test.

## Verification

- `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-702`
