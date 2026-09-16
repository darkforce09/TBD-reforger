# 3A-D1 — delete the five flat adapters, repoint their call sites

Agent 6 of 8, and the first of the three that REMOVE things. `3A-C` finished: everything these
adapters adapt now lives engine-side.

## Your five

| file | LOC | `pub fn`s |
|---|--:|--:|
| `attrs.rs` | 247 | 11 |
| `transform.rs` | 163 | 5 |
| `compositions.rs` | 124 | 8 |
| `reassign.rs` | 103 | 4 |
| `slot_ids.rs` | 7 | 1 (a bare `pub use`) |

Under `apps/website/frontend/src/editor/state/operations/`. The eight `entity/` adapters belong to
`3A-D2`; the façade and the `3A-C` shims belong to `3A-D3`. **Touch neither.**

## How the call sites actually look — read this before planning

The façade flattens everything. **No call site names the source module.** Consumers write
`editor_ops::attrs_update_position(…)` or `ops::reassign_rows()`, never
`editor_ops::attrs::update_position`. There are **107 distinct symbols** reached that way across
the frontend, and yours are a subset.

So derive your set mechanically: the `pub fn`s (and the `pub use`) declared in your five files ARE
your symbol list. Repoint exactly those call sites and no others.

Every adapter has the identical shape, no logic of its own:

```rust
pub fn X(args) -> R {
    let r = with_batch("label", || OPS_CTX.with(|c| { /* borrow chain */
        website_map_engine::data::store::operations::<mod>::X(core, args)
    }));
    if changed { mission_history::after_local_edit(); }
    r
}
```

What they supplied now exists engine-side: `editing/host.rs` (the doc handle), `editing/batch.rs`
(`with_batch` undo grouping) and `editing/history/` (`after_local_edit`). Call sites move to those.

## Three constraints that will bite you

**1 · `align_selection` — which one wins is deliberate, and it is not `transform`'s.**

Proven, not guessed. `batch.rs:25` defines `align_selection(edge)` as
`with_batch("align", || transform::align_selection(edge))`. `transform.rs:91` defines its own,
unwrapped. The façade re-exports `batch::*` wholesale but takes transform through an **explicit
list that omits `align_selection`**:

```rust
pub use transform::{
    apply_pattern_to_selection, orient_selection, rotate_selection_to_face, space_selection,
};
```

So the undo-grouped one wins today. **`batch.rs` stays frontend** (phase 3B moves it), so when you
delete `transform.rs`, repoint `batch::align_selection`'s inner call at the engine and keep the
`with_batch("align", …)` wrapper exactly where it is. Losing that wrapper silently turns one
undoable align into N undo steps — a behaviour change, and the contract is zero.

**2 · `confirm_bulk` must survive `transform.rs`'s deletion.**

`transform.rs:43` defines `confirm_bulk` (a `window.confirm`) and `:55` `confirm_bulk_n_step`.
That closure is passed **into** the engine function — keep the inversion: the engine keeps taking
a confirm closure, the frontend keeps supplying the DOM one. Never make the engine own a
`window.confirm`.

`confirm_bulk_n_step` is consumed from outside your five, at
`state/operations/cargo.rs:131` and `:145` — a `3A-C2` shim that lives until `3A-D3`. So it needs a
home that outlives this brief. `batch.rs` and `context/` both stay frontend through all of 3A;
put it in whichever reads more honestly, and say which in your report.

**3 · The façade must keep compiling.**

`state/operations.rs` re-exports your five. As each goes, its `pub use` line goes with it. The
frontend must build and pass at every commit you make — `3A-D2` and `3A-D3` run after you.

## Watch for

- `state/operations/` has **zero** inline `#[cfg(test)]` modules, so there is no test tail to
  rescue. But the call sites you repoint are covered by frontend tests that must stay green.
- Cross-boundary types are verified re-exports, not duplicate definitions: `MissionEnv`,
  `FactionRow` / `SquadRow`, `TacticalDraft`, `plan_reassign`, `DrawTarget`.
- `pages/operations/orbat_manager.rs` is a consumer. It is unrouted editor code that **phase 3B**
  relocates — do not move it, just repoint its imports if it touches your symbols.
- Source-text pins may `include_str!` the files you delete. Repoint a pin at the engine seam its
  subject moved to; never weaken or delete one to make it pass.
- **Never `cd` before cargo.** An earlier agent leaked five stray `target-container/` dirs that
  way, costing 6.2 GB.

## Done when

Your five files are gone, every one of their call sites reaches the engine directly, and:

```
CARGO_TARGET_DIR=target-container cargo xtask verify engine-layers
CARGO_TARGET_DIR=target-container cargo test -p website-map-engine --all-features
CARGO_TARGET_DIR=target-container cargo test -p website-frontend
```

Baseline: `verify engine-layers` PASS on all 8 rules; map-engine **1414** passed / 0 failed / 2
ignored; frontend **1317** passed / 0 failed. Neither may fall.
