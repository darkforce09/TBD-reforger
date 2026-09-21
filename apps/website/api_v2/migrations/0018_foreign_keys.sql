-- The schema's foreign keys.
--
-- Until this file, every relationship below was unconstrained free text or a bare uuid. What
-- that cost, measured in the tree rather than asserted:
--
--   * `remove_event_mission` deletes `event_registrations` and `orbat_slots` for the detached
--     mission and does not touch `orbat_reservations`. Every mission detached from an event
--     left its squad reservations behind, keyed to an `event_mission_id` that no longer exists.
--     Constraint 4 below fixes that with no handler edit.
--   * `missions.current_version_id` could name a version row that does not exist, and the
--     event dossier answered that with a 500. Constraint 18.
--   * A server row could not be purged safely: removing it means deleting the two dependent
--     rows in one transaction, which belongs with the migration that adds the missing
--     `ON DELETE` foreign keys. Constraints 10 and 11 are that migration.
--
-- ── WHAT THIS FILE DELIBERATELY DOES NOT DO ─────────────────────────────────────────────────
--
-- It does not make `DELETE /api/v1/events/:id` cascade. That handler (`delete_event`) writes
-- `deleted_at = now()` and nothing else. The FK that would make a hard delete cascade —
-- `event_missions.event_id … ON DELETE CASCADE` — is constraint 1, so the day that handler is
-- changed to a hard delete the cascade is already in place and correct.
--
-- ═══ THE INVENTORY ═══════════════════════════════════════════════════════════════════════════
--
-- Three tiers, and the tier decides ON DELETE. Twenty-five constraints; twenty-one candidate
-- relationships were examined and deliberately left unconstrained (see THE ABSTENTIONS).
--
-- A. OWNERSHIP → CASCADE. The child is a *part* of the parent and is unreachable by any read
--    path in the crate once the parent is gone. An orphan here is not data, it is litter.
--
--      1  event_missions.event_id             → events(id)
--      2  event_registrations.event_mission_id→ event_missions(id)
--      3  orbat_slots.event_mission_id        → event_missions(id)
--      4  orbat_reservations.event_mission_id → event_missions(id)
--      5  mission_versions.mission_id         → missions(id)
--      6  mission_armories.mission_id         → missions(id)
--      7  mission_bookmarks.mission_id        → missions(id)
--      8  modpack_mods.modpack_id             → modpacks(id)
--      9  registry_compat.modpack_id          → modpacks(id)
--     10  server_statuses.server_id           → servers(id)
--     11  server_status_histories.server_id   → servers(id)
--     12  match_player_stats.match_id         → matches(id)
--
--    2/3/4 are exactly what `remove_event_mission` does by hand, minus the bug: it deletes the
--    first two and forgets the third. 8 is what `delete_modpack` does by hand. Where the handler
--    already cascades, the constraint agrees with it; where it forgets, the constraint finishes
--    the job.
--
-- B. REFERENCE → RESTRICT. A pointer between two independently-owned entities, where the crate
--    ALREADY refuses the delete in a handler and answers 409. The constraint is the same answer
--    one layer down, for the paths that do not go through the handler (psql, a future admin
--    tool, a script).
--
--     13  event_missions.mission_id           → missions(id)
--            `delete_mission` counts `event_missions` and 409s if any exist.
--     14  registry_items.modpack_id           → modpacks(id)
--     15  servers.required_modpack_id         → modpacks(id)
--     16  events.modpack_id                   → modpacks(id)
--            `delete_modpack` counts all three of the above and 409s.
--     17  events.server_id                    → servers(id)
--            `deactivate_server` never hard-deletes, so this can only fire against
--            a manual DELETE — which is precisely the caller the handler cannot reach.
--
--    RESTRICT rather than the NO ACTION default on purpose: NO ACTION defers its check to the
--    end of the statement and can be made DEFERRABLE, which is the right choice for a
--    constraint you expect legitimate code to violate transiently. Nothing in this crate does
--    that on these five — every handler deletes children first — so the immediate check is
--    free, and it fails at the offending row rather than at COMMIT, which is a better error.
--
-- C. NULLABLE POINTER → SET NULL. The referencing row survives its target; the pointer does not.
--
--     18  missions.current_version_id         → mission_versions(id)
--            A mission with no published version is a legal, handled state (409 "publish a
--            version"); a mission naming a version that is not there is not. SET NULL converts
--            the second into the first.
--     19  event_registrations.slot_id         → orbat_slots(id)
--            This column and `orbat_slots.assigned_to` are a denormalised pair. When a seat row
--            goes, the registration stays (the person is still registered) and becomes
--            bench/waitlist-shaped, which is a state the readers already handle. CASCADE here
--            would silently unregister people when a leader re-materialises an ORBAT.
--
-- D. IDENTITY. The association columns `assigned_to`, `discord_id`, `reserved_by`, plus the
--    three other tables of the same shape. THE RULE, and it is narrow on purpose:
--
--        constrain a discord_id ONLY when the row is a pure ASSOCIATION — its whole content is
--        "this user ↔ this thing" and it is written by an authenticated session about itself.
--
--     20  event_registrations.discord_id      → users(discord_id)   CASCADE
--     21  orbat_slots.assigned_to             → users(discord_id)   SET NULL
--     22  orbat_reservations.reserved_by      → users(discord_id)   CASCADE
--     23  refresh_tokens.discord_id           → users(discord_id)   CASCADE
--     24  user_discord_roles.discord_id       → users(discord_id)   CASCADE
--     25  mission_bookmarks.discord_id        → users(discord_id)   CASCADE
--
--    21 is SET NULL and not CASCADE because a NULL `assigned_to` is not a missing row, it is a
--    FREE SEAT — 0017 says so in as many words and its partial unique index depends on it. The
--    seat outlives its occupant; deleting it would silently shrink the ORBAT.
--    23 is the one with a security consequence: a refresh token outliving its user is a live
--    credential for an account that no longer exists.
--
--    Nothing in the crate deletes a `users` row — there is no `DELETE FROM users`, and nothing
--    writes `users.deleted_at` either, though readers check it. So these six ON DELETE actions
--    are unreachable through the API as it stands; their value today is the INSERT-side check,
--    and their value tomorrow is that the first user-deletion feature to be written cannot ship
--    the orphan bug. Stated rather than implied, because "the cascade is tested" and "the
--    cascade is reachable from a route" are different claims and only the first is true.
--
-- ═══ THE ABSTENTIONS — twenty-one relationships examined and left alone ══════════════════════
--
-- Every constraint is a new way for a legitimate write to fail, so each of these is a decision,
-- not an oversight.
--
-- (i) ACTOR AND AUTHORSHIP STAMPS — eleven columns, all → users(discord_id):
--       audit_logs.actor_id, announcements.author_id, missions.author_id, missions.reviewed_by,
--       mission_versions.created_by, events.created_by, warnings.issued_by,
--       leave_requests.reviewed_by, wiki_pages.updated_by, users.banned_by,
--       fire_missions.created_by
--     These record WHO DID SOMETHING. An audit line, a warning, a rejection reason must outlive
--     the person named in it — that is the entire point of keeping them — so the only ON DELETE
--     that is not actively wrong is NO ACTION, and a constraint whose delete behaviour is "refuse
--     everything" buys nothing here: every one of these columns is written from an authenticated
--     extractor, so the referent already exists by construction. Eleven constraints for zero
--     delete semantics and zero new guarantees is over-constraining.
--
-- (ii) DOCUMENTS OWNED BY A USER — user_factions.owner_id, leave_requests.discord_id,
--     warnings.discord_id. These are association-shaped but carry a BODY (a faction library, a
--     leave request, a disciplinary record). "The user is gone, so cascade the record away" is a
--     policy decision with review consequences, and it should be made by whoever writes the
--     user-deletion feature, in daylight, not inherited from a migration that had no user
--     deletion to reason about.
--
-- (iii) EXTERNAL IDENTITY — match_player_stats.discord_id, identity_link_codes.discord_id.
--     `discord_id` here is a Discord snowflake that may not correspond to a local user yet. That
--     is the designed state, not a defect: the leaderboard filters with
--     `WHERE discord_id IS NOT NULL`, link-confirm exists to *claim* those rows later, and
--     `identity_link_codes` is by definition minted before the identity is bound.
--
-- (iv) POINTERS TAKEN VERBATIM FROM A REQUEST BODY WITH NO EXISTENCE CHECK — matches.event_id,
--     matches.mission_id, server_statuses.current_match_id, fire_missions.event_id.
--     Constraining one of these needs a handler that maps SQLSTATE 23503 onto a 400 naming the
--     pointer; without it a violation reaches the client as a **500**, and on the match ingest —
--     an endpoint with no human in the loop — "one attribution pointer is wrong" becomes "the
--     entire scoreline is lost". The match row itself is valuable; only the pointer is bad. 0019
--     adds the first three once `match_telemetry::handlers::ingest_parsing::foreign_key_error`
--     exists; `fire_missions.event_id` waits for the same arm in its own handler.
--
--     `server_statuses.server_id` (constraint 10) is the one column of this shape that IS
--     constrained, and the difference is what the unconstrained write produces. A heartbeat for
--     an unknown server writes a `server_statuses` row that `list_servers` — which drives off
--     `servers` — can never read: garbage, invisible, accumulating forever. Failing loudly beats
--     writing that.
--
-- (v) user_discord_roles.discord_role_id → discord_roles. NOT constrained, and this one would
--     break production login. `identity_and_access::services::discord_role_sync` stores every
--     snowflake Discord returns, and its own doc comment says why: unmapped ids are still stored
--     so a later admin mapping + resync promotes them. An unmapped role HAS NO `discord_roles`
--     ROW. The FK would make `sync_roles` fail for any member holding a role the admin has not
--     mapped yet — i.e. the OAuth callback, for most of the guild.
--
-- ═══ ON DELETE ONLY — NO `ON UPDATE` ════════════════════════════════════════════════════════
--
-- Every parent key here is immutable: `gen_random_uuid()` surrogate ids, and `users.discord_id`,
-- which is a Discord snowflake the platform does not mint and cannot renumber. The default
-- `ON UPDATE NO ACTION` is therefore the accurate statement. CASCADE would be dead code that
-- reads as though the ids move.
--
-- ═══ VALIDATING, NOT `NOT VALID` — DECIDED, NOT DEFAULTED ═══════════════════════════════════
--
-- A `NOT VALID` + later `VALIDATE CONSTRAINT` split exists to avoid holding a lock while a huge
-- child table is scanned. It cannot help here and would cost something real:
--
--   * sqlx runs one migration file in ONE transaction. Both halves of a split inside this file
--     would hold their locks to the same COMMIT, so the split buys exactly nothing unless the
--     VALIDATE lands in a LATER migration file.
--   * Putting the VALIDATE in a later file ships a window — one release, maybe more — in which
--     the constraint is recorded but existing rows were never checked. `NOT VALID` still
--     enforces on new writes, so the window would hide precisely the pre-existing orphans this
--     migration exists to find, and the cleanup below would be running blind.
--   * The scan is small. The largest child here is `server_status_histories`, an append-only
--     telemetry table with one row per heartbeat per server; at the observed shape (three
--     servers) it is a single sequential scan measured in milliseconds on anything this platform
--     has produced. Every other child is bounded by content the operator authors by hand.
--
-- ═══ NEUTRALISE FIRST, THEN ENFORCE ═════════════════════════════════════════════════════════
--
-- ADDING A FOREIGN KEY TO A TABLE THAT HOLDS ORPHANS FAILS, and migrations run on boot, so that
-- is a dead API rather than a failed deploy step. So the shape is 0010's and 0017's: make the
-- offending rows non-offending FIRST, in the same transaction, recording what was taken, then
-- apply the DDL.
--
-- The steps below match zero rows on a clean database and are correct on a dirty one: they were
-- proven against a database DELIBERATELY SEEDED WITH ORPHANS (each `ADD CONSTRAINT` was first
-- shown to be REJECTED with 23503 when the sweep above it was removed), not only against a clean
-- census.
--
-- WHY A NEW QUARANTINE TABLE AND NOT 0010's `url_quarantine`. 0010 states the reuse argument
-- ("one table beats four") and 0017 took it. It does not stretch to this job: `url_quarantine`
-- keys a capture as (row_id uuid, original_value text), and half the rows here have no uuid at
-- all — `mission_bookmarks` and `user_discord_roles` are keyed on a composite of two text/uuid
-- columns, `server_status_histories` on a bigint. More importantly this migration DELETES WHOLE
-- ROWS, not single column values, so "the original value" is the entire tuple. `fk_orphans`
-- stores it as jsonb, which loses nothing and makes the undo one `jsonb_populate_record`.
--
-- ORDER IS LOAD-BEARING AND IS TOP-DOWN. Deleting an orphan `event_missions` row creates orphan
-- `orbat_slots`. So parents are cleaned before children, and each child sweep therefore also
-- catches whatever the sweep above it just orphaned. Read the section numbers as a dependency
-- order, not a list.
--
-- IDEMPOTENT. Every `ADD CONSTRAINT` is preceded by `DROP CONSTRAINT IF EXISTS`, so a replay
-- converges on the same end state inside the one transaction rather than erroring on
-- "constraint already exists" — and because both statements are in that transaction there is no
-- instant at which a previously-enforced constraint is off. The cleanup steps find nothing on a
-- second run (the first run removed the rows, and the constraints now prevent new ones), and
-- `CREATE TABLE IF NOT EXISTS` / `CREATE INDEX IF NOT EXISTS` are no-ops when already in force.

-- ═════════════════════════════════════════════════════════════════════════════════════════════
-- §0  Quarantine
-- ═════════════════════════════════════════════════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS public.fk_orphans (
    id             bigserial PRIMARY KEY,
    table_name     text NOT NULL,
    relationship   text NOT NULL,   -- 'child.col -> parent.col', the constraint this was blocking
    action         text NOT NULL,   -- 'deleted' | 'nulled'
    row_data       jsonb NOT NULL,  -- the whole row as it stood, so the undo needs nothing else
    ticket         text NOT NULL,
    quarantined_at timestamptz NOT NULL DEFAULT now()
);

COMMENT ON TABLE public.fk_orphans IS
    'T-262. Rows removed or de-pointed so the schema''s first foreign keys could be applied. '
    'row_data is the complete tuple; restore with jsonb_populate_record(NULL::<table>, row_data).';

-- ═════════════════════════════════════════════════════════════════════════════════════════════
-- §1  Events → event_missions.  Cleaned first: everything in §2 hangs off event_missions.
-- ═════════════════════════════════════════════════════════════════════════════════════════════

INSERT INTO public.fk_orphans (table_name, relationship, action, row_data, ticket)
SELECT 'event_missions', 'event_missions.event_id -> events.id', 'deleted', to_jsonb(x), 'T-262'
  FROM public.event_missions x
 WHERE NOT EXISTS (SELECT 1 FROM public.events p WHERE p.id = x.event_id);

DELETE FROM public.event_missions x
 WHERE NOT EXISTS (SELECT 1 FROM public.events p WHERE p.id = x.event_id);

INSERT INTO public.fk_orphans (table_name, relationship, action, row_data, ticket)
SELECT 'event_missions', 'event_missions.mission_id -> missions.id', 'deleted', to_jsonb(x), 'T-262'
  FROM public.event_missions x
 WHERE NOT EXISTS (SELECT 1 FROM public.missions p WHERE p.id = x.mission_id);

DELETE FROM public.event_missions x
 WHERE NOT EXISTS (SELECT 1 FROM public.missions p WHERE p.id = x.mission_id);

-- ═════════════════════════════════════════════════════════════════════════════════════════════
-- §2  event_missions → seats, reservations, registrations.  Also sweeps up whatever §1 orphaned.
-- ═════════════════════════════════════════════════════════════════════════════════════════════

INSERT INTO public.fk_orphans (table_name, relationship, action, row_data, ticket)
SELECT 'orbat_slots', 'orbat_slots.event_mission_id -> event_missions.id', 'deleted', to_jsonb(x), 'T-262'
  FROM public.orbat_slots x
 WHERE NOT EXISTS (SELECT 1 FROM public.event_missions p WHERE p.id = x.event_mission_id);

DELETE FROM public.orbat_slots x
 WHERE NOT EXISTS (SELECT 1 FROM public.event_missions p WHERE p.id = x.event_mission_id);

-- The rows `remove_event_mission` forgets. On any database where a mission has ever been
-- detached from an event, this is the step that finds something.
INSERT INTO public.fk_orphans (table_name, relationship, action, row_data, ticket)
SELECT 'orbat_reservations', 'orbat_reservations.event_mission_id -> event_missions.id', 'deleted', to_jsonb(x), 'T-262'
  FROM public.orbat_reservations x
 WHERE NOT EXISTS (SELECT 1 FROM public.event_missions p WHERE p.id = x.event_mission_id);

DELETE FROM public.orbat_reservations x
 WHERE NOT EXISTS (SELECT 1 FROM public.event_missions p WHERE p.id = x.event_mission_id);

INSERT INTO public.fk_orphans (table_name, relationship, action, row_data, ticket)
SELECT 'event_registrations', 'event_registrations.event_mission_id -> event_missions.id', 'deleted', to_jsonb(x), 'T-262'
  FROM public.event_registrations x
 WHERE NOT EXISTS (SELECT 1 FROM public.event_missions p WHERE p.id = x.event_mission_id);

DELETE FROM public.event_registrations x
 WHERE NOT EXISTS (SELECT 1 FROM public.event_missions p WHERE p.id = x.event_mission_id);

-- Registrations naming a seat that is gone — including seats this file just deleted. NULLed,
-- never deleted: the person is still registered, they are simply no longer seated, which is the
-- bench state the readers already render.
INSERT INTO public.fk_orphans (table_name, relationship, action, row_data, ticket)
SELECT 'event_registrations', 'event_registrations.slot_id -> orbat_slots.id', 'nulled', to_jsonb(x), 'T-262'
  FROM public.event_registrations x
 WHERE x.slot_id IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM public.orbat_slots p WHERE p.id = x.slot_id);

