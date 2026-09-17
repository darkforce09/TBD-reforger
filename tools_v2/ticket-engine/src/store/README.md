# Ticket Engine Store (`ticket-engine/src/store`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Houses the in-memory repository store (`Corpus`) and transactional file I/O for the `.ai/tickets/` directory.

---

## Responsibilities

- **`corpus.rs`**: Loads all `T-*.toml` files into `BTreeMap<String, Ticket>`. Validates parent-child referential integrity (dotted notation rules) and loads `ScopeVocab`.
- **`disk.rs`**: Coordinates atomic file persistence. Writes files using a write-to-temp-then-atomic-rename strategy to prevent partial writes during agent crashes.
