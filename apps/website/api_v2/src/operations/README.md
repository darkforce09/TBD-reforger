# Operations Subsystem (`operations/`)

Community operations, event scheduling, ORBAT reservations, attendance records, member deployment service histories, and saved mortar fire missions.

---

## 1. Subsystem Topology & Responsibilities

The `operations/` domain decomposes the massive 3,022 LOC `events.rs` monolith, re-homes member deployments from `telemetry/`, and adopts `FireMission` from `admin.rs` into focused, cohesive modules:

```text
src/operations/
├── README.md                           <-- Domain documentation (this document)
├── routes.rs                           <-- /api/v1/events & /api/v1/event-missions sub-router (<90 LOC)
│
├── models/
│   ├── mod.rs
│   ├── event.rs                        <-- Event, EventMission, EventRegistration, OrbatSlot, LOA
│   └── fire_mission.rs                 <-- FireMission model with all 17 columns (MOVED OUT OF ADMIN!)
│
├── handlers/
│   ├── mod.rs
│   ├── events_crud.rs                  <-- Operation creation, listing, dossier & soft-delete (<420 LOC)
│   ├── event_lifecycle.rs              <-- State transitions (Scheduled -> Live -> Completed) (<360 LOC)
│   ├── registration.rs                 <-- Concurrency Gate G7b: slot claims & waitlists (<390 LOC)
│   ├── orbat_structure.rs              <-- Squad reservations, leader slot assignment, roster (<430 LOC)
│   ├── deployments.rs                  <-- Member service records, stats, LOA review (<450 LOC)
│   └── fire_missions.rs                <-- CRUD persistence for saved mortar fire missions (<290 LOC)
│
└── tests/                              <-- Non-inline sibling unit tests
    ├── events_crud.rs
    ├── event_lifecycle.rs
    ├── registration.rs
    ├── orbat_structure.rs
    ├── deployments.rs
    └── fire_missions.rs
```

---

## 2. HTTP Route Catalog

| Verb | Path | Handler | Auth Extractor | Description |
|:---|:---|:---|:---|:---|
| `GET` | `/api/v1/events` | `events_crud::list_events` | `AuthUser` | Filterable operations list (upcoming, past, all). |
| `POST` | `/api/v1/events` | `events_crud::create_event` | `AdminUser` | Create new operation container. |
| `GET` | `/api/v1/events/{id}` | `events_crud::get_event` | `AuthUser` | Event Hub dossier with nested mission cards and armories. |
| `PATCH` | `/api/v1/events/{id}` | `events_crud::update_event` | `AdminUser` | Partial update with lifecycle state transition validation. |
| `DELETE` | `/api/v1/events/{id}` | `events_crud::delete_event` | `AdminUser` | Soft-delete operation (`deleted_at = now()`). |
| `POST` | `/api/v1/events/{id}/missions` | `orbat_structure::add_event_mission` | `AdminUser` | Attach mission; snapshots and materializes ORBAT slots. |
| `DELETE` | `/api/v1/events/{id}/missions/{emid}`| `orbat_structure::remove_event_mission`| `AdminUser` | Detach mission, cascading removal of registrations and slots. |
| `GET` | `/api/v1/event-missions/{emid}/orbat`| `orbat_structure::get_orbat` | `AuthUser` | Grouped ORBAT tree by `(faction, squad)` with member display names. |
| `POST` | `/api/v1/event-missions/{emid}/register`| `registration::register_for_event_mission`| `AuthUser` | **Gate G7b**: Lock row, release prior seat, claim slot or waitlist. |
| `DELETE`| `/api/v1/event-missions/{emid}/register`| `registration::withdraw_from_event_mission`| `AuthUser` | Release seat, remove registration, auto-promote oldest waitlist. |
| `PUT` | `/api/v1/event-missions/{emid}/slots/{slotId}/assign`| `orbat_structure::assign_slot`| `LeaderUser` | Squad leader or admin assigns member directly to slot. |
| `DELETE`| `/api/v1/event-missions/{emid}/slots/{slotId}/assign`| `orbat_structure::clear_slot`| `LeaderUser` | Squad leader or admin unseats slot occupant. |
| `POST` | `/api/v1/event-missions/{emid}/squads/reserve`| `orbat_structure::reserve_squad`| `LeaderUser` | Hold entire squad for designated assignment. |
| `POST` | `/api/v1/event-missions/{emid}/squads/release`| `orbat_structure::release_squad`| `LeaderUser` | Release squad hold. |
| `GET` | `/api/v1/members` | `orbat_structure::search_members` | `AuthUser` | Directory search for leader slot assignment. |
| `GET` | `/api/v1/ingest/events/{id}/roster`| `orbat_structure::ingest_event_roster`| `ServiceAuth`| Dedicated server roster mapping `arma_id` to slot `uid`. |
| `GET` | `/api/v1/me/deployments` | `deployments::get_my_deployments` | `AuthUser` | Member service record: upcoming signups, combat totals, past ops. |
| `POST` | `/api/v1/me/leave-requests` | `deployments::submit_leave` | `AuthUser` | File Leave of Absence (LOA). |
| `GET` | `/api/v1/me/leave-requests` | `deployments::list_my_leave` | `AuthUser` | List caller's submitted LOAs. |
| `GET` | `/api/v1/admin/leave-requests` | `deployments::list_all_leave` | `AdminUser` | LOA administrative review queue. |
| `PATCH` | `/api/v1/admin/leave-requests/{id}`| `deployments::review_leave` | `AdminUser` | Approve or deny LOA request. |
| `POST` | `/api/v1/fire-missions` | `fire_missions::save_fire` | `AuthUser` | Compute and persist mortar firing solution to database. |
| `GET` | `/api/v1/events/{id}/fire-missions` | `fire_missions::list_event_fire_missions`| `AuthUser` | Retrieve all saved mortar solutions for an event. |

---

## 3. Key Invariants & Concurrency Rules

### 3.1 Concurrency Gate G7b (`registration.rs`)
Slot claims enforce a two-level transactional lock order:
1. `SELECT ... FOR UPDATE` on `event_missions` to verify event mission state.
2. `SELECT ... FOR UPDATE OF e` on `events` to serialize concurrent claims against the operation container.
3. Atomic seat release: If the user already occupies a seat in the mission, the prior seat is cleared in the same transaction before claiming the new seat.
4. If the target slot is occupied, the caller is placed on the FIFO waitlist. Withdrawal auto-promotes the oldest waitlist entry into the vacated slot.

### 3.2 Relocation of `FireMission` Model
`FireMission` is moved out of `models/admin.rs` into `operations/models/fire_mission.rs`. The 7 nullable coordinate and solution fields (`fp_x`, `fp_y`, `tgt_x`, `tgt_y`, `azimuth_mils`, `charge`, `time_of_flight_s`) serialize unconditionally as `null` when unset to inform clients of field presence without fabricating values.