UPDATE public.event_registrations x SET slot_id = NULL
 WHERE x.slot_id IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM public.orbat_slots p WHERE p.id = x.slot_id);

-- ═════════════════════════════════════════════════════════════════════════════════════════════
-- §3  Missions → versions, armories, bookmarks; then the mission's own version pointer.
-- ═════════════════════════════════════════════════════════════════════════════════════════════

INSERT INTO public.fk_orphans (table_name, relationship, action, row_data, ticket)
SELECT 'mission_versions', 'mission_versions.mission_id -> missions.id', 'deleted', to_jsonb(x), 'T-262'
  FROM public.mission_versions x
 WHERE NOT EXISTS (SELECT 1 FROM public.missions p WHERE p.id = x.mission_id);

DELETE FROM public.mission_versions x
 WHERE NOT EXISTS (SELECT 1 FROM public.missions p WHERE p.id = x.mission_id);

INSERT INTO public.fk_orphans (table_name, relationship, action, row_data, ticket)
SELECT 'mission_armories', 'mission_armories.mission_id -> missions.id', 'deleted', to_jsonb(x), 'T-262'
  FROM public.mission_armories x
 WHERE NOT EXISTS (SELECT 1 FROM public.missions p WHERE p.id = x.mission_id);

DELETE FROM public.mission_armories x
 WHERE NOT EXISTS (SELECT 1 FROM public.missions p WHERE p.id = x.mission_id);

