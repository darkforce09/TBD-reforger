# Operations Services (`operations/services/`)

Internal business logic and transactional workflows supporting event operations, slotting promotions, and fire support calculations.

---

## 1. Domain Services

### `slot_promoter.rs` (<350 LOC)
- **Purpose**: Manages attendee slot lifecycle transitions, waitlist promotions, and vacancy backfills.
- **Key Functions**:
  - `promote_oldest_waitlist_attendee(pool: &PgPool, event_mission_id: Uuid) -> Result<Option<PromotedAttendee>, AppError>`: Selects the oldest waitlisted attendee (`ORDER BY created_at ASC FOR UPDATE SKIP LOCKED`) and promotes their status to `confirmed`.
  - `reconcile_mission_capacity(pool: &PgPool, event_mission_id: Uuid) -> Result<CapacitySummary, AppError>`: Recalculates occupied vs. waitlist headcounts against configured slot ceilings.
- **Invariants**:
  - Concurrency Gate G7b: Promotions and slot claims run inside serialized database transactions (`BEGIN ... COMMIT`) to prevent over-subscription under high contention.

### `orbat_materializer.rs` (<380 LOC)
- **Purpose**: Generates and materializes concrete `event_slots` and squad groupings from scenario templates when attaching a mission to an event.
- **Key Functions**:
  - `materialize_mission_orbat(pool: &PgPool, event_mission_id: Uuid, mission_id: Uuid) -> Result<usize, AppError>`: Clones scenario ORBAT definition into mutable database slot records.
  - `teardown_event_mission_slots(pool: &PgPool, event_mission_id: Uuid) -> Result<usize, AppError>`: Safely removes unassigned slots or rejects deletion if active registrations exist.

### `fire_mission_calculator.rs` (<320 LOC)
- **Purpose**: Backend computational service for mortar ballistic solutions and firing tables.
- **Key Functions**:
  - `calculate_solution(battery: &BatteryPosition, target: &TargetPosition, charge: MortarCharge) -> Result<FiringSolution, BallisticError>`: Calculates azimuth, elevation mil angles, time-of-flight, and dispersion ellipses.
- **Invariants**:
  - Validates ballistic boundaries (minimum/maximum range limits per charge configuration).
  - Target for future unification into `website-map-engine` for zero-allocation client/server shared math.
