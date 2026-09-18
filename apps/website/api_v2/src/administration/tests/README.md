# Administration Sibling Unit Tests (`administration/tests/`)

Sibling test files for the administration domain, adhering strictly to **Monorepo Law #7** (*no inline test modules; tests live in sibling files declared via `#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`*).

---

## 1. Test Suite Catalog

### `personnel_roster.rs` (<100 LOC)
- Tests `push_search` SQL generator across `username`, `discord_handle`, `arma_id`, `arma_character`.
- Asserts mandatory projection of bare `users.total_deployments` column against synthetic zero literals.
- Verifies boundary limits: default 20 items, max 100 items, non-negative offsets.

### `disciplinary.rs` (<150 LOC)
- Tests ban input validation (rejects empty or whitespace-only reason strings per T-317).
- Tests warning input validation (rejects padded whitespace per T-343).
- Asserts token revocation side-effect upon ban execution.

### `role_management.rs` (<80 LOC)
- Tests `valid_role` parser against closed literal set (`enlisted`, `leader`, `mission_maker`, `admin`).
- Tests role rank comparisons (`role_rank`).

### `audit_logs.rs` (<120 LOC)
- Tests CSV formula escaping (`escape_csv_formula`) for `=`, `+`, `-`, `@`.
- Tests keyset pagination cursor validation (`id < $1`).
- Tests SSE row formatting and fallbacks.

### `audit_notifier.rs` (<60 LOC)
- Tests backoff reconnect timing and signal distribution.
