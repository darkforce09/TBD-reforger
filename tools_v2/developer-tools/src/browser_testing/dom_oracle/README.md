# DOM Regression Oracle (`browser_testing/dom_oracle`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Decomposed from the 573-line `vsuite.rs` file into production logic (<420 LOC) and sibling tests.

---

## Responsibilities

- **`oracle.rs`**: Captures normalized DOM tree snapshots from the headless browser, strips dynamic session IDs, compares against golden baselines, and emits human-readable unified diffs on regression.
