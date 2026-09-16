# 3A-D4 — rehome the survivors, delete the façade and the directory

Agent 9 of 9, and the last of phase 3A. Everything else is done: the thirteen thin adapters and
the ten `3A-C` shims are gone, and every document operation reaches the engine directly.

## What is left

```
state/operations.rs                       37   the façade
state/operations/batch.rs                 64   with_batch grouping + confirm_bulk
state/operations/context/{mod,registration,refresh,environment,attributes}.rs
                                         565   OPS_CTX
state/operations/README.md, context/README.md
```

The parent brief says two things that read as contradictory, and both are right: *"`state/operations/`
and `state/operations.rs` are gone"* and *"`batch.rs` and `context/` must NOT be deleted"*. The
resolution is that they **relocate**. They are the only frontend code in that tree with no engine
counterpart, and deleting them would be a behaviour change.

## Why they cannot cross the wall

`context/` holds `OPS_CTX`, a thread-local carrying **both** a live `DocHandle` and leptos
`RwSignal`s. The handle half is already engine-side in `editing/host.rs`. The **signals stay in
the frontend** — gate rule 5 would reject them, correctly, and phase 3B moves them to `shell/`.

`batch.rs` holds `with_batch` undo grouping and `confirm_bulk` / `confirm_bulk_n_step`, a
`window.confirm` passed *into* engine commands as a closure. **Keep that inversion.** The engine
must never own a `window.confirm`.

## Naming — Law 4 applies, and "operations" is the word that is going away

Pick homes under `apps/website/frontend/src/editor/state/`. Do **not** carry the name
`operations` forward: after this brief nothing in the frontend performs document operations, so a
module called that would misdescribe itself from the moment you land it.

Name each for what it actually holds — the installed document context and its signal mirrors; the
undo grouping and bulk confirmation. Both names must be unmistakable with zero project context.
Say in your report what you chose and why. Phase 3B will move both again into `shell/`, so
optimise for being obvious now, not for guessing 3B's final shape.

The two `README.md` files describe a directory that is ceasing to exist. Rewrite whatever survives
to describe the landed tree, or delete them — do not leave a README describing a deleted structure.

## Repointing

Roughly twenty files reach these through the façade as `editor_ops::` or
`crate::editor::state::operations`, including `state/{history,doc_host,entity_selection,mod}.rs`,
`state/armed_placement/*`, `canvas/{commands,tactical_graphics_authoring}.rs`,
`editor/{mod,eden_chrome,world_assets/mod}.rs`, `pages/operations/orbat_manager.rs`, and six
`mission_editor_tests/*`.

`agent 8` widened `OPS_CTX`, `OpsCtx` and its fields, `Pending` and `bump_doc_tick` to
`pub(crate)` so relocated host state could reach them. Once the façade is gone, tighten any
visibility that no longer needs to be that wide — but never at the cost of a call site.

`pages/operations/orbat_manager.rs` is unrouted editor code that **phase 3B** relocates to
`v2/apps/editor/ui/modals/`. Do not move it. Repoint its imports only.

## Watch for

- **`state/operations/` is `#![cfg(target_arch = "wasm32")]`** — `cargo test -p website-frontend`
  is native and never compiles it. The wasm32 and fmt checks below are not optional.
- Source-text pins `include_str!` these files. Repoint each at whatever the subject became.
  **Never weaken, skip, or delete a pin to make it pass.**
- `class_r_scrub::live_code()` blanks a frontend file from its FIRST `#[cfg(test)]` to EOF — any
  test module you add to a frontend file goes at the BOTTOM.
- `.coding-standards-allowlist.yaml:237` allowlists `…/state/operations/entity.rs`, a path that
  has not existed since before this work. **Phase 3C owns the allowlist — leave it alone**, but
  do not add new rows to it either.
- **Never `cd` before cargo.** Four agents have now leaked stray `target-container/` dirs into the
  source tree that way. Run cargo from the repo root; `( cd sub && … )` if you truly need another cwd.

## Done when

`apps/website/frontend/src/editor/state/operations.rs` and
`apps/website/frontend/src/editor/state/operations/` **do not exist**, every consumer reaches the
relocated modules, and:

```
CARGO_TARGET_DIR=target-container cargo xtask verify engine-layers
CARGO_TARGET_DIR=target-container cargo check --target wasm32-unknown-unknown -p website-frontend
CARGO_TARGET_DIR=target-container cargo fmt --all -- --check
CARGO_TARGET_DIR=target-container cargo test -p website-map-engine --all-features
CARGO_TARGET_DIR=target-container cargo test -p website-frontend
```

Baseline: `verify engine-layers` PASS on all 8 rules; wasm32 clean at **8 warnings** or fewer;
fmt clean; map-engine **1414** passed / 0 failed / 2 ignored; frontend **1317** passed / 0 failed.
Nothing may fall.

Phase 3A's closing gate — `cargo test -p xtask` and `cargo xtask mk ci-local-leptos` — is run by
the operator after you land, not by you.
