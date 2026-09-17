# Browser Preflight Diagnostics (`browser_testing/diagnostics`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Decomposed from the 742-line `doctor.rs` file into focused submodules (<400 LOC each).

---

## Responsibilities

- **`environment_checks.rs`**: Validates that system RAM meets thresholds, headless font caches (`Liberation Sans`, `Fontconfig`) are present, and Chrome version pins match CI locks.
- **`liveness_probe.rs`**: Probes Chromium startup latency and WebSocket connectivity before test suite execution begins.
