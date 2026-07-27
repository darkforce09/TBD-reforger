-- T-284: drop dead `events.match_id`.
--
-- The column exists on `events` (0001_initial_schema.sql) and is SELECTed into the Event
-- model / every event list+get query, but no INSERT or UPDATE ever writes it. The real
-- event↔match link is `matches.event_id` (written by telemetry ingest). Leaving a
-- forever-NULL column on the wire invites false reads ("this event has no match") when
-- the truth is the other direction.
--
-- SAFE: nothing writes the column; `matches.event_id` is untouched.

ALTER TABLE public.events DROP COLUMN IF EXISTS match_id;
