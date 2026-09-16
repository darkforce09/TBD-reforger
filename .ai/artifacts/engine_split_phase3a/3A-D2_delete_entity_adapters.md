# 3A-D2 — delete the seven `entity/` adapters, repoint their call sites

Agent 7 of 8. `3A-D1` deleted the five flat adapters and landed the hosted-command pattern you
will follow.

## Your seven — and the one file that must SURVIVE

| file | LOC |
|---|--:|
| `entity/selection.rs` | 237 |
| `entity/roster.rs` | 214 |
| `entity/vehicles.rs` | 190 |
| `entity/connections.rs` | 139 |
| `entity/markers.rs` | 124 |
| `entity/zones.rs` | 113 |
| `entity/comments.rs` | 108 |

**1,125 LOC. `entity/mod.rs` (214 LOC) is NOT deleted.** The parent brief lists it among the
thirteen adapters, and that is misleading: it is *also* the module root that declares the eight
`3A-C` shims which live until `3A-D3` deletes the whole directory.

`entity/mod.rs` declares fifteen modules. Seven are yours. **These eight must keep working:**
`layers`, `layer_drag`, `arming`, `refile`, `placement`, `zone_draw`, `triggers`,
`selection_index`.

So: delete your seven files, strip their `mod` lines and re-exports out of `entity/mod.rs`, and
leave the rest of that file standing. Note `selection.rs` (yours) and `selection_index.rs` (a
surviving `3A-C1` shim) are different files — do not confuse them.

## Follow D1's landing pattern

`3A-D1` put the adapter bodies in `apps/website/map-engine/src/editing/hosted_commands/`, reaching
`editing::host::{with_host, with_doc, selection_ids}`, `editing::batch::with_batch` and
`editing::history::after_local_edit` instead of `OPS_CTX` + `mission_history`. It is a sibling of
`editing/commands/` rather than part of it, because that module's invariant is "every function
here is pure over its arguments" and a hosted mutator is not.

Read commit `1226a4f24` and extend that module. Do not invent a second home.

## Deriving your call sites

The façade flattens everything — **no call site names its source module.** Consumers write
`editor_ops::<fn>(…)`, never `editor_ops::entity::<fn>`. 107 distinct symbols reach through it
frontend-wide; yours are the `pub fn`s and `pub use`s declared in your seven files. Derive that set
mechanically and repoint exactly those.

## One constraint D1 handed you

`DEFAULT_LAYER_ID` and `DEFAULT_LAYER_NAME` are defined at `entity/selection.rs:40` and `:43` —
**a file you are deleting.** They are imported by `entity/mod.rs:106` and consumed by
`entity/arming.rs:128-129`, a `3A-C1` shim that outlives you.

They are passed into the engine's `ensure_layer` as parameters, deliberately: the default folder's
name is host vocabulary, not document law. Keep that inversion. But find them a home that survives
this brief — and say which you chose and why. (`state/title_prefer.rs:185` also declares a
`DEFAULT_LAYER_ID`; it is an unrelated test constant, `"T570-LAYER"`. Leave it alone.)

## Watch for

- **`state/operations/` is `#![cfg(target_arch = "wasm32")]`.** `cargo test -p website-frontend`
  is native and **never compiles it**. Your primary artifact is invisible to that suite — the
  wasm32 check and the fmt check in the verification list below are not optional extras.
- Cross-boundary types are verified re-exports, not duplicate definitions: `MissionEnv`,
  `FactionRow` / `SquadRow`, `TacticalDraft`, `plan_reassign`, `DrawTarget`.
- `zones.rs` and `DrawTarget`: `zones_panel::DrawTarget` versus
  `data/store/operations/zones.rs:8 pub enum DrawTarget` is a re-export, already verified. Not a
  duplicate to reconcile.
- `pages/operations/orbat_manager.rs` is a heavy consumer (27 sites). It is unrouted editor code
  that **phase 3B** relocates — do not move it, just repoint its imports.
- Source-text pins `include_str!` files you delete. Repoint each pin at the engine seam its subject
  moved to. **Never weaken, skip, or delete a pin to make it pass.**
- `state/operations/` has zero inline `#[cfg(test)]` modules, so there is no test tail to rescue —
  but the call sites you repoint are covered by frontend tests that must stay green.
- **Never `cd` before cargo.** Two agents have now leaked stray `target-container/` dirs into the
  source tree that way, costing 6.2 GB and 2.1 GB.

## Done when

Your seven files are gone, `entity/mod.rs` still declares the eight surviving shims, every
repointed call site reaches the engine directly, and:

```
CARGO_TARGET_DIR=target-container cargo xtask verify engine-layers
CARGO_TARGET_DIR=target-container cargo check --target wasm32-unknown-unknown -p website-frontend
CARGO_TARGET_DIR=target-container cargo fmt --all -- --check
CARGO_TARGET_DIR=target-container cargo test -p website-map-engine --all-features
CARGO_TARGET_DIR=target-container cargo test -p website-frontend
```

Baseline: `verify engine-layers` PASS on all 8 rules; wasm32 clean at **19 warnings** (the same 19
by name — a floor, not a target); fmt clean; map-engine **1414** passed / 0 failed / 2 ignored;
frontend **1317** passed / 0 failed. Nothing may fall.
