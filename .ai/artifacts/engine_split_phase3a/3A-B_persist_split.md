# B — `state/{persist,hydrate}.rs` → `editing/persist/`

Both files are still wholly in the frontend:

```
apps/website/frontend/src/editor/state/persist.rs    1719
apps/website/frontend/src/editor/state/hydrate.rs    1159
```

## The cut

**Crosses into `map-engine/src/editing/persist/`:** serialization, deserialization, and the
merge / conflict policy. These are decidable without a browser and must be `cargo test`-able.

**Stays in the frontend:** the transports. `persist.rs` drives IndexedDB through the `idb` crate;
`hydrate.rs` performs an authed HTTP GET. Neither belongs in the engine, and gate rule 5 will
reject them if you try.

The reload rule decides anything not named here: *the map engine owns state that survives a
reload; the frontend owns state that dies with the tab.*

## There is an unfinished start on disk

```
apps/website/map-engine/src/editing/persist/blob.rs        96 LOC
apps/website/map-engine/src/editing/persist/record_key.rs  43 LOC
```

Uncommitted, and **not declared in `editing/mod.rs`**. The previous agent was interrupted
mid-thought writing them. Keep, rewrite or delete as you judge — but do not leave them dangling
and undeclared.

## Watch for

- `state/persist.rs` reports every save `Err` into `state/save_status.rs`, which is deliberately
  ungated so native `cargo test` can prove a silent `console.warn` cannot return. Do not break
  that path.
- `state/tab_lock.rs` holds the save-decision policy `persist.rs` obeys. `tab_lock` stays in the
  frontend (3B moves it to `shell/`), so the policy seam has to survive the split intact.
- Law 7: 1719 and 1159 both far exceed the 500-line ceiling. Whatever you land engine-side is born
  compliant — split it as you go, tests in sibling files.

## Done when

```
CARGO_TARGET_DIR=target-container cargo xtask verify engine-layers
rg 'web_sys|leptos|wasm_bindgen' apps/website/map-engine/src/editing     # EMPTY
CARGO_TARGET_DIR=target-container cargo test -p website-map-engine --all-features
CARGO_TARGET_DIR=target-container cargo test -p website-frontend
```
