# D — delete the adapters and unwind the façade

**Requires [3A-C](3A-C_operations_move_engine_side.md) to have landed.** The adapters cannot go until
what they adapt lives engine-side.

This is the riskiest brief in Phase 3A, not because the logic is hard but because the blast radius
is wide: roughly **500 call sites** across the frontend reach these symbols through one façade.

## D1 · Delete 13 thin adapters

Every one has the identical shape — no logic of its own:

```rust
pub fn X(args) -> R {
    let r = with_batch("label", || OPS_CTX.with(|c| { /* borrow chain */
        website_map_engine::data::store::operations::<mod>::X(core, args)
    }));
    if changed { mission_history::after_local_edit(); }
    r
}
```

`attrs`, `compositions`, `reassign`, `slot_ids`, `transform`, and
`entity/{mod, comments, connections, markers, roster, selection, vehicles, zones}`.

Everything they supplied now exists engine-side: `editing/host.rs` (the doc handle),
`editing/batch.rs` (`with_batch` undo grouping) and `editing/history/` (`after_local_edit`) all
landed in the first agent's commits. Call sites move to those.

`slot_ids.rs` is 7 lines — a bare `pub use`. `transform.rs` passes a `confirm_bulk` closure
(a `window.confirm`) **into** the engine function: keep that inversion. The engine keeps taking a
confirm closure; the frontend keeps supplying the DOM one.

## D2 · Unwind the façade

`apps/website/frontend/src/editor/state/operations.rs` is 98 lines, `#![cfg(target_arch = "wasm32")]`,
and is a pure re-export surface: eight glob `pub use X::*` lines plus a single
`pub use entity::{...}` naming **112 symbols**.

Consumers reach it as `use crate::editor::state::operations as editor_ops;` from
`state/history.rs:28`, `mission_editor.rs:42`, `canvas/{commands,gestures,overlays}.rs`,
`panels/{dock_left,dock_right}.rs`, plus ~15 function-local `as ops;` aliases in
`panels/{outliner_drag,zones_panel,dock_right,attributes_modal,top_strip}.rs`.

Note one deliberate subtlety before you flatten anything: `align_selection` is taken from
`batch::*` (the `with_batch`-wrapped version), **not** from `transform`. Preserve which one wins.

## D3 · What must NOT be deleted

`batch.rs` and `context/` have no engine counterpart. `context/` is 5 files / 567 LOC and holds
`OPS_CTX` — a thread-local carrying **both** a live `DocHandle` and leptos `RwSignal`s. The handle
half is already engine-side; the **signals and every `open_*` / `close_*` signal writer stay in the
frontend**, and 3B moves them. Do not drag them across: gate rule 5 will reject them, correctly.

## Watch for

- `state/operations/` has **zero** inline `#[cfg(test)]` modules, so there is no test tail to
  rescue here — but the call sites you repoint are covered by frontend tests that must stay green.
- `pages/operations/orbat_manager.rs` is one of the consumers. It is unrouted editor code that 3B
  relocates to `v2/apps/editor/ui/modals/` — do not move it here, just repoint its imports.
- Cross-boundary types are already re-exports, verified: `MissionEnv`, `FactionRow`/`SquadRow`,
  `TacticalDraft`, `plan_reassign`, `DrawTarget`. None is a duplicate definition.

## Done when

`apps/website/frontend/src/editor/state/operations/` and `state/operations.rs` are gone, every
call site reaches the engine directly, and:

```
CARGO_TARGET_DIR=target-container cargo xtask verify engine-layers
rg 'web_sys|leptos|wasm_bindgen' apps/website/map-engine/src/editing     # EMPTY
CARGO_TARGET_DIR=target-container cargo test -p website-map-engine --all-features
CARGO_TARGET_DIR=target-container cargo test -p website-frontend
CARGO_TARGET_DIR=target-container cargo test -p xtask
CARGO_TARGET_DIR=target-container cargo xtask mk ci-local-leptos
```

That last command is Phase 3A's closing gate: fmt, wasm32 clippy, native test, and a trunk
release build.

---

## Two things `3A-C1` uncovered that `3A-D` owns

**1. `editing/host.rs` carries a second, dormant definition of `Pending` and `ZoneDraft`.**

`editing/host.rs:34` defines `pub enum Pending` and `:58` defines `pub struct ZoneDraft`. Neither
is referenced anywhere in the map engine outside that file — verified by grep — and the frontend
reaches `editing::host` only for `install()` (`mission_editor.rs:1941`). They are dormant state
waiting to be wired.

`3A-C1` landed the live `ZoneDraft` at `data/store/operations/entity/zone_draw.rs:17`, so the
crate now carries **two definitions of that type**. `editing/` may depend on `data/`; `data/` may
never name `editing/`. So the fix has one direction: delete `host.rs`'s copies and use the `data/`
ones. Do it as part of wiring the adapters' call sites, not as a separate afterthought.

**2. `DEFAULT_LAYER_ID` / `DEFAULT_LAYER_NAME` are still frontend constants.**

They live in frontend `entity/selection.rs` and are passed into the engine's `ensure_layer` as
parameters — deliberate, because the default folder's name is host vocabulary rather than document
law. But `state/operations/` is deleted by `3A-D3`, so decide where they land: the arsenal/UI
vocabulary they belong to, not a deleted directory.
