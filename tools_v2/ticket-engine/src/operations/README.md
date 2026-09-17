# Transactional Operations (`ticket-engine/src/operations`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Pure, clock-injected mutation engine providing ACID guarantees over the ticket registry.

Decomposed from the 2,477-line `ops.rs` file into focused submodules (<450 LOC each).

---

## Submodules

- **`status.rs`**: Lifecycle state transitions (`mark_ready`, `ship`, `cancel`, `defer`).
- **`edit.rs`**: Adding new tickets (`add`, `add_child`), removing tickets, reordering tickets in the queue.
- **`slice.rs`**: Worktree slice advancement, slice completion, and commit SHA stamping (`stamp_sha`).
- **`validation.rs`**: Pre-commit post-image verification ensuring invariants are satisfied before any write lands on disk.
