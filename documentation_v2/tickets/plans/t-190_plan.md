# T-190 — Plan

## Context
F-32 (verified): two tabs on one mission write the same IndexedDB key blind; the last five-second debounce wins and
the reload prompt misattributes the divergence to the server. The CRDT can merge and is never asked to.

## Approach
1. wasm test in `state/persist.rs`: two stores, one key, interleaved saves → last write wins on main; paste the red.
2. New `state/tab_lock.rs` (register in `state/mod.rs`): BroadcastChannel `tbd-mission-<id>` presence ping/ack;
   second tab gets a read-only banner signal; first tab's close releases it.
3. `persist.rs` save: read stored blob → `apply_update` into a scratch doc → merge → write; keep the debounce.
4. `state/hydrate.rs`: ConflictInfo gains {local_count, server_count, local_at, server_at}; `mission_editor.rs`
   conflict arm renders them and marks "Load server version" destructive (call site only — allowlisted SIZE-3 file).
5. Perturbation: skip the merge → convergence test red; restore, `touch`, green.
## Risks
- BroadcastChannel absent in some embedded browsers → fall back to a storage-event heartbeat; document it.
- T-937.4 lands first on persist.rs; rebase on its SaveStatus signal and report the merge.

## Verification
- `cargo xtask mk ci-local-leptos` · `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-190`