INSERT INTO public.fk_orphans (table_name, relationship, action, row_data, ticket)
SELECT 'mission_bookmarks', 'mission_bookmarks.mission_id -> missions.id', 'deleted', to_jsonb(x), 'T-262'
  FROM public.mission_bookmarks x
 WHERE NOT EXISTS (SELECT 1 FROM public.missions p WHERE p.id = x.mission_id);

DELETE FROM public.mission_bookmarks x
 WHERE NOT EXISTS (SELECT 1 FROM public.missions p WHERE p.id = x.mission_id);

-- AFTER the mission_versions sweep above, so it also catches a pointer this file just broke.
-- This is the event dossier's 500 — a mission naming a version that is not there.
INSERT INTO public.fk_orphans (table_name, relationship, action, row_data, ticket)
SELECT 'missions', 'missions.current_version_id -> mission_versions.id', 'nulled', to_jsonb(x), 'T-262'
  FROM public.missions x
 WHERE x.current_version_id IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM public.mission_versions p WHERE p.id = x.current_version_id);

UPDATE public.missions x SET current_version_id = NULL
 WHERE x.current_version_id IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM public.mission_versions p WHERE p.id = x.current_version_id);

-- ═════════════════════════════════════════════════════════════════════════════════════════════
-- §4  Modpacks → mods, compat edges, registry items; and the two pointers at modpacks.
-- ═════════════════════════════════════════════════════════════════════════════════════════════

