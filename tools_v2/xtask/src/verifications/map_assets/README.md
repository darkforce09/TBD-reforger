# Map Asset Verifications (`verifications/map_assets`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Static analysis checks verifying exported map data, elevation labels, and BVH manifests.

---

## Verifications

- **`map_object_golden.rs`** (formerly `golden_gate.rs`, 990 LOC): Enforces semantic object golden gates S2–S15 against committed Everon chunk baselines.
- **`terrain_labels.rs`** (formerly `label_gates.rs`, 975 LOC): Topographic contour label sampling, decluttering distance checks, and DEM elevation bounds.
- **`blas_manifest.rs`** (formerly `verify_blas_manifest.rs`, 247 LOC): Validates `blas-manifest.json` entries against emitted `.bvh` files.
