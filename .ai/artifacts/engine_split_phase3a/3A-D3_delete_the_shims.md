# 3A-D3 — delete the ten `3A-C` shims, rehome what is genuinely host state

Agent 8 of 9. `3A-D1` and `3A-D2` deleted the thirteen thin adapters. What is left in
`state/operations/` is the residue of `3A-C`: ten files that delegate to the engine, plus the
survivors `3A-D4` handles.

## Your ten — 1,808 LOC

| file | LOC | | file | LOC |
|---|--:|---|---|--:|
| `entity/zone_draw.rs` | 266 | | `entity/layer_drag.rs` | 124 |
| `tactical_graphics.rs` | 236 | | `entity/triggers.rs` | 123 |
| `entity/layers.rs` | 207 | | `entity/mod.rs` | 122 |
| `entity/arming.rs` | 193 | | `entity/refile.rs` | 103 |
| `cargo.rs` | 160 | | `entity/selection_index.rs` | 140 |
| `entity/placement.rs` | 134 | | | |

**Do NOT touch `batch.rs` (64), `context/` (565 across 5 files), or the façade
`state/operations.rs` (67).** Those are `3A-D4`'s, and they must still work when you finish.

## These are not pure delegation — read before you delete

`3A-D2` deliberately moved genuine host state *into* several of these shims, because it is host
state and belongs frontend-side. Deleting it would be a behaviour change. At minimum:

- `entity/selection_index.rs` — `select_slot`, `select_all_in_view`, `center_on_selection`. These
  move the camera and rebind the renderer's tint, then run the leptos selection mirror.
- `entity/arming.rs` — `begin_place_marker`, `armed_marker_icon`, `ensure_active_layer`, and the
  `Pending` arm every palette place shares.
- `entity/zone_draw.rs` — `add_whole_terrain_zone`, built entirely from the Zones panel's ring,
  label and schema type.
- `tactical_graphics.rs` / `cargo.rs` — the `OPS_CTX` borrow chain, signal reads/writes,
  `bump_doc_tick()`, and the `after_local_edit()` tail.

**Sort every item in all ten files into exactly one of two piles:**

1. **Engine-bound delegation** — the call site repoints straight at the engine (`engine_ops::`,
   the convention `3A-D1` and `3A-D2` established) and the shim body dies.
2. **Genuine host state** — camera, renderer tint, leptos signals, `Pending` arms, panel
   vocabulary. It **relocates** to the frontend module that owns that concern. It does not die,
   and it does not go to the engine.

For pile 2, follow `3A-D2`'s precedent: it put `DEFAULT_LAYER_ID` / `DEFAULT_LAYER_NAME` in
`panels/outliner.rs` beside `UNFILED_ID` and the other tree vocabulary, because that module *is*
the Layers tree. Host state goes to the panel or module that owns the concern — never to a new
catch-all bucket. Name the home in your report for each thing you relocate.

## Keep the closure inversions

`ensure_active_layer`, the zone-type and marker-icon schema predicates, `confirm_bulk`, and
`record_placed` all cross into the engine as closures so the engine never owns host vocabulary or
a `window.confirm`. Preserve every one. Never make the engine reach back for host state.

## One duplication to settle

`editing/host.rs:34` defines `pub enum Pending` and `:58` `pub struct ZoneDraft`. Both are
**dormant** — nothing in the map engine references them, and the frontend reaches `editing::host`
only for `install()` (`mission_editor.rs:1941`). The live `ZoneDraft` is
`data/store/operations/entity/zone_draw.rs:17`, so the crate carries two definitions of that type.

`editing/` may depend on `data/`; `data/` may never name `editing/`. So there is exactly one fix
direction: delete `host.rs`'s copies and use the `data/` ones. Do it while you are wiring
`zone_draw`'s call sites, not as an afterthought.

## Watch for

- **`state/operations/` is `#![cfg(target_arch = "wasm32")]`.** `cargo test -p website-frontend`
  is native and **never compiles it**. The wasm32 and fmt checks below are not optional extras.
- Known pre-existing dead code your deletions will clear: `arming::mint_id` and five orphaned
  imports in `entity/mod.rs` — six of the current 18 wasm32 warnings.
- Source-text pins `include_str!` files you delete. Repoint each at the engine seam its subject
  moved to. **Never weaken, skip, or delete a pin to make it pass.**
- `class_r_scrub::live_code()` blanks a frontend file from its FIRST `#[cfg(test)]` to EOF — any
  test module you add to a frontend file goes at the BOTTOM.
- **Never `cd` before cargo.** Three agents have now leaked stray `target-container/` dirs into
  the source tree that way, costing 6.2 GB, 2.1 GB and 3.2 GB.

## Done when

Your ten files are gone, every engine-bound call site reaches the engine directly, every piece of
host state has a named home outside `state/operations/`, `editing/host.rs` no longer duplicates
`Pending`/`ZoneDraft`, and `batch.rs` + `context/` + the façade still compile:

```
CARGO_TARGET_DIR=target-container cargo xtask verify engine-layers
CARGO_TARGET_DIR=target-container cargo check --target wasm32-unknown-unknown -p website-frontend
CARGO_TARGET_DIR=target-container cargo fmt --all -- --check
CARGO_TARGET_DIR=target-container cargo test -p website-map-engine --all-features
CARGO_TARGET_DIR=target-container cargo test -p website-frontend
```

Baseline: `verify engine-layers` PASS on all 8 rules; wasm32 clean at **18 warnings** or fewer;
fmt clean; map-engine **1414** passed / 0 failed / 2 ignored; frontend **1317** passed / 0 failed.
Nothing may fall.