INSERT INTO public.fk_orphans (table_name, relationship, action, row_data, ticket)
SELECT 'modpack_mods', 'modpack_mods.modpack_id -> modpacks.id', 'deleted', to_jsonb(x), 'T-262'
  FROM public.modpack_mods x
 WHERE NOT EXISTS (SELECT 1 FROM public.modpacks p WHERE p.id = x.modpack_id);

DELETE FROM public.modpack_mods x
 WHERE NOT EXISTS (SELECT 1 FROM public.modpacks p WHERE p.id = x.modpack_id);

INSERT INTO public.fk_orphans (table_name, relationship, action, row_data, ticket)
SELECT 'registry_compat', 'registry_compat.modpack_id -> modpacks.id', 'deleted', to_jsonb(x), 'T-262'
  FROM public.registry_compat x
 WHERE NOT EXISTS (SELECT 1 FROM public.modpacks p WHERE p.id = x.modpack_id);

DELETE FROM public.registry_compat x
 WHERE NOT EXISTS (SELECT 1 FROM public.modpacks p WHERE p.id = x.modpack_id);

INSERT INTO public.fk_orphans (table_name, relationship, action, row_data, ticket)
SELECT 'registry_items', 'registry_items.modpack_id -> modpacks.id', 'deleted', to_jsonb(x), 'T-262'
  FROM public.registry_items x
 WHERE NOT EXISTS (SELECT 1 FROM public.modpacks p WHERE p.id = x.modpack_id);

