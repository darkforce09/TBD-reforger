-- T-511: one occupant per event-mission on orbat_slots.
--
-- Handlers already enforce one seat per caller (T-324 release-then-claim). The
-- remaining blocker was tests/events.rs seeding a legacy two-seat shape for T-318
-- multi-seat withdraw recovery — that seed is retired (orphan recovery via
-- assigned_to remains; multi-seat is unreachable under this index, and a partial
-- unique cannot be DEFERRABLE).
--
-- NULL assigned_to rows are free seats and may repeat; only claimed seats are
-- unique on (event_mission_id, assigned_to). Name follows idx_orbat_slots_* from
-- 0001_initial_schema.sql (idx_orbat_slots_assigned_to is the non-unique btree
-- on assigned_to alone — leave it; this is the structural one-seat guarantee).
--
-- ── T-555: THE HEADER USED TO CLAIM THIS WAS ALREADY SAFE. IT WAS NOT. ────────
--
-- The clause removed from the paragraph above read: "and T-331 cleared the
-- content_golden double-seat that blocked CREATE UNIQUE INDEX on a populated DB".
-- That is false, and the falsehood is the whole defect. T-331 edited
-- seeds/content_golden.sql so a FRESH database would stop producing the double
-- seat. It never touched the row the OLD seed had already inserted. Fixing a seed
-- does not fix data already seeded.
--
-- So on every database that had run the pre-T-331 seed — the operator's dev DB,
-- staging, production — this file died on arrival:
--
--   could not create unique index "idx_orbat_slots_em_assigned"
--   Key (event_mission_id, assigned_to)=(89b1b731-…-3c7ff7de5eb3, 000000000000000005)
--   is duplicated.
--
-- and because migrations run on boot (bin/api.rs:26) that is a dead API, not a
-- failed deploy step. Measured on a scratch DB rebuilt to the pre-T-555 shape:
-- user 000000000000000005 held BOTH slot …0005 (Alpha/Grenadier, assigned
-- 2026-07-17 20:03:47) and slot …0015 (Recon/Designated Marksman, 2026-07-18
-- 11:22:03) on event_mission 89b1b731-37a8-4926-901a-3c7ff7de5eb3.
--
-- The gate could not see it: ensure_gate_db force-drops its migrate database every
-- run, so db_migrate only ever ran these files FORWARD FROM EMPTY, where the fixed
-- seed produces no duplicate by construction. That hole is closed by the
-- `db_migrate persist` step in scripts/platform/wave.sh, which keeps a database
-- across runs and applies only what is new to it.
--
-- ── NEUTRALISE, THEN ENFORCE ─────────────────────────────────────────────────
--
-- The established shape in this repo is T-405's
-- 0010_backfill_aar_replay_url_scheme.sql: make the offending rows non-offending
-- FIRST, in the same transaction, then apply the constraint. sqlx runs this file in
-- one transaction, so a database is either fully deduplicated-and-indexed or
-- completely untouched — never stripped of seats by a migration that then failed to
-- add the index it stripped them for.
--
-- KEEP THE EARLIEST SEAT, FREE THE REST. Earliest = lowest assigned_at, with
-- slot_index then id as deterministic tie-breaks so two runs over the same data
-- always choose the same survivor. `NULLS LAST` puts a claimed-but-undated seat
-- (assigned_to set, assigned_at NULL — an anomaly, not a state the claim path can
-- produce) behind every dated one, so a real recorded claim always outranks it.
--
-- A NULL assigned_to is a FREE SEAT and is explicitly legal under this partial
-- index, so freeing the later duplicates is the minimum edit that makes the DDL
-- apply. assigned_at is cleared alongside it: every free seat in
-- seeds/content_golden.sql carries both columns NULL, and leaving a timestamp on an
-- unoccupied seat would invent a state the readers do not expect.
--
-- QUARANTINE FIRST, because this DELETES A FACT — that a specific person held a
-- specific seat — and a migration that silently forgets it leaves an incident review
-- with nothing to read. The rows go to public.url_quarantine (created by 0010, which
-- runs first). That table is reused rather than duplicated on 0010's own stated
-- design: "The table is deliberately generic (`table_name` / `column_name` /
-- `row_id`) … One table beats four." The name says url only because URLs were its
-- first tenant; the shape is (what was taken, from where, why, by which ticket), and
-- one place to look beats two. Undo for a wrongly-freed seat is a single
-- `UPDATE orbat_slots … FROM url_quarantine`.
--
-- IDEMPOTENT. A second run finds no seat_rank > 1 (the first run made them NULL, and
-- NULL assigned_to is excluded by the WHERE), the INSERT is ON CONFLICT DO NOTHING
-- against 0010's unique key, and CREATE UNIQUE INDEX IF NOT EXISTS is a no-op once
-- the index is in force. Safe on a database that never had a duplicate: all three
-- statements match zero rows.

INSERT INTO public.url_quarantine
    (table_name, column_name, row_id, original_value, reason, ticket)
SELECT 'orbat_slots', 'assigned_to', d.id, d.assigned_to,
       'duplicate seat — this occupant already held an earlier seat in the same '
       || 'event_mission; freed so idx_orbat_slots_em_assigned could be created',
       'T-555'
FROM (
    SELECT id,
           assigned_to,
           row_number() OVER (
               PARTITION BY event_mission_id, assigned_to
               ORDER BY assigned_at NULLS LAST, slot_index, id
           ) AS seat_rank
    FROM public.orbat_slots
    WHERE assigned_to IS NOT NULL
) d
WHERE d.seat_rank > 1
ON CONFLICT (table_name, column_name, row_id) DO NOTHING;

UPDATE public.orbat_slots
   SET assigned_to = NULL,
       assigned_at = NULL
 WHERE id IN (
    SELECT d.id
    FROM (
        SELECT id,
               row_number() OVER (
                   PARTITION BY event_mission_id, assigned_to
                   ORDER BY assigned_at NULLS LAST, slot_index, id
               ) AS seat_rank
        FROM public.orbat_slots
        WHERE assigned_to IS NOT NULL
    ) d
    WHERE d.seat_rank > 1
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_orbat_slots_em_assigned
    ON public.orbat_slots USING btree (event_mission_id, assigned_to)
    WHERE assigned_to IS NOT NULL;
