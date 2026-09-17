# View Synchronization Engine (`ticket-engine/src/sync`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Generates read-only projections and views of the ticket database into repository documentation.

Relocated from `xtask/src/sync.rs`. Decomposed into modules under 300 LOC.

---

## Submodules

- **`markdown_views.rs`**: Renders `TICKET_REGISTRY.md`, `TICKET_LEAD.md`, `TICKET_DEV_QUEUE.md`, and `TICKET_MOD_QUEUE.md` from the validated corpus.
- **`queue_json.rs`**: Generates `.ai/tickets/queue.json` and synchronizes roadmap markers in milestone files.
- **`runner.rs`**: Top-level entry point `sync_all(&Corpus, &Path) -> Result<()>`. Refuses empty writes.
