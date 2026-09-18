-- T-228: ORBAT slot uniqueness was faction-blind.
--
-- `idx_orbat_slot` was (event_mission_id, squad, slot_index). `orbat_slots.faction` has
-- existed since the frozen initial schema and `materialize_slots` (handlers/events.rs:391)
-- has always bound it — the unique key simply ignored it. So a mission whose ORBAT fields
-- the same squad name on both sides ("Alpha 1-1" for BLUFOR *and* OPFOR is the normal naming
-- convention, not a corner case) collides on the second faction's slot 0. The INSERT carries
-- no ON CONFLICT, so the unique violation propagates out of `materialize_slots` as a sqlx
-- error and `add_event_mission` returns 500 — with the whole attach rolled back, so the
-- event mission is lost too, not just the duplicate squad.
--
-- SAFE ON POPULATED DATABASES WITH NO DEDUPE STEP. The new key is a strict superset of the
-- old one: any row set satisfying (event_mission_id, squad, slot_index) necessarily satisfies
-- (event_mission_id, faction, squad, slot_index). The old index was in force for the entire
-- life of the table, so a violating row cannot exist. Widening a unique index only ever
-- admits rows; it never rejects one it previously accepted.
--
-- Column order puts faction second so the index still covers get_orbat's
-- `ORDER BY faction ASC, squad ASC, slot_index ASC` (handlers/events.rs:1053) as an ordered
-- scan rather than a sort.
--
-- Drop-then-recreate under the same name follows the 0007 precedent
-- (idx_registry_compat_edge). The IF EXISTS / IF NOT EXISTS guards make the pair replayable;
-- sqlx runs each migration in a transaction, so the window with no unique key is not
-- observable by any other session.

DROP INDEX IF EXISTS idx_orbat_slot;

CREATE UNIQUE INDEX IF NOT EXISTS idx_orbat_slot
    ON public.orbat_slots USING btree (event_mission_id, faction, squad, slot_index);

-- NOT INCLUDED, DELIBERATELY — the T-318/T-324 one-seat-per-user constraint:
--
--   CREATE UNIQUE INDEX ... ON orbat_slots (event_mission_id, assigned_to)
--       WHERE assigned_to IS NOT NULL;
--
-- It is semantically right and it is the only thing that would force
-- `event_registrations.slot_id` and `orbat_slots.assigned_to` to agree. It cannot ship here:
-- the operator's live `tbd_reforger` already violates it (discord_id 000000000000000005 holds
-- BLUFOR/Alpha #1 and OPFOR/Recon #1 in event mission 89b1b731-…-3c7ff7de5eb3), so CREATE
-- UNIQUE INDEX would ERROR — and migrations run on API boot (bin/api.rs:26), which turns one
-- bad row into a platform that will not start. Landing it needs a dedupe step whose
-- "which seat wins" rule silently unseats a real user, and it must land in the same wave as
-- T-324's handler fix or it converts that silent bug into a 500 on the claim path.
