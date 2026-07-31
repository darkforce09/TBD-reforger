-- T-585 — the three ingest-pointer foreign keys 0018 abstained from.
--
-- 0018 constrained 25 relationships and deliberately left four alone. Abstention (iv)
-- (`0018_foreign_keys.sql:151-169`) names them: `matches.event_id`, `matches.mission_id`,
-- `server_statuses.current_match_id`, `fire_missions.event_id` — "POINTERS TAKEN VERBATIM FROM A
-- REQUEST BODY WITH NO EXISTENCE CHECK". Its stated reason was not that the constraint is wrong.
-- It was that there is no SQLSTATE 23503 handler in the crate, so a violation would reach a
-- no-human-in-the-loop game-server bridge as a **500** and cost the whole scoreline.
--
-- T-576 removed that reason. `handlers/telemetry.rs:69` now carries `foreign_key_error`, which
-- maps a 23503 onto a 400 that names the pointer the database could not find, and it is armed
-- for these three constraint names ALREADY (`telemetry.rs:46-48`) — written before this file
-- existed, on purpose, so that lifting the abstention would be pure SQL.
--
-- ── VERIFIED ON MAIN BEFORE WRITING A LINE ──────────────────────────────────────────────────
--
-- Against the operator's live `tbd_reforger` at migration 18, read-only, 2026-07-31:
--
--   SELECT conname FROM pg_constraint
--    WHERE contype='f' AND conrelid::regclass::text IN ('matches','server_statuses');
--   → server_statuses_server_id_fkey
--
-- ONE row. That is 0018's constraint 10 — the single ingest pointer T-262 did constrain. None of
-- the three below exists anywhere yet, and `grep -rn` over the whole tree finds their names in
-- exactly one place: the three `const` items in `telemetry.rs` waiting for this file.
--
-- All three columns are nullable (`information_schema.columns`, same database, same run:
-- matches.event_id YES, matches.mission_id YES, server_statuses.current_match_id YES), which is
-- what makes `ON DELETE SET NULL` expressible at all.
--
-- ═══ THE NAMES ARE A CONTRACT, NOT A STYLE CHOICE ═══════════════════════════════════════════
--
-- `foreign_key_error` branches on the constraint name as a literal string. A constraint that
-- does the same job under any other name matches NO arm, falls through to `Err(e) => e.into()`,
-- and **silently restores the exact 500 this file exists to prevent** — with the FK now in
-- force, so the write fails too. That failure mode is invisible: the schema looks right, the
-- handler looks right, and only a live 23503 shows it.
--
-- So the three names below are copied from `telemetry.rs:46-48` character for character:
--
--     server_statuses_current_match_id_fkey
--     matches_event_id_fkey
--     matches_mission_id_fkey
--
-- They are also what Postgres would have auto-named them (`<table>_<column>_fkey`) and what all
-- 25 of 0018's constraints use, so the convention and the contract agree. `telemetry.rs`'s
-- `fk_constant_names_follow_migration_convention` test pins the convention from the other side.
-- **If you rename a constraint here you must edit `telemetry.rs` in the same commit.**
--
-- ═══ ON DELETE SET NULL, FOR ALL THREE ══════════════════════════════════════════════════════
--
-- Each of these is an *attribution* pointer: a nullable annotation on a row whose value does not
-- depend on it. The match record is the scoreline; the event it belonged to is metadata. So when
-- the parent goes, the child must lose the pointer and keep the row.
--
--   * RESTRICT/NO ACTION would make deleting an event impossible once any match referenced it,
--     and would do it from a table nobody deleting an event is looking at.
--   * CASCADE is worse and in one case destructive: cascading `matches` → would delete the
--     scoreline (and its `match_player_stats` children, which 0018 already CASCADEs) because an
--     *event* was removed; cascading `server_statuses.current_match_id` would delete a server's
--     entire live status row when a finished match is purged, and `list_servers` renders that
--     row. NULL is precisely the state both readers already handle — `server_statuses` is
--     seeded with a NULL `current_match_id` on the idle server for that reason
--     (`seeds/content_golden.sql` §7), and `matches` with NULL `event_id` on all three rows.
--
-- No `ON UPDATE`, for 0018's reason: every parent key here is a `gen_random_uuid()` surrogate
-- that the platform never renumbers, so the default `NO ACTION` is the accurate statement.
--
-- ═══ VALIDATING, NOT `NOT VALID` ════════════════════════════════════════════════════════════
--
-- 0018 decided this and its reasoning is unchanged: sqlx runs a migration file in ONE
-- transaction and `persist_apply_one` (`wave.sh:1264`) wraps the psql path in `BEGIN … COMMIT`,
-- so both halves of a `NOT VALID` + `VALIDATE` split would hold their locks to the same COMMIT
-- and buy nothing. Splitting the VALIDATE into a LATER file would buy something — a release in
-- which the constraint is recorded but existing rows were never checked, which is exactly the
-- window that would hide the pre-existing orphans the cleanup below exists to find. The scan is
-- three rows of `matches` and two of `server_statuses` on the live database.
--
-- ═══ NEUTRALISE FIRST, THEN ENFORCE — THE T-555 / 0017 LESSON ═══════════════════════════════
--
-- 0017 created a unique index without deduplicating the rows already there and died on arrival
-- on every populated database. ADDING A FOREIGN KEY TO A TABLE THAT HOLDS ORPHANS FAILS
-- IDENTICALLY, migrations run on boot (`bin/api.rs:26`), and these three columns have been
-- written straight from a request body with no existence check since `0001` — so a dangling
-- pointer is not hypothetical here, it is the designed behaviour of the endpoint up to today.
-- Every `ADD CONSTRAINT` below is therefore preceded by a sweep that makes the offending rows
-- non-offending, in this same transaction, recording what it took.
--
-- MEASURED, not assumed. Read-only census of the live `tbd_reforger`, 2026-07-31:
--     matches                                 3 rows   event_id NOT NULL 0    ORPHANS 0
--                                                      mission_id NOT NULL 3  ORPHANS 0
--     server_statuses                         2 rows   current_match_id NOT NULL 1  ORPHANS 0
-- Zero orphans, so this migration applies cleanly on the operator's next restart. That is a fact
-- about ONE database. Production was not examined and a cleanup that only runs where I could
-- look is not a cleanup, so the sweeps below are written to match zero rows on a clean database
-- and to be correct on a dirty one — and they were proven against a database DELIBERATELY
-- SEEDED WITH ORPHANS (each `ADD CONSTRAINT` was first shown to be REJECTED with 23503 when the
-- sweep above it was removed), not against the clean one.
--
-- QUARANTINE. `public.fk_orphans` is 0018's table (`0018:248`) and this file reuses it rather
-- than minting a second one: same shape, same job, and 0018's own argument against reusing
-- 0010's `url_quarantine` does not apply between these two. All three sweeps here record
-- `action = 'nulled'` — nothing is deleted by this migration.
--
-- ORDER. `matches` is swept before `server_statuses`, because `server_statuses.current_match_id`
-- points at `matches`; no sweep here deletes a row, so none of them can orphan another, but the
-- top-down order is 0018's convention and keeps the file readable as a dependency order.
--
-- IDEMPOTENT. `DROP CONSTRAINT IF EXISTS` before every `ADD`, both inside this file's single
-- transaction, so a replay converges instead of erroring on "constraint already exists" and
-- there is no instant at which a previously-enforced constraint is off. A second run finds
-- nothing to sweep: the first run nulled it and the constraints now prevent new ones.
--
-- ═══ THE SEED REORDER SHIPS WITH THIS FILE, AND IS REQUIRED BY IT ═══════════════════════════
--
-- 0018 named this as an independent blocker and it was real. `seeds/content_golden.sql` had TWO
-- forward references that these constraints turn into hard errors:
--
--   * `server_statuses` set `current_match_id = …f000-000000000003` before `matches` inserted it
--   * `matches` named mission `…c000-000000000001` before `missions` inserted it
--
-- `persist_seed` (`wave.sh:1278`) feeds that file to psql statement-by-statement in AUTOCOMMIT,
-- so `DEFERRABLE INITIALLY DEFERRED` would not have saved it either. The seed is reordered in
-- the same commit as this migration — missions → matches → servers → server_statuses — and
-- **the two changes must land together**: this file without the reorder makes the golden seed
-- unloadable, which breaks every fresh environment (`wave.sh:1284` has an error message for
-- precisely that).
--
-- ═══ WHAT THE GAME-SERVER BRIDGE SEES CHANGE — ANNOUNCE THIS ════════════════════════════════
--
-- For `matches.event_id` / `matches.mission_id` the constraint makes the write FAIL EITHER WAY.
-- A 400 does not save the scoreline T-262 was protecting; what it buys is a LEGIBLE, RETRIABLE
-- failure — the bridge is told which pointer is wrong, and `upsert_match` is idempotent on
-- `source_match_id`, so a corrected re-POST lands the row. Weighed against today's behaviour
-- (200, row stored with a dangling `event_id`, attendance then silently never marked) that is
-- the better failure. It is still a contract change for anything POSTing
-- `/api/v1/ingest/match-results`, and it should be announced rather than slipped in.
--
-- ═══ THE FOURTH ABSTENTION IS STILL OPEN ════════════════════════════════════════════════════
--
-- `fire_missions.event_id` is NOT constrained here. It is written by `handlers/field_tools.rs`
-- (`save_fire`), which has no `foreign_key_error` arm, so a constraint on it today would
-- reintroduce the 500 — for a JWT-authenticated human route (POST /api/v1/fire-missions), where
-- the cost is a lost fire-mission record rather than a lost scoreline. It needs its handler arm
-- first; adding the FK without one is the mistake this file was written to avoid repeating.

