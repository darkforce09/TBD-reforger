# 3A-B1 — `state/persist.rs`: the decidable half crosses into `editing/persist/`

Agent 2 of 8. Your scope is `apps/website/frontend/src/editor/state/persist.rs` (1,719 LOC) and
nothing else. `state/hydrate.rs` belongs to `3A-B2` — do not touch it.

## The cut

**Crosses into `apps/website/map-engine/src/editing/persist/`:** serialization, deserialization,
key arithmetic, and the merge / conflict policy. Everything decidable without a browser, so it
becomes natively `cargo test`-able.

**Stays in the frontend:** the transport. `persist.rs` drives IndexedDB through the `idb` crate,
and the whole debounce / timer / flight-guard machine around it is browser lifecycle. Gate rule 5
rejects any of it engine-side, correctly.

The reload rule decides anything not named here: *the map engine owns state that survives a
reload; the frontend owns state that dies with the tab.*

### Engine-side, by inspection

| item | line | why |
|---|--:|---|
| `owner_token`, `owner_prefix`, `scoped_key`, `split_scoped` | 217–246 | pure string arithmetic over an owner token and a logical key |
| `unreadable_backoff_ms` | 338 | pure |
| `update_client_count` | 709 | reads a Yjs v1 update's leading var-int |
| `blob_has_content` | 765 | pure over bytes |
| `merge_stored`, `merge_before_write` | 923, 950 | the merge / conflict policy |
| `slots_digest` | 1355 | pure over `MissionDocCore` |

`merge_before_write` is `async` and takes a `&GetBytes` closure. Keep that inversion: the engine
takes the closure, the frontend supplies the transport-backed one. Do not drag `idb` across to
make it concrete.

### Frontend-side, non-negotiably

Everything naming `idb::` — `open_db`, `put_raw`, `read_raw`, `get_raw`, `has_raw`, `delete_raw`,
`all_keys`, `save_state*`, `load_state`, `clear_state`, `purge_owner`, `evict_foreign_records`,
`orphan_keys`, `adopt_orphans`, `retry_unreadable_keys` — plus the debounce machine
(`PendingSave`, `TimerEntry`, `SaveFlightGuard`, `save_state_debounced`, `arm_debounced`,
`install_pending`, `flush_state`, `register_flush_on_hide`, `clear_timer`, `run_save`,
`spawn_promise`, the wasm `now_ms`, `register_tab_sync`, `pull_peer_record`,
`schedule_edit_persist`).

## There is an unfinished start on disk — you own the decision

```
apps/website/map-engine/src/editing/persist/blob.rs         96 LOC
apps/website/map-engine/src/editing/persist/record_key.rs   43 LOC
apps/website/map-engine/src/editing/persist/tests/          EMPTY
```

Untracked, and **`persist` is not declared in `editing/mod.rs`**. A previous agent was interrupted
mid-thought writing them; `record_key.rs` tails off mid doc-comment, so treat it as unfinished.

They cover exactly the right ground — `blob.rs` holds the backoff and the client-count/has-content
pair, `record_key.rs` the key arithmetic and an `ANON_OWNER` namespace. Keep, rewrite or delete as
you judge, but **nothing may be left dangling and undeclared** when you finish. `3A-B2` inherits
this module, so leave it settled.

## Watch for

- `state/save_status.rs` (450 LOC) is deliberately ungated so native `cargo test` can prove a
  silent `console.warn` cannot return. `persist.rs` reports **every** save `Err` into it. Do not
  break that path.
- `state/tab_lock.rs` (993 LOC) holds the save-decision policy `persist.rs` obeys. `tab_lock`
  **stays in the frontend** — phase 3B moves it to `shell/` — so the policy seam has to survive
  the split intact.
- `now_ms` has a `#[cfg(target_arch = "wasm32")]` / `not(...)` pair. The engine half must be the
  browser-free one.
- **The scrub truncation trap** in `00_rules_every_agent_obeys.md`. If you add a test module to a
  frontend file, declare it at the BOTTOM.
- Law 7: 1,719 LOC is far over the 500-line ceiling. Whatever you land engine-side is **born
  compliant** — split it as you go, tests in sibling files via
  `#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`.

## Done when

```
CARGO_TARGET_DIR=target-container cargo xtask verify engine-layers
rg 'web_sys|leptos|wasm_bindgen|idb' apps/website/map-engine/src/editing     # EMPTY
CARGO_TARGET_DIR=target-container cargo test -p website-map-engine --all-features
CARGO_TARGET_DIR=target-container cargo test -p website-frontend
```

Baseline: `verify engine-layers` PASS on all 8 rules; map-engine 1290 passed / 0 failed / 2
ignored; frontend 1317 passed / 0 failed. Map-engine rises as you add engine-side tests; frontend
must not fall.
