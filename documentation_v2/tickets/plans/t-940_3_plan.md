# T-940.3 — Plan

## Context
events.rs:1338-1340 reschedules the event only; missions/missions.rs:816 delete_mission is a soft delete that leaves
event_missions and registrations pointing at a hidden mission. After T-940.2.

## Approach
1. Verify on main: reschedule by two hours leaves event_missions.start_time unchanged; paste the red.
2. `migrations/0024_event_missions_sync.sql`: hidden_at on event_missions; index on mission_id.
3. Reschedule: apply the delta to every event_missions row in the same transaction.
4. delete_mission: set hidden_at, withdraw registrations with a system reason; listings exclude hidden rows.
5. Perturbation: delta on the first mission only → multi-mission test red; restore, touch, green.

## Risks
- Missions rescheduled independently earlier: the delta applies to all; document it.

## Verification
- `cargo xtask db test-it`
- `cargo xtask platform wave gate --slice T-940.3`
