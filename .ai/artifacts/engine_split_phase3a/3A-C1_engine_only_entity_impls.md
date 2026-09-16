# 3A-C1 — the seven engine-only `entity/` implementations cross into the engine

Agent 4 of 8. Your scope is exactly the seven files below. `3A-C2` owns the three additive
host-state files (`cargo.rs`, `tactical_graphics.rs`, `entity/placement.rs`) — **do not touch
them.**

## Purely additive. Delete nothing.

`3A-C` adds; `3A-D` removes. Splitting it this way keeps every commit bisectable and the frontend
compiling throughout. The thirteen thin adapters stay exactly where they are; `3A-D3` deletes the
whole `state/operations/` directory at the end.

## The seven

No counterpart exists in `data/store/operations/`. This is real logic living in the wrong crate.
Every name is free engine-side — verified, no collisions.

| file | LOC |
|---|--:|
| `entity/layers.rs` | 264 |
| `entity/zone_draw.rs` | 264 |
| `entity/arming.rs` | 151 |
| `entity/triggers.rs` | 120 |
| `entity/layer_drag.rs` | 108 |
| `entity/refile.rs` | 85 |
| `entity/selection_index.rs` | 34 |

Source: `apps/website/frontend/src/editor/state/operations/entity/`.
Destination: `apps/website/map-engine/src/data/store/operations/entity/`.

### Two that need reading before you move them

`zone_draw.rs` carries a draw-session state machine — `Pending::Zone`,
`ZoneShape::{Circle,Polygon}`, vertex push/pop, and an `advance_zone_draw` returning a `Commit`
enum. It is session state, and it belongs engine-side. But `circle_from_clicks` lives in
`editor/panels/zones_panel.rs:1045`, is frontend-only, and has no engine twin — **leave it
there.**

`layer_drag.rs` makes **zero engine calls.** It is a `PENDING_LAYER_DRAG` thread-local state
machine that delegates to its siblings `layers::reparent_layer` and `refile_slot_to_layer`. It
moves because its siblings do, not because it calls anything.

## One implementation, not two

Do not leave the logic in both crates. The engine takes ownership; the frontend file becomes a
thin delegation to it, so there is exactly one source of truth while the frontend keeps compiling.
`3A-D3` deletes those shims with the rest of the directory.

## Follow the destination's conventions

`data/store/operations/entity/mod.rs` declares **private** modules with explicit re-exports:

```rust
mod markers;
pub use markers::{MarkerRow, marker_rows_of, mint_marker_id};
```

Match that. Do not write `pub mod`, and do not glob-re-export.

## Boundaries

- `data/store/mod.rs` pins its public surface with
  `#[cfg(test)] #[path = "tests/reexports.rs"] mod reexport_pins;`. **Update it as the surface
  grows** — that pin is the contract.
- **Gate rule 7** (the world/data wall) reads **zero in both directions** today and must stay
  zero. Nothing you add under `data/` may name `crate::{camera,frame,io,overlay,spatial,streaming,
  world}` or `website_graphics_engine`.
- Gate rule 5 covers `editing/` only, but anything you land in `data/store/` must still be
  browser-free to be testable at all: no `web_sys`, no `leptos`, no signals.
- Law 7: everything you create is **born compliant** — under 500 LOC, tests in sibling files via
  `#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`.

## Done when

The engine holds all seven, the frontend still compiles and passes unchanged, and:

```
CARGO_TARGET_DIR=target-container cargo xtask verify engine-layers
CARGO_TARGET_DIR=target-container cargo test -p website-map-engine --all-features
CARGO_TARGET_DIR=target-container cargo test -p website-frontend
```

Baseline: `verify engine-layers` PASS on all 8 rules; map-engine **1351** passed / 0 failed / 2
ignored as the floor (it rises as you add engine-side tests); frontend **1317** passed / 0 failed
and it must not fall.
