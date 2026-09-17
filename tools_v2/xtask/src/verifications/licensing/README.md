# Licensing & Intellectual Property (`verifications/licensing`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Guards against third-party code leaks and license violations.

---

## Verifications

- **`no_crf_oracle_leak.rs`** (formerly `gate_crf_leak.rs`): Audits `apps/mod/tbd-framework` to guarantee zero upstream Coalition Reforger Framework (CRF) symbols, class names, or GUIDs leak into the proprietary TBD codebase.
