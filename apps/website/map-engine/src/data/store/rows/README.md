# store

The Yrs document owner and its field-level transactions.

`doc_core.rs` declares shared state. Construction and hydration establish roots; materialization and row modules project authored JSON. Entity modules preserve field-level writes. Merge modules preserve authored order and remint colliding IDs. Undo clocks and transaction origins remain unchanged.
