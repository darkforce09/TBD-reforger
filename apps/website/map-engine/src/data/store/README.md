# data/store

The interactive mission document — the `yrs` layer. Enable `store` to use it and the headless
editor operations.

`rows/` owns CRDT transactions and projections; `crdt/` holds ordered arrays and undo clocks;
`selection.rs` resolves geometric row candidates to mission identities; `operations/` composes
edits. Camera projection and spatial queries are supplied by the rest of the map engine through
the frontend. Nothing here has a UI, DOM, or graphics dependency.

`rows` and `selection` are PRIVATE. The five `pub use` blocks in `mod.rs` are the whole surface,
and they were the whole surface before the fold too — `doc::picking` was a public module that
re-exported nothing, because its contents are inherent `impl MissionDocCore` blocks.

Folded in from `website-mission-core/src/doc` at T-0xx Phase 2A, which is also when the term
"mission doc" was retired: it never named a concept, only this store.

## Boundaries

Geometric queries return row indices and squared distances; the frontend supplies world coordinates
and pixel tolerances converted by its camera. Slot candidates keep their established precedence over
equal-distance vehicles. Authored order, numeric precision, and undo behaviour are preserved.
