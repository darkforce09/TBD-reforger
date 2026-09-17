# Task Runner Test Fixtures (`xtask/test_fixtures`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Golden assets, reference BVH baselines, and recorded agent CLI receipts.

Renamed from `xtask/tests/` to eliminate confusion with Cargo integration test crates.

---

## Fixture Inventory

- **`FarmHouse_E_1L01_Wood.*`**: Golden BVH acceleration trees, instances, and blueprint JSON baselines for building verification.
- **`world_parity_*.json`**: Golden entity census and placement parity baselines.
- **`slice_run_*.json`**: Recorded Claude Code and Cursor CLI run receipts for parser testing.
