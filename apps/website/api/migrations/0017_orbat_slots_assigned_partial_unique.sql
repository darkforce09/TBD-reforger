-- T-511: one occupant per event-mission on orbat_slots.
--
-- Handlers already enforce one seat per caller (T-324 release-then-claim), and
-- T-331 cleared the content_golden double-seat that blocked CREATE UNIQUE INDEX
-- on a populated DB. The remaining blocker was tests/events.rs seeding a legacy
-- two-seat shape for T-318 multi-seat withdraw recovery — that seed is retired
-- (orphan recovery via assigned_to remains; multi-seat is unreachable under this
-- index, and a partial unique cannot be DEFERRABLE).
--
-- NULL assigned_to rows are free seats and may repeat; only claimed seats are
-- unique on (event_mission_id, assigned_to). Name follows idx_orbat_slots_* from
-- 0001_initial_schema.sql (idx_orbat_slots_assigned_to is the non-unique btree
-- on assigned_to alone — leave it; this is the structural one-seat guarantee).

CREATE UNIQUE INDEX IF NOT EXISTS idx_orbat_slots_em_assigned
    ON public.orbat_slots USING btree (event_mission_id, assigned_to)
    WHERE assigned_to IS NOT NULL;
