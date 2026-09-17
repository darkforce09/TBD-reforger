# Database Verifications (`verifications/database`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Static analysis checks enforcing PostgreSQL safety and SQL seed integrity.

---

## Verifications

- **`no_select_star.rs`** (formerly `sql_gates.rs`): Bans `SELECT *` across all SQL queries on tables with nullable columns to prevent sqlx runtime decoding crashes.
- **`faction_library_seed.rs`** (formerly `gate_t440.rs`): Verifies that the US Army 1980s user faction seed SQL applies cleanly and seeds expected data.
- **`wiki_page_seed.rs`** (formerly `gate_t444.rs`): Verifies that tactical doctrine wiki page seeds apply cleanly.