-- ═════════════════════════════════════════════════════════════════════════════════════════════
-- §1  matches → events.  Attribution pointer, nulled, never deleted: the scoreline stands on
--     its own and the event it claimed does not exist.
-- ═════════════════════════════════════════════════════════════════════════════════════════════

INSERT INTO public.fk_orphans (table_name, relationship, action, row_data, ticket)
SELECT 'matches', 'matches.event_id -> events.id', 'nulled', to_jsonb(x), 'T-585'
  FROM public.matches x
 WHERE x.event_id IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM public.events p WHERE p.id = x.event_id);

UPDATE public.matches x SET event_id = NULL
 WHERE x.event_id IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM public.events p WHERE p.id = x.event_id);

-- ═════════════════════════════════════════════════════════════════════════════════════════════
-- §2  matches → missions.  Same shape. `mission_id` is the pointer `mark_attendance` joins on
--     (`telemetry.rs`), so a dangling one has been silently costing attendance marks.
-- ═════════════════════════════════════════════════════════════════════════════════════════════

INSERT INTO public.fk_orphans (table_name, relationship, action, row_data, ticket)
SELECT 'matches', 'matches.mission_id -> missions.id', 'nulled', to_jsonb(x), 'T-585'
  FROM public.matches x
 WHERE x.mission_id IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM public.missions p WHERE p.id = x.mission_id);

