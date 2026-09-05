# T-940.1 — Plan

## Context
events.rs:2042-2053 promotes a waitlisted row with `UPDATE … SET state='registered'` only; slot_id stays NULL and capacity
(T-227, :1768-1798) is not re-checked. Priority 0.

## Approach
1. Verify on main: integration test promoting into a full mission succeeds with slot_id NULL; paste the red.
2. `migrations/0022_waitlist_seat.sql`: partial unique index (event_mission_id, slot_id) where registered and slot not null.
3. Promotion: one transaction — pick a free slot `FOR UPDATE SKIP LOCKED`, set slot_id + state, else 409 EVENT_FULL.
4. Response carries the assigned slot.
5. Perturbation: drop FOR UPDATE → concurrent test red; restore, touch, green.

## Risks
- Index conflicts with existing duplicate rows: the migration de-duplicates first or fails loudly.

## Verification
- `cargo xtask db test-it`
- `cargo xtask platform wave gate --slice T-940.1`
