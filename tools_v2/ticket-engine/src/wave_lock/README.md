# Wave Lock Compiler (`ticket-engine/src/wave_lock`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Compiles and verifies the wave lockfile (`.ai/tickets/wave.lock`).

Relocated from `xtask/src/wave_lock.rs` (which contained 1,142 lines of production code and 1,027 lines of inline tests). Decomposed into modular files (<450 LOC each) with tests extracted to `ticket-engine/tests/wave_lock_tests.rs`.

---

## Submodules

- **`model.rs`**: Struct definitions for `WaveLockFile`, `WaveGroup`, and canonical TOML serialization ensuring deterministic output order.
- **`packer.rs`**: Topological DAG sorting, dependency resolution, and file-disjoint slice packing into waves.
- **`verifier.rs`**: Recomputes the plan in memory and fails with an explicit diff if `.ai/tickets/wave.lock` drifts from source tickets.
- **`runner.rs`**: Entry points for `repack()` (the sole legal writer) and `check()` (the CI verifier).
