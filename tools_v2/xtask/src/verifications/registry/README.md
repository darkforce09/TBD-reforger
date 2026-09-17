# Registry Census Verifications (`verifications/registry`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Audits entity registries and prefab aliases.

---

## Verifications

- **`object_alias_spawn_census.rs`** (formerly `gate_t439.rs`): Audits parity between editor object palette aliases and Enfusion spawn registry entries, proving matching GUID rows exist for every palette item.
