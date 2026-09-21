-- ORBAT slot uniqueness includes the faction.
--
-- `idx_orbat_slot` was (event_mission_id, squad, slot_index) while `orbat_slots.faction` has
-- always been bound by `materialize_slots`. A mission whose ORBAT uses the same squad name on
-- both sides ("Alpha 1-1" for BLUFOR *and* OPFOR is the normal naming convention, not a corner
-- case) collided on the second faction's slot 0. The INSERT carries no ON CONFLICT, so the unique
-- violation propagated out of `materialize_slots` as a sqlx error and `add_event_mission`
-- answered 500 — with the whole attach rolled back, so the event mission was lost too, not just
-- the duplicate squad.
--
-- SAFE ON POPULATED DATABASES WITH NO DEDUPE STEP. The new key is a strict superset of the
-- old one: any row set satisfying (event_mission_id, squad, slot_index) necessarily satisfies
-- (event_mission_id, faction, squad, slot_index). The old index was in force for the entire
-- life of the table, so a violating row cannot exist. Widening a unique index only ever
-- admits rows; it never rejects one it previously accepted.
--
-- Column order puts faction second so the index still covers `get_orbat`'s
-- `ORDER BY faction ASC, squad ASC, slot_index ASC` as an ordered scan rather than a sort.
--
-- Drop-then-recreate under the same name follows the 0007 precedent
-- (idx_registry_compat_edge). The IF EXISTS / IF NOT EXISTS guards make the pair replayable;
-- sqlx runs each migration in a transaction, so the window with no unique key is not
-- observable by any other session.


DROP INDEX IF EXISTS idx_orbat_slot;

CREATE UNIQUE INDEX IF NOT EXISTS idx_orbat_slot
    ON public.orbat_slots USING btree (event_mission_id, faction, squad, slot_index);

-- NOT INCLUDED HERE — the one-seat-per-user constraint:
--
--   CREATE UNIQUE INDEX ... ON orbat_slots (event_mission_id, assigned_to)
--       WHERE assigned_to IS NOT NULL;
--
-- It is semantically right and it is the only thing that forces `event_registrations.slot_id`
-- and `orbat_slots.assigned_to` to agree. It cannot ship in this file: a populated database can
-- hold one member on two seats of one event mission, so CREATE UNIQUE INDEX would ERROR — and
-- migrations run on API boot, which turns one bad row into a platform that will not start.
-- Landing it needs a dedupe step whose "which seat wins" rule is deterministic and recorded,
-- and the handlers must already refuse a second seat, or the index converts that silent bug
-- into a 500 on the claim path. 0017 lands it that way.
