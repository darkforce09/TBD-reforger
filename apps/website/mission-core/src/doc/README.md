# doc

The interactive mission document. Enable `doc` to use the Yrs store and headless editor operations.

`store/` owns CRDT transactions and projections; `crdt/` holds ordered arrays and undo clocks; `picking/` resolves geometric row candidates to mission identities; `operations/` composes edits. Camera projection and spatial queries are supplied by the graphics engine through the frontend. This crate has no UI, DOM, or graphics dependency.