DELETE FROM public.registry_items x
 WHERE NOT EXISTS (SELECT 1 FROM public.modpacks p WHERE p.id = x.modpack_id);

-- Both nullable, so the repair is to drop the pointer rather than the row. A server or an event
-- with no modpack binding is a state both already support (`0011` ships both columns NULL).
INSERT INTO public.fk_orphans (table_name, relationship, action, row_data, ticket)
SELECT 'servers', 'servers.required_modpack_id -> modpacks.id', 'nulled', to_jsonb(x), 'T-262'
  FROM public.servers x
 WHERE x.required_modpack_id IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM public.modpacks p WHERE p.id = x.required_modpack_id);

UPDATE public.servers x SET required_modpack_id = NULL
 WHERE x.required_modpack_id IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM public.modpacks p WHERE p.id = x.required_modpack_id);

INSERT INTO public.fk_orphans (table_name, relationship, action, row_data, ticket)
SELECT 'events', 'events.modpack_id -> modpacks.id', 'nulled', to_jsonb(x), 'T-262'
  FROM public.events x
 WHERE x.modpack_id IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM public.modpacks p WHERE p.id = x.modpack_id);

UPDATE public.events x SET modpack_id = NULL
 WHERE x.modpack_id IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM public.modpacks p WHERE p.id = x.modpack_id);

-- ═════════════════════════════════════════════════════════════════════════════════════════════
-- §5  Servers → statuses, history; and the event's server pointer.
-- ═════════════════════════════════════════════════════════════════════════════════════════════

-- Rows a heartbeat from a host whose server row is gone leaves behind: invisible to every read
-- endpoint, accumulating.
INSERT INTO public.fk_orphans (table_name, relationship, action, row_data, ticket)
SELECT 'server_statuses', 'server_statuses.server_id -> servers.id', 'deleted', to_jsonb(x), 'T-262'
  FROM public.server_statuses x
 WHERE NOT EXISTS (SELECT 1 FROM public.servers p WHERE p.id = x.server_id);

DELETE FROM public.server_statuses x
 WHERE NOT EXISTS (SELECT 1 FROM public.servers p WHERE p.id = x.server_id);

INSERT INTO public.fk_orphans (table_name, relationship, action, row_data, ticket)
SELECT 'server_status_histories', 'server_status_histories.server_id -> servers.id', 'deleted', to_jsonb(x), 'T-262'
  FROM public.server_status_histories x
 WHERE NOT EXISTS (SELECT 1 FROM public.servers p WHERE p.id = x.server_id);

DELETE FROM public.server_status_histories x
 WHERE NOT EXISTS (SELECT 1 FROM public.servers p WHERE p.id = x.server_id);

INSERT INTO public.fk_orphans (table_name, relationship, action, row_data, ticket)
SELECT 'events', 'events.server_id -> servers.id', 'nulled', to_jsonb(x), 'T-262'
  FROM public.events x
 WHERE x.server_id IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM public.servers p WHERE p.id = x.server_id);

UPDATE public.events x SET server_id = NULL
 WHERE x.server_id IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM public.servers p WHERE p.id = x.server_id);

-- ═════════════════════════════════════════════════════════════════════════════════════════════
-- §6  Matches → per-player stats.
-- ═════════════════════════════════════════════════════════════════════════════════════════════

INSERT INTO public.fk_orphans (table_name, relationship, action, row_data, ticket)
SELECT 'match_player_stats', 'match_player_stats.match_id -> matches.id', 'deleted', to_jsonb(x), 'T-262'
  FROM public.match_player_stats x
 WHERE NOT EXISTS (SELECT 1 FROM public.matches p WHERE p.id = x.match_id);

