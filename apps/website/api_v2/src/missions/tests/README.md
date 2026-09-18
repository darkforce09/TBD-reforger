# Missions Tests (`missions/tests/`)

Sibling unit and integration test specifications for scenario models, compilation pipelines, approval workflows, and prefab catalogs.

---

## 1. Test Modules

Declared via Monorepo Law #7 (`#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`).

### `library.rs`
- **Coverage**: Scenario catalog queries, pagination limits, filter combinations (author, terrain, tag, play count), and soft-delete protections.

### `upload_ingest.rs`
- **Coverage**: Scenario upload size limits (400MB ceiling), ZIP format validation, manifest extraction, duplicate GUID collision detection, and malformed payload rejections.

### `approvals_queue.rs`
- **Coverage**: Submission transitions (draft -> pending -> approved / rejected), reviewer role gating (`mission_maker` vs `admin`), and automated audit log emission.

### `prefab_registry.rs`
- **Coverage**: Prefix and category filtering on prefabs, full-text asset searches, pagination, and invalid GUID queries.

### `game_server_injection.rs`
- **Coverage**: Game agent payload signing, HMAC secret authentication, timeout handling, and server error response parsing.

### `mission_compiler.rs`
- **Coverage**: AST node validation, circular dependency detection in trigger conditions, entity count limits, and deterministic output byte hashing.

### `validation.rs`
- **Coverage**: Terrain boundary violations, missing required vehicle crew roles, invalid loadout equipment slotting, and schema version mismatches.
