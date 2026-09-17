# Forest Polygon Smoothing (`world_export_pipeline/forest_smoothing`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Runs computational geometry smoothing and topology verification algorithms on forest canopy regions.

Decomposed from the 1,244-line legacy `forest_smooth.rs` file into modules strictly under **450 LOC** with unit tests moved to sibling files.

---

## Submodules

- **`chaikin.rs`** (<450 LOC): Core Chaikin polygon curve subdivision algorithm.
- **`topology.rs`** (<400 LOC): Resolves self-intersections and prunes degenerate polygon rings.
