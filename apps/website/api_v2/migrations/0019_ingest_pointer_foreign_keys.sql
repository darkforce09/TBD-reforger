-- The three ingest-pointer foreign keys 0018 abstained from.
--
-- 0018 constrained 25 relationships and deliberately left four alone (abstention iv):
-- `matches.event_id`, `matches.mission_id`, `server_statuses.current_match_id`,
-- `fire_missions.event_id` — pointers taken verbatim from a request body with no existence
-- check. Its reason was not that the constraint is wrong. It was that, without a SQLSTATE 23503
-- handler, a violation would reach a no-human-in-the-loop game-server bridge as a **500** and
-- cost the whole scoreline.
--
-- `match_telemetry::handlers::ingest_parsing::foreign_key_error` removes that reason: it maps a
-- 23503 onto a 400 that names the pointer the database could not find, and it is armed for
-- these three constraint names (`FK_MATCH_EVENT`, `FK_MATCH_MISSION`, `FK_STATUS_MATCH`).
--
-- All three columns are nullable, which is what makes `ON DELETE SET NULL` expressible at all.
--
-- ═══ THE NAMES ARE A CONTRACT, NOT A STYLE CHOICE ═══════════════════════════════════════════
--
-- `foreign_key_error` branches on the constraint name as a literal string. A constraint that
-- does the same job under any other name matches NO arm, falls through to a plain error, and
-- **silently restores the exact 500 this file exists to prevent** — with the FK now in force, so
-- the write fails too. That failure mode is invisible: the schema looks right, the handler looks
-- right, and only a live 23503 shows it.
--
-- So the three names below are the constants in `ingest_parsing.rs`, character for character:
--
--     server_statuses_current_match_id_fkey
--     matches_event_id_fkey
--     matches_mission_id_fkey
--
-- They are also what Postgres would have auto-named them (`<table>_<column>_fkey`) and what all
-- 25 of 0018's constraints use, so the convention and the contract agree. The handler's sibling
-- test `fk_constant_names_follow_migration_convention` pins the convention from the other side.
-- **If you rename a constraint here you must edit those constants in the same commit.**
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
-- transaction, so both halves of a `NOT VALID` + `VALIDATE` split would hold their locks to the
-- same COMMIT and buy nothing. Splitting the VALIDATE into a LATER file would buy something — a
-- release in which the constraint is recorded but existing rows were never checked, which is
-- exactly the window that would hide the pre-existing orphans the cleanup below exists to find.
-- The scan is a handful of rows in `matches` and `server_statuses`.
--
-- ═══ NEUTRALISE FIRST, THEN ENFORCE ═════════════════════════════════════════════════════════
--
-- ADDING A FOREIGN KEY TO A TABLE THAT HOLDS ORPHANS FAILS, migrations run on boot, and these
-- three columns have been written straight from a request body with no existence check since
-- `0001` — so a dangling pointer is not hypothetical here, it is the designed behaviour of the
-- endpoint up to this file. Every `ADD CONSTRAINT` below is therefore preceded by a sweep that
-- makes the offending rows non-offending, in this same transaction, recording what it took.
--
-- The sweeps match zero rows on a clean database and are correct on a dirty one: they were
-- proven against a database DELIBERATELY SEEDED WITH ORPHANS (each `ADD CONSTRAINT` was first
-- shown to be REJECTED with 23503 when the sweep above it was removed), not only against a
-- clean census.
--
-- QUARANTINE. `public.fk_orphans` is 0018's table and this file reuses it rather than minting a
-- second one: same shape, same job, and 0018's own argument against reusing 0010's
-- `url_quarantine` does not apply between these two. All three sweeps here record
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
-- ═══ THE SEED ORDER IS PART OF THIS FILE ════════════════════════════════════════════════════
--
-- `seeds/content_golden.sql` inserts missions before matches and servers before server_statuses,
-- because these constraints turn a forward reference into a hard error: the seed is fed to psql
-- statement by statement in autocommit, where `DEFERRABLE INITIALLY DEFERRED` cannot help. This
-- file without that order would make the golden seed unloadable on every fresh environment.
--
-- ═══ WHAT THE GAME-SERVER BRIDGE SEES CHANGE ════════════════════════════════════════════════
--
-- For `matches.event_id` / `matches.mission_id` the constraint makes the write FAIL EITHER WAY.
-- A 400 does not save the scoreline the abstention was protecting; what it buys is a LEGIBLE,
-- RETRIABLE failure — the bridge is told which pointer is wrong, and `upsert_match` is idempotent
-- on `source_match_id`, so a corrected re-POST lands the row. Weighed against the earlier
-- behaviour (200, row stored with a dangling `event_id`, attendance then silently never marked)
-- that is the better failure. It is still a contract change for anything POSTing
-- `/api/v1/ingest/match-results`.
--
-- ═══ THE FOURTH ABSTENTION STAYS ════════════════════════════════════════════════════════════
--
-- `fire_missions.event_id` is NOT constrained here. It is written by
-- `operations::handlers::fire_missions::save_fire`, which has no `foreign_key_error` arm, so a
-- constraint on it would reintroduce the 500 — for a JWT-authenticated human route
-- (POST /api/v1/fire-missions), where the cost is a lost fire-mission record rather than a lost
-- scoreline. It needs its handler arm first; adding the FK without one is the mistake this file
-- was written to avoid repeating.

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
-- §2  matches → missions.  Same shape. `mission_id` is the pointer attendance attribution joins
--     on (`match_telemetry::handlers::attendance_attribution`), so a dangling one silently costs
--     attendance marks.
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
--     uuid = set — `parse_uuid_opt`), and NULL is the state the reader already renders:
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
--     `deleted_at = now()`. `grep -rniE 'delete +from +(events|missions)'` over `src/` returns
--     nothing. Their `ON DELETE SET NULL` is not on any path.
--   * `matches` is hard-deleted in the integration suites' cleanup only — never in `src/`.
--     The child there is `server_statuses`, which is bounded by the number of registered servers
--     (three on the live database, one row per server by PK), so the scan is a three-row seq
--     scan on a table Postgres holds in one page. An index would be write cost on the hottest
--     write in the system (every heartbeat) to save nothing measurable.
-- If a match-purge endpoint ever ships, revisit `server_statuses.current_match_id`.
-- ═════════════════════════════════════════════════════════════════════════════════════════════

-- ═════════════════════════════════════════════════════════════════════════════════════════════
-- §5  The constraints.  DROP IF EXISTS + ADD, both inside this file's single transaction.
--     THE NAMES ARE THE CONTRACT WITH `ingest_parsing.rs` — see the header before editing one.
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
