# Mathematical Invariant Gates (`world_export_pipeline/mathematical_gates`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Automated mathematical integrity verification enforcing G1–G12 spatial invariants and phase transition gates.

Decomposed from the 1,290-line legacy `gates.rs` file into modules strictly under **450 LOC**.

---

## Submodules

- **`global_invariants.rs`** (<450 LOC): Enforces G1–G12 mathematical integrity rules on exported chunk datasets.
- **`phase_gates.rs`** (<450 LOC): Enforces PH-P1 and PH-P2 phase validation rules during export pipeline execution.
- **`coverage_gates.rs`** (<390 LOC): Verifies vegetation density bounds and forest polygon area thresholds.