DELETE FROM public.match_player_stats x
 WHERE NOT EXISTS (SELECT 1 FROM public.matches p WHERE p.id = x.match_id);

-- ═════════════════════════════════════════════════════════════════════════════════════════════
-- §7  Users → the six association tables.  Last, because §1-§2 may already have removed some.
-- ═════════════════════════════════════════════════════════════════════════════════════════════

-- Free the seat, do not delete it — 0017's rule, and the reason its partial unique index is
-- `WHERE assigned_to IS NOT NULL`. `assigned_at` goes with it: a timestamp on an unoccupied seat
-- is a state no reader expects, and seeds/content_golden.sql carries both NULL on every free seat.
INSERT INTO public.fk_orphans (table_name, relationship, action, row_data, ticket)
SELECT 'orbat_slots', 'orbat_slots.assigned_to -> users.discord_id', 'nulled', to_jsonb(x), 'T-262'
  FROM public.orbat_slots x
 WHERE x.assigned_to IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM public.users p WHERE p.discord_id = x.assigned_to);

UPDATE public.orbat_slots x SET assigned_to = NULL, assigned_at = NULL
 WHERE x.assigned_to IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM public.users p WHERE p.discord_id = x.assigned_to);

INSERT INTO public.fk_orphans (table_name, relationship, action, row_data, ticket)
SELECT 'event_registrations', 'event_registrations.discord_id -> users.discord_id', 'deleted', to_jsonb(x), 'T-262'
  FROM public.event_registrations x
 WHERE NOT EXISTS (SELECT 1 FROM public.users p WHERE p.discord_id = x.discord_id);

DELETE FROM public.event_registrations x
 WHERE NOT EXISTS (SELECT 1 FROM public.users p WHERE p.discord_id = x.discord_id);

INSERT INTO public.fk_orphans (table_name, relationship, action, row_data, ticket)
SELECT 'orbat_reservations', 'orbat_reservations.reserved_by -> users.discord_id', 'deleted', to_jsonb(x), 'T-262'
  FROM public.orbat_reservations x
 WHERE NOT EXISTS (SELECT 1 FROM public.users p WHERE p.discord_id = x.reserved_by);

DELETE FROM public.orbat_reservations x
 WHERE NOT EXISTS (SELECT 1 FROM public.users p WHERE p.discord_id = x.reserved_by);

-- `token_hash` is stripped from the capture. It is the credential; quarantining it would move a
-- live secret from a table nothing selects into a table an incident review reads out loud. The
-- rest of the row is enough to say who held a token and when it would have expired.
INSERT INTO public.fk_orphans (table_name, relationship, action, row_data, ticket)
SELECT 'refresh_tokens', 'refresh_tokens.discord_id -> users.discord_id', 'deleted',
       to_jsonb(x) - 'token_hash', 'T-262'
  FROM public.refresh_tokens x
 WHERE NOT EXISTS (SELECT 1 FROM public.users p WHERE p.discord_id = x.discord_id);

DELETE FROM public.refresh_tokens x
 WHERE NOT EXISTS (SELECT 1 FROM public.users p WHERE p.discord_id = x.discord_id);

INSERT INTO public.fk_orphans (table_name, relationship, action, row_data, ticket)
SELECT 'user_discord_roles', 'user_discord_roles.discord_id -> users.discord_id', 'deleted', to_jsonb(x), 'T-262'
  FROM public.user_discord_roles x
 WHERE NOT EXISTS (SELECT 1 FROM public.users p WHERE p.discord_id = x.discord_id);

DELETE FROM public.user_discord_roles x
 WHERE NOT EXISTS (SELECT 1 FROM public.users p WHERE p.discord_id = x.discord_id);

INSERT INTO public.fk_orphans (table_name, relationship, action, row_data, ticket)
SELECT 'mission_bookmarks', 'mission_bookmarks.discord_id -> users.discord_id', 'deleted', to_jsonb(x), 'T-262'
  FROM public.mission_bookmarks x
 WHERE NOT EXISTS (SELECT 1 FROM public.users p WHERE p.discord_id = x.discord_id);

DELETE FROM public.mission_bookmarks x
 WHERE NOT EXISTS (SELECT 1 FROM public.users p WHERE p.discord_id = x.discord_id);

-- ═════════════════════════════════════════════════════════════════════════════════════════════
-- §8  Supporting indexes — three, and only three.
--
-- THE RULE: index a referencing column only when it is not already the leading column of an
-- existing index AND the referenced parent is hard-deleted somewhere in the crate. Without the
-- index Postgres runs the referential-action query as a sequential scan ONCE PER DELETED PARENT
-- ROW, which is what turns a modpack delete into N scans of the registry.
--
-- Parents that are genuinely hard-deleted: `modpacks` (`delete_modpack`), `orbat_slots` and
-- `event_missions` (`remove_event_mission`). Everything else this file references is soft-deleted
-- (`events`, `missions`, `users`) or deactivated in place (`servers`), so its ON DELETE action
-- is not on a hot path and an index would be write cost for nothing.
--
-- Already covered, checked one by one against 0001: event_missions.event_id, orbat_slots and
-- event_registrations and orbat_reservations on event_mission_id, mission_versions.mission_id,
-- mission_armories.mission_id, modpack_mods.modpack_id, registry_items.modpack_id,
-- registry_compat.modpack_id (leading in idx_registry_compat_modpack_to),
-- server_statuses.server_id (PK), server_status_histories.server_id (leading in idx_status_hist),
-- match_player_stats.match_id, event_registrations.discord_id, orbat_slots.assigned_to,
-- orbat_reservations.reserved_by, refresh_tokens.discord_id, user_discord_roles.discord_id and
-- mission_bookmarks.discord_id (both leading in their composite PKs).
-- ═════════════════════════════════════════════════════════════════════════════════════════════

