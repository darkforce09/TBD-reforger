# Ticket Engine Core (`ticket-engine/src/core`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Houses the fundamental domain types, vocabulary constraints, serialization rules, and time formatting for the ticket registry.

---

## Modules

- **`types.rs`**: Core domain enums and structs: `Domain` (`Website`, `Mod`, `Schema`, `Engine`, `Repo`), `ScopeV2` (4-level flat scope: Domain → Layer → Component → Surface), `Status` (`Idea`, `Queued`, `Ready`, `Running`, `Review`, `Shipped`, `Deferred`, `Cancelled`), `Ticket`.
- **`timestamp.rs`**: Strict RFC 3339 UTC lifecycle stamp parsing (`validate_rfc3339_utc`) and generation (`now_utc_rfc3339`). Rejects non-UTC or lowercase timezone formats.
- **`vocab.rs`**: Scope v2 vocabulary loader for `.ai/tickets/scope-vocab.toml`. Enforces valid architectural surfaces.
- **`encoding.rs`**: Custom TOML serializer and deserializer for `.ai/tickets/T-*.toml`. Preserves strict canonical key order, maps `status` + `order` onto `Status` enum.
