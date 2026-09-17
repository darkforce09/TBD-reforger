# Architectural Verifications (`verifications/architecture`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Enforces monorepo layering rules and interface contracts.

---

## Verifications

- **`engine_layer_boundaries.rs`** (formerly `gate_engine_layers.rs`): Enforces **Law 6 (Strict Boundary Layers)**. Asserts that `graphics-engine` never imports `map-engine` and contains zero map domain vocabulary (`terrain`, `symbology`, `mission`, `orbat`).
- **`route_tags.rs`** (formerly `gate_route_tags.rs`): Verifies bidirectional parity between Axum HTTP route handlers and markdown `@route` doc tags.
- **`editor_orbat_coherency.rs`** (formerly `gate_t180.rs`): Enforces that all ORBAT and entity edits route strictly through the Yjs CRDT store; bans direct mutation shortcuts.