-- `orbat_slots` is hard-deleted for real, on every mission detach, 16 rows at a time in the
-- golden data alone — and `event_registrations` grows with every sign-up.
CREATE INDEX IF NOT EXISTS idx_event_registrations_slot_id
    ON public.event_registrations USING btree (slot_id);

-- `modpacks` is hard-deleted by `delete_modpack`. Neither of these two columns appears
-- in any index in 0001-0017.
CREATE INDEX IF NOT EXISTS idx_servers_required_modpack_id
    ON public.servers USING btree (required_modpack_id);

CREATE INDEX IF NOT EXISTS idx_events_modpack_id
    ON public.events USING btree (modpack_id);

-- ═════════════════════════════════════════════════════════════════════════════════════════════
-- §9  The constraints.  DROP IF EXISTS + ADD, so a replay converges; both inside this file's
--     single transaction, so there is no instant at which an enforced constraint is off.
-- ═════════════════════════════════════════════════════════════════════════════════════════════

-- ── A. Ownership → CASCADE ───────────────────────────────────────────────────────────────────

ALTER TABLE public.event_missions DROP CONSTRAINT IF EXISTS event_missions_event_id_fkey;
ALTER TABLE public.event_missions ADD CONSTRAINT event_missions_event_id_fkey
    FOREIGN KEY (event_id) REFERENCES public.events(id) ON DELETE CASCADE;

ALTER TABLE public.event_registrations DROP CONSTRAINT IF EXISTS event_registrations_event_mission_id_fkey;
ALTER TABLE public.event_registrations ADD CONSTRAINT event_registrations_event_mission_id_fkey
    FOREIGN KEY (event_mission_id) REFERENCES public.event_missions(id) ON DELETE CASCADE;

ALTER TABLE public.orbat_slots DROP CONSTRAINT IF EXISTS orbat_slots_event_mission_id_fkey;
ALTER TABLE public.orbat_slots ADD CONSTRAINT orbat_slots_event_mission_id_fkey
    FOREIGN KEY (event_mission_id) REFERENCES public.event_missions(id) ON DELETE CASCADE;

ALTER TABLE public.orbat_reservations DROP CONSTRAINT IF EXISTS orbat_reservations_event_mission_id_fkey;
ALTER TABLE public.orbat_reservations ADD CONSTRAINT orbat_reservations_event_mission_id_fkey
    FOREIGN KEY (event_mission_id) REFERENCES public.event_missions(id) ON DELETE CASCADE;

ALTER TABLE public.mission_versions DROP CONSTRAINT IF EXISTS mission_versions_mission_id_fkey;
ALTER TABLE public.mission_versions ADD CONSTRAINT mission_versions_mission_id_fkey
    FOREIGN KEY (mission_id) REFERENCES public.missions(id) ON DELETE CASCADE;

ALTER TABLE public.mission_armories DROP CONSTRAINT IF EXISTS mission_armories_mission_id_fkey;
ALTER TABLE public.mission_armories ADD CONSTRAINT mission_armories_mission_id_fkey
    FOREIGN KEY (mission_id) REFERENCES public.missions(id) ON DELETE CASCADE;

ALTER TABLE public.mission_bookmarks DROP CONSTRAINT IF EXISTS mission_bookmarks_mission_id_fkey;
ALTER TABLE public.mission_bookmarks ADD CONSTRAINT mission_bookmarks_mission_id_fkey
    FOREIGN KEY (mission_id) REFERENCES public.missions(id) ON DELETE CASCADE;

ALTER TABLE public.modpack_mods DROP CONSTRAINT IF EXISTS modpack_mods_modpack_id_fkey;
ALTER TABLE public.modpack_mods ADD CONSTRAINT modpack_mods_modpack_id_fkey
    FOREIGN KEY (modpack_id) REFERENCES public.modpacks(id) ON DELETE CASCADE;

ALTER TABLE public.registry_compat DROP CONSTRAINT IF EXISTS registry_compat_modpack_id_fkey;
ALTER TABLE public.registry_compat ADD CONSTRAINT registry_compat_modpack_id_fkey
    FOREIGN KEY (modpack_id) REFERENCES public.modpacks(id) ON DELETE CASCADE;

ALTER TABLE public.server_statuses DROP CONSTRAINT IF EXISTS server_statuses_server_id_fkey;
ALTER TABLE public.server_statuses ADD CONSTRAINT server_statuses_server_id_fkey
    FOREIGN KEY (server_id) REFERENCES public.servers(id) ON DELETE CASCADE;

