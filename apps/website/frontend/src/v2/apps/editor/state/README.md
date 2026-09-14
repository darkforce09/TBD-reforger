# Reactive State & Persistence (`state`)

This directory manages the core document data layer.

---

## Responsibilities
- **CRDT Document Host (`doc.rs`):** Yrs CRDT document hosting and transaction boundaries.
- **Hydration (`hydrate.rs`):** Ingesting backend API JSON payloads into the CRDT structure.
- **Persistence (`persist.rs`):** Debounced autosave pipeline transmitting binary or JSON updates to the server.
- **History (`history.rs`):** Multi-level Undo/Redo stack with mutation receipts.
- **Session Management (`session.rs`):** Multi-tab editing locks and local user state.
