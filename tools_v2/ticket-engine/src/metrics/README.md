# Metrics & Token Estimator (`ticket-engine/src/metrics`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Execution receipt analysis, elapsed time calculation, and ticket token estimation.

Relocated from `xtask/src/metrics.rs` and `xtask/src/estimate_tokens.rs`.

---

## Responsibilities

- **`receipts.rs`**: Parses agent run receipts under `.ai/tickets/metrics/<id>/` and extracts elapsed execution durations using RFC 3339 timestamp arithmetic.
- **`tokens.rs`**: Analyzes prompt token usage across completed and shipped tickets to calibrate agent task budgets.