ALTER TABLE public.server_status_histories DROP CONSTRAINT IF EXISTS server_status_histories_server_id_fkey;
ALTER TABLE public.server_status_histories ADD CONSTRAINT server_status_histories_server_id_fkey
    FOREIGN KEY (server_id) REFERENCES public.servers(id) ON DELETE CASCADE;

ALTER TABLE public.match_player_stats DROP CONSTRAINT IF EXISTS match_player_stats_match_id_fkey;
ALTER TABLE public.match_player_stats ADD CONSTRAINT match_player_stats_match_id_fkey
    FOREIGN KEY (match_id) REFERENCES public.matches(id) ON DELETE CASCADE;

-- ── B. Reference → RESTRICT ──────────────────────────────────────────────────────────────────

ALTER TABLE public.event_missions DROP CONSTRAINT IF EXISTS event_missions_mission_id_fkey;
ALTER TABLE public.event_missions ADD CONSTRAINT event_missions_mission_id_fkey
    FOREIGN KEY (mission_id) REFERENCES public.missions(id) ON DELETE RESTRICT;

ALTER TABLE public.registry_items DROP CONSTRAINT IF EXISTS registry_items_modpack_id_fkey;
ALTER TABLE public.registry_items ADD CONSTRAINT registry_items_modpack_id_fkey
    FOREIGN KEY (modpack_id) REFERENCES public.modpacks(id) ON DELETE RESTRICT;

ALTER TABLE public.servers DROP CONSTRAINT IF EXISTS servers_required_modpack_id_fkey;
ALTER TABLE public.servers ADD CONSTRAINT servers_required_modpack_id_fkey
    FOREIGN KEY (required_modpack_id) REFERENCES public.modpacks(id) ON DELETE RESTRICT;

ALTER TABLE public.events DROP CONSTRAINT IF EXISTS events_modpack_id_fkey;
ALTER TABLE public.events ADD CONSTRAINT events_modpack_id_fkey
    FOREIGN KEY (modpack_id) REFERENCES public.modpacks(id) ON DELETE RESTRICT;

ALTER TABLE public.events DROP CONSTRAINT IF EXISTS events_server_id_fkey;
ALTER TABLE public.events ADD CONSTRAINT events_server_id_fkey
    FOREIGN KEY (server_id) REFERENCES public.servers(id) ON DELETE RESTRICT;

-- ── C. Nullable pointer → SET NULL ───────────────────────────────────────────────────────────

ALTER TABLE public.missions DROP CONSTRAINT IF EXISTS missions_current_version_id_fkey;
ALTER TABLE public.missions ADD CONSTRAINT missions_current_version_id_fkey
    FOREIGN KEY (current_version_id) REFERENCES public.mission_versions(id) ON DELETE SET NULL;

ALTER TABLE public.event_registrations DROP CONSTRAINT IF EXISTS event_registrations_slot_id_fkey;
ALTER TABLE public.event_registrations ADD CONSTRAINT event_registrations_slot_id_fkey
    FOREIGN KEY (slot_id) REFERENCES public.orbat_slots(id) ON DELETE SET NULL;

-- ── D. Identity → users(discord_id) ──────────────────────────────────────────────────────────

ALTER TABLE public.event_registrations DROP CONSTRAINT IF EXISTS event_registrations_discord_id_fkey;
ALTER TABLE public.event_registrations ADD CONSTRAINT event_registrations_discord_id_fkey
    FOREIGN KEY (discord_id) REFERENCES public.users(discord_id) ON DELETE CASCADE;

ALTER TABLE public.orbat_slots DROP CONSTRAINT IF EXISTS orbat_slots_assigned_to_fkey;
ALTER TABLE public.orbat_slots ADD CONSTRAINT orbat_slots_assigned_to_fkey
    FOREIGN KEY (assigned_to) REFERENCES public.users(discord_id) ON DELETE SET NULL;

ALTER TABLE public.orbat_reservations DROP CONSTRAINT IF EXISTS orbat_reservations_reserved_by_fkey;
ALTER TABLE public.orbat_reservations ADD CONSTRAINT orbat_reservations_reserved_by_fkey
    FOREIGN KEY (reserved_by) REFERENCES public.users(discord_id) ON DELETE CASCADE;

ALTER TABLE public.refresh_tokens DROP CONSTRAINT IF EXISTS refresh_tokens_discord_id_fkey;
ALTER TABLE public.refresh_tokens ADD CONSTRAINT refresh_tokens_discord_id_fkey
    FOREIGN KEY (discord_id) REFERENCES public.users(discord_id) ON DELETE CASCADE;

ALTER TABLE public.user_discord_roles DROP CONSTRAINT IF EXISTS user_discord_roles_discord_id_fkey;
ALTER TABLE public.user_discord_roles ADD CONSTRAINT user_discord_roles_discord_id_fkey
    FOREIGN KEY (discord_id) REFERENCES public.users(discord_id) ON DELETE CASCADE;

ALTER TABLE public.mission_bookmarks DROP CONSTRAINT IF EXISTS mission_bookmarks_discord_id_fkey;
ALTER TABLE public.mission_bookmarks ADD CONSTRAINT mission_bookmarks_discord_id_fkey
    FOREIGN KEY (discord_id) REFERENCES public.users(discord_id) ON DELETE CASCADE;
