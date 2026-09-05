# T-295 — Plan

## Context
The editor persists yrs to IndexedDB only; the API has no websocket (axum without `ws`) and no edit-since-load
check on POST /versions. T-190 adds local CRDT merging first; this ticket adds the transport and the server check.

## Approach
1. `apps/website/api/Cargo.toml`: axum `ws` feature (no new crates beyond yrs already in the workspace).
2. New `api/src/realtime/mod.rs` + `yrs_sync.rs` (register `pub mod realtime;` in `lib.rs`): room per mission id,
   broadcast of update/awareness frames, cookie auth; route in `app.rs` beside :751.
3. `api/src/app.rs` versions handler path: `base_version_id` param; mismatch → 409 with the head id.
4. New `frontend/src/editor/state/collab_sync.rs` (register in `state/mod.rs`): ws client, apply_update on receive,
   send on local txn; `persist.rs` keeps its debounce and passes base_version_id on save.
5. Tests: two-client convergence (integration), 409 on stale base; perturbation: drop the base check → red.

## Risks
- Awareness storms on large ORBATs; throttle to 10 Hz. Reconnect with backoff; offline still works via IndexedDB.

## Verification
- `cargo test -p website-api realtime` · leptos gates · `cargo xtask platform wave gate --slice T-295`
