# C — move the operations that belong in the engine into `data/store/operations/`

**Purely additive. Delete nothing.** C adds; [3A-D](3A-D_operations_delete_adapters.md) removes.
Splitting it this way keeps each commit bisectable and keeps the frontend compiling throughout.

Source: `apps/website/frontend/src/editor/state/operations/` — 29 files, 4,320 LOC, untouched so
far. The full per-file audit is in [`01_original_full_3a_brief.md`](01_original_full_3a_brief.md)
section B. Your half is the 10 files below.

## C1 · Seven engine-only implementations, zero DOM

No counterpart exists in `data/store/operations/`. These are real logic living in the wrong crate.
They become new files there.

| file | LOC |
|---|--:|
| `entity/layers.rs` | 264 |
| `entity/zone_draw.rs` | 264 |
| `entity/arming.rs` | 151 |
| `entity/triggers.rs` | 120 |
| `entity/layer_drag.rs` | 108 |
| `entity/refile.rs` | 85 |
| `entity/selection_index.rs` | 34 |

`entity/zone_draw.rs` carries a draw-session state machine (`Pending::Zone`,
`ZoneShape::{Circle,Polygon}`, vertex push/pop, an `advance_zone_draw` → `Commit` enum). Note that
`circle_from_clicks` lives in `editor/panels/zones_panel.rs:1045` and is frontend-only with no
engine twin — leave it there.

`entity/layer_drag.rs` makes **zero engine calls**; it is a `PENDING_LAYER_DRAG` state machine
that delegates to its siblings `layers::reparent_layer` / `refile_slot_to_layer`.

## C2 · Three additive host-state files

These DO have same-named engine counterparts, but they are not duplicates — each adds
host/session state the engine does not model. **Move that state engine-side. Do not discard it,
and do not "reconcile" it away.**

| file | LOC | the additive part |
|---|--:|---|
| `cargo.rs` | 202 | `CARGO_DEFAULTS` / `LOADOUT_BUFFER` / `APPLY_SEED` thread-locals |
| `tactical_graphics.rs` | 299 | the `TG_STATE` draw machine — selected / draft / `VertexDrag` |
| `entity/placement.rs` | 194 | placement policy: crew toggle, active-layer resolution, cargo seeding |

`tactical_graphics.rs` is the lopsided one: only 4 of its ~20 public fns forward to the engine;
the rest are the session state machine.

## Boundaries

- `data/store/mod.rs` pins its public surface with
  `#[cfg(test)] #[path = "tests/reexports.rs"] mod reexport_pins;` — update it as the surface grows.
- Gate rule 7 (the world/data wall) reads **zero in both directions** today and must stay zero.
  Nothing you add under `data/` may name `crate::{camera,frame,io,overlay,spatial,streaming,world}`
  or `website_graphics_engine`.
- Gate rule 5 covers `editing/` only, but anything you put in `data/store/` must still be
  browser-free to be testable — no `web_sys`, no signals.
- Law 7: everything you create is born compliant. Tests in sibling files.

## Done when

The engine holds all ten, the frontend still compiles and passes unchanged (its adapters are
still in place — D removes them), and:

```
CARGO_TARGET_DIR=target-container cargo xtask verify engine-layers
CARGO_TARGET_DIR=target-container cargo test -p website-map-engine --all-features
CARGO_TARGET_DIR=target-container cargo test -p website-frontend
```
