# High-Level CLI Operations (`ticket-engine/src/cli`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Implements high-level ticket operations invoked by `cargo xtask ticket ...` and `apps/ticketboard`.

Relocated from `xtask/src/cmds.rs` (which contained 1,343 lines of production code and 871 lines of inline tests). Decomposed into domain files (<350 LOC each) with tests extracted into `ticket-engine/tests/ticket_cmds_tests.rs`.

---

## Submodules

- **`query.rs`**: Read-only queries: `brief`, `show`, `list`, `next`, `milestone`, `prompt`, `get`.
- **`mutation.rs`**: Write operations: `add`, `add_child`, `remove`, `reorder`, `set_status`, `advance_slice`.
- **`shipping.rs`**: Shipping lifecycle: `ship`, `ship_opt`, `stamp_sha`, `done`.
- **`batch_run.rs`**: Batch operations: `run`, `plan_batch`, `ready_ids`, `config`.
