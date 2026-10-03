//! Role: the world residency's unit tests, grouped by what they drive.
//! Position: `chunk_draw_buffers::tests`, compiled only in test builds from
//! the crate root; each child imports the items it drives from their owning modules.
//! Signals & state: none; every case builds its own residencies.
//! Invariants: the cases drive the composed world residency, so each checks the chunk residency
//! and the draw buffers together.

mod everon_glyphs_and_strips;
mod ingest_budget_and_building_toggle;
mod prefab_lane_parity;
mod residency_lifecycle;
