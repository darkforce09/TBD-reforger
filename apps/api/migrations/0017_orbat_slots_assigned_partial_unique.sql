-- One occupant per event mission on `orbat_slots`.
--
-- The handlers enforce one seat per caller (release-then-claim); this index is the
-- structural guarantee. NULL assigned_to rows are free seats and may repeat; only claimed
-- seats are unique on (event_mission_id, assigned_to). The name follows idx_orbat_slots_*
-- from the initial schema (idx_orbat_slots_assigned_to is the non-unique btree on
-- assigned_to alone — it stays).
--
-- ── NEUTRALISE, THEN ENFORCE ─────────────────────────────────────────────
--
-- A populated database can hold one member on two seats of one event mission: an earlier
-- golden seed produced exactly that, and fixing a seed does not fix data already seeded.
-- CREATE UNIQUE INDEX over such rows fails with
--
--   could not create unique index "idx_orbat_slots_em_assigned"
--   Key (event_mission_id, assigned_to)=(…) is duplicated.
--
-- and because migrations run on boot that is a dead API, not a failed deploy step.
--
-- The established shape is 0010's: make the offending rows non-offending FIRST, in the same
-- transaction, then apply the constraint. sqlx runs this file in one transaction, so a
-- database is either fully deduplicated-and-indexed or completely untouched — never stripped
-- of seats by a migration that then failed to add the index it stripped them for.
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
-- first tenant; the shape is (what was taken, from where, why, under which change), and
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