UPDATE public.matches x SET mission_id = NULL
 WHERE x.mission_id IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM public.missions p WHERE p.id = x.mission_id);

-- ═════════════════════════════════════════════════════════════════════════════════════════════
-- §3  server_statuses → matches.  AFTER §1/§2 by convention; neither of those deletes a
--     `matches` row, so nothing above can have orphaned this one.
--
--     `current_match_id` is three-stated on the wire (absent = keep, present-empty = clear,
--     uuid = set — `parse_uuid_opt`, T-316), and NULL is the state the reader already renders:
--     the handler COALESCEs it out of the JSON entirely, and the idle server in the golden seed
--     carries NULL for exactly that purpose.
-- ═════════════════════════════════════════════════════════════════════════════════════════════

INSERT INTO public.fk_orphans (table_name, relationship, action, row_data, ticket)
SELECT 'server_statuses', 'server_statuses.current_match_id -> matches.id', 'nulled', to_jsonb(x), 'T-585'
  FROM public.server_statuses x
 WHERE x.current_match_id IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM public.matches p WHERE p.id = x.current_match_id);

UPDATE public.server_statuses x SET current_match_id = NULL
 WHERE x.current_match_id IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM public.matches p WHERE p.id = x.current_match_id);

-- ═════════════════════════════════════════════════════════════════════════════════════════════
-- §4  Supporting indexes — NONE, and that is a decision.
--
-- 0018's rule (`0018:523`): index a referencing column only when it is not already the leading
-- column of an existing index AND the referenced parent is hard-deleted somewhere in the crate,
-- because without one Postgres runs the referential-action query as a sequential scan ONCE PER
-- DELETED PARENT ROW.
--
-- Checked one by one rather than assumed:
--   * `events` and `missions` are SOFT-deleted — `delete_event` / the mission delete path write
--     `deleted_at = now()`. `grep -rniE 'delete +from +(events|missions)'` over
--     `apps/website/api/src/` returns nothing. Their `ON DELETE SET NULL` is not on any path.
--   * `matches` is hard-deleted in `apps/website/api/tests/` cleanup only — never in `src/`.
--     The child there is `server_statuses`, which is bounded by the number of registered servers
--     (three on the live database, one row per server by PK), so the scan is a three-row seq
--     scan on a table Postgres holds in one page. An index would be write cost on the hottest
--     write in the system (every heartbeat) to save nothing measurable.
-- If a match-purge endpoint ever ships, revisit `server_statuses.current_match_id`.
-- ═════════════════════════════════════════════════════════════════════════════════════════════

-- ═════════════════════════════════════════════════════════════════════════════════════════════
-- §5  The constraints.  DROP IF EXISTS + ADD, both inside this file's single transaction.
--     THE NAMES ARE THE CONTRACT WITH `telemetry.rs:46-48` — see the header before editing one.
-- ═════════════════════════════════════════════════════════════════════════════════════════════

ALTER TABLE public.matches DROP CONSTRAINT IF EXISTS matches_event_id_fkey;
ALTER TABLE public.matches ADD CONSTRAINT matches_event_id_fkey
    FOREIGN KEY (event_id) REFERENCES public.events(id) ON DELETE SET NULL;

ALTER TABLE public.matches DROP CONSTRAINT IF EXISTS matches_mission_id_fkey;
ALTER TABLE public.matches ADD CONSTRAINT matches_mission_id_fkey
    FOREIGN KEY (mission_id) REFERENCES public.missions(id) ON DELETE SET NULL;

ALTER TABLE public.server_statuses DROP CONSTRAINT IF EXISTS server_statuses_current_match_id_fkey;
ALTER TABLE public.server_statuses ADD CONSTRAINT server_statuses_current_match_id_fkey
    FOREIGN KEY (current_match_id) REFERENCES public.matches(id) ON DELETE SET NULL;
