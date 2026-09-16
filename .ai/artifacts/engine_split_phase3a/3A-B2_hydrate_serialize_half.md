# 3A-B2 — `state/hydrate.rs`: the decidable half crosses into `editing/persist/`

Agent 3 of 8. Your scope is `apps/website/frontend/src/editor/state/hydrate.rs` (1,159 LOC) and
nothing else. `state/persist.rs` was split by `3A-B1` — **do not revisit it.**

## You are inheriting a settled module

`3A-B1` landed `apps/website/map-engine/src/editing/persist/`, declared in `editing/mod.rs`:

| file | holds |
|---|---|
| `record_key.rs` | `ANONYMOUS_OWNER`, `owner_token_or_anonymous`, `owner_prefix`, `scoped_key`, `split_scoped_key` |
| `stored_blob.rs` | `restores_to_authored_content` |
| `record_read_retry.rs` | `backoff_before_attempt_ms` |
| `merge_policy.rs` | `apply_update_into_document`, `merge_before_write` |
| `slot_fingerprint.rs` | `slots_digest` |

**Reuse these.** `backup_key` is key arithmetic and `record_key.rs` already owns that shape;
anything you need to say about a blob's worth, `stored_blob.rs` already says. Do not write a
second spelling of either.

## The cut

**Crosses into `editing/persist/`:** the conflict classification and adoption policy — which of
server and local wins, what "the same authored content" means, and what a snapshot is worth.
Decidable without a browser, so it becomes natively `cargo test`-able.

**Stays in the frontend:** the transport and the session. `hydrate.rs` performs an **authed HTTP
GET** (`AuthStore`, `MissionDetail`) and drives leptos signals and DOM toasts. Gate rule 5 rejects
all of it engine-side, correctly.

The reload rule decides anything not named here: *the map engine owns state that survives a
reload; the frontend owns state that dies with the tab.*

### Engine-side, by inspection

| item | line | why |
|---|--:|---|
| `is_uuid` | 124 | pure |
| `enum Local`, `classify_local` | 453, 479 | the local-vs-server conflict verdict — the policy this file exists for |
| `server_slot_count` | 519 | pure over `serde_json::Value` |
| `same_authored_content` | 541 | pure comparison; defines what a conflict IS |
| `RowMeta`, `enum Adopt` | 549, 574 | the adoption vocabulary |
| `adopt_payload`, `apply_row` | 599, 635 | write through `DocHandle`, which is already engine-side (`editing/host.rs`) |
| `opt` | 653 | pure |
| `enum Snapshot`, `backup_key` | 673, 715 | snapshot vocabulary and key arithmetic — fold the key into `record_key.rs` |
| `snapshot_local` | 892 | over `DocHandle`, decidable |

`compile_payload` is already engine-side (`data::scenario::compile`), so a classification that
compares compiled payloads belongs next to it, not across the wall from it.

### Frontend-side, non-negotiably

`get_mission_measured` (149) and `hydrate_from_server` (233) — the authed GET and its
orchestration. `resolve_conflict_server` / `resolve_conflict_local` (397, 438) and `notify` (1069)
— signals and toasts. `LocalBackup`, `LiveEditor` and the thread-locals (723–747),
`set_live_editor` / `live_editor_is`, `register_mission_backup` — session state that dies with the
tab. `remember` / `recall` / `forget_snapshot` / `forget_owner` / `has_snapshot` /
`clear_local_backups` / `purge_local_documents` / `restore_*` — record-store transport.

Split the orchestrators where they are mixed: the **decision** crosses, the **I/O and the signal
write** stay. That is the same inversion `3A-B1` used for `merge_before_write` — the engine owns
the order, the frontend supplies the transport closure. Follow that precedent rather than
inventing a second pattern.

## Watch for

- `state/tab_lock.rs` stays frontend (phase 3B moves it to `shell/`). Its seam must survive.
- `state/save_status.rs` stays ungated so native `cargo test` can prove a silent `console.warn`
  cannot return. Do not break that path.
- **The scrub truncation trap** in `00_rules_every_agent_obeys.md`. Test-module declarations go at
  the BOTTOM of a frontend file.
- Law 7: everything you land engine-side is **born compliant** — under 500 LOC, tests in sibling
  files via `#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`.

## Done when

```
CARGO_TARGET_DIR=target-container cargo xtask verify engine-layers
rg 'web_sys|leptos|wasm_bindgen|idb|gloo' apps/website/map-engine/src/editing     # EMPTY
CARGO_TARGET_DIR=target-container cargo test -p website-map-engine --all-features
CARGO_TARGET_DIR=target-container cargo test -p website-frontend
```

Baseline: `verify engine-layers` PASS on all 8 rules; map-engine **1318** passed / 0 failed / 2
ignored as the floor (it rises as you add engine-side tests); frontend **1317** passed / 0 failed
and it must not fall.
