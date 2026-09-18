# Operations Sibling Unit Tests (`operations/tests/`)

Sibling test files for the operations domain, adhering strictly to **Monorepo Law #7** (*no inline test modules; tests live in sibling files declared via `#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`*).

---

## 1. Test Suite Catalog

### `events_crud.rs` (<150 LOC)
- Tests `name_override` whitespace rejection.
- Asserts soft-delete preservation of historical event records.
- Tests event list scope filters (`upcoming`, `past`, `all`).

### `event_lifecycle.rs` (<150 LOC)
- Tests state transition validity matrix (`can_transition`).
- Tests advisory transaction lock behavior (`pg_try_advisory_xact_lock`).
- Tests effective status derivation from `now()`.

### `registration.rs` (<180 LOC)
- Tests Gate G7b two-level lock sequence.
- Tests atomic release-then-claim when switching seats in the same operation.
- Tests waitlist promotion ordering and zero-overpromotion invariant.

### `orbat_structure.rs` (<140 LOC)
- Tests zero-slot ORBAT attachment rejection.
- Tests squad reservation lockouts.
- Tests leader assignment permissions.

### `deployments.rs` (<100 LOC)
- Tests LOA start/end date ordering.
- Tests attendance rate calculation against attended registrations.

### `fire_missions.rs` (<80 LOC)
- Tests fire mission coordinate bounds and column projection.
