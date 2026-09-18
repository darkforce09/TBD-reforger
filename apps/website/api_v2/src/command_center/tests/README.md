# Command Center Sibling Unit Tests (`command_center/tests/`)

Sibling test files for the command center domain, adhering strictly to **Monorepo Law #7** (*no inline test modules; tests live in sibling files declared via `#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`*).

---

## 1. Test Suite Catalog

### `live_dashboard.rs` (<100 LOC)
- Tests composite dashboard query assembly.
- Asserts null-tolerance: when database has zero servers, zero operations, and zero announcements, endpoint responds with 200 OK and null fields rather than 500.
- Asserts priority ordering: operations authored or registered by the caller appear first in `next_event`.

### `leaderboards.rs` (<120 LOC)
- Tests whitelisted `ORDER BY` columns against arbitrary injection strings.
- Asserts mandatory presence of `, lt.discord_id ASC` tie-breaker on every sorting branch (T-311).
- Tests pagination bounds (min 1, max 100, default 20).

### `user_stats.rs` (<80 LOC)
- Tests attendance rate calculation with zero registrations (returns 0.0, no NaN or division-by-zero).
- Tests career dossier assembly from `users` and `leaderboard_totals`.
