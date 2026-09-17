# Repository Invariant Verifications (`xtask/src/verifications`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Categorized repository integrity, architecture, licensing, and schema verification checks (formerly scattered flat under `gate_*.rs`).

In accordance with **Law 4 (Zero Context Needed)** and **Law 5 (Categorize Primitives — Avoid Flat Dumps)**, checks are partitioned into domain directories and named after the invariant they enforce *now* rather than historical ticket numbers.

---

## Verification Domains

- **`architecture/`**: Engine boundary layer isolation (Law 6) and Axum HTTP `@route` tag parity.
- **`licensing/`**: Upstream Arma Public License (APL) and Coalition code leak prevention.
- **`language_bans/`**: Hard-zero tracking for shell scripts (LANG-1), Python (LANG-2), and Node/npm (LANG-3).
- **`database/`**: SQL deserialization safety (no `SELECT *`) and database seed integrity.
- **`deployment/`**: Staging Docker compose file path pins.
- **`ci/`**: GitHub Actions workflow script hygiene and schema task wiring pins.
- **`registry/`**: Object palette alias ↔ spawn registry census audit.
- **`mod_scripts/`**: Enfusion mod script compilation, layout parsing, and spawn determinism.
- **`map_assets/`**: Semantic map object golden gates, label decluttering, and BLAS manifests.
- **`schemas/`**: Decomposed schema verification gates (<500 LOC per file).
