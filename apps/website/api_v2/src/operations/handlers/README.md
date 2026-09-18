# Operations Handlers (`operations/handlers/`)

HTTP endpoint handlers for operations CRUD, lifecycle transitions, Gate G7b registration, ORBAT tree compilation, service records, and saved fire missions.

---

## 1. Handlers & Route Mappings

### `events_crud.rs` (<420 LOC)
- **`GET /api/v1/events`** (`list_events`): Filterable operations list.
- **`POST /api/v1/events`** (`create_event`): Create operation container.
- **`GET /api/v1/events/{id}`** (`get_event`): Dossier aggregation with nested mission cards.
- **`PATCH /api/v1/events/{id}`** (`update_event`): Update metadata and execute validated transitions.
- **`DELETE /api/v1/events/{id}`** (`delete_event`): Soft-delete operation (`deleted_at = now()`).

### `event_lifecycle.rs` (<360 LOC)
- Dynamic SQL status derivation (`EFFECTIVE_STATUS_SQL`).
- Validates transition graph (`can_transition`: scheduled -> open -> locked -> live -> completed).
- Advisory transactional locking (`pg_try_advisory_xact_lock(0x7BD_0225)`).

### `registration.rs` (<390 LOC)
- **`POST /api/v1/event-missions/{emid}/register`** (`register_for_event_mission`): Concurrency Gate G7b slot claim/waitlist.
- **`DELETE /api/v1/event-missions/{emid}/register`** (`withdraw_from_event_mission`): Release seat, auto-promote oldest waitlist entry.

### `orbat_structure.rs` (<430 LOC)
- **`POST /api/v1/events/{id}/missions`** (`add_event_mission`): Attach mission; materializes ORBAT slots.
- **`DELETE /api/v1/events/{id}/missions/{emid}`** (`remove_event_mission`): Detach mission.
- **`GET /api/v1/event-missions/{emid}/orbat`** (`get_orbat`): Grouped ORBAT tree by `(faction, squad)`.
- **`PUT /api/v1/event-missions/{emid}/slots/{slotId}/assign`** (`assign_slot`): Leader slot assignment.
- **`DELETE /api/v1/event-missions/{emid}/slots/{slotId}/assign`** (`clear_slot`): Unseat occupant.
- **`POST /api/v1/event-missions/{emid}/squads/reserve`** (`reserve_squad`): Hold squad.
- **`POST /api/v1/event-missions/{emid}/squads/release`** (`release_squad`): Release squad hold.
- **`GET /api/v1/members`** (`search_members`): Member search for leader slotting.
- **`GET /api/v1/ingest/events/{id}/roster`** (`ingest_event_roster`): Server roster sync.

### `deployments.rs` (<450 LOC)
- **`GET /api/v1/me/deployments`** (`get_my_deployments`): Service record.
- **`POST /api/v1/me/leave-requests`** (`submit_leave`): Submit LOA.
- **`GET /api/v1/me/leave-requests`** (`list_my_leave`): List caller's LOAs.
- **`GET /api/v1/admin/leave-requests`** (`list_all_leave`): LOA review queue.
- **`PATCH /api/v1/admin/leave-requests/{id}`** (`review_leave`): Approve/deny LOA.

### `fire_missions.rs` (<290 LOC)
- **`POST /api/v1/fire-missions`** (`save_fire`): Persist mortar firing solution.
- **`GET /api/v1/events/{id}/fire-missions`** (`list_event_fire_missions`): Retrieve event fire missions.
