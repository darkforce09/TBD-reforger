-- content_golden.sql — the populated content golden
--
-- THE POPULATED CONTENT GOLDEN. This is the database state that the committed
-- fixture corpus at apps/website/frontend/tests/fixtures/api/ was captured from.
-- Apply it to an empty, migrated database and every fixture in that directory
-- reproduces (see the "REPRODUCING THE FIXTURES" recipe at the bottom).
--
-- WHY THIS FILE EXISTS
-- --------------------
-- Eleven of the twenty-one committed fixtures were captured against a database
-- that had no content: `{"data":[]}`. Because those goldens only ever proved the
-- EMPTY branch of a page renders, the populated branch of servers / events /
-- announcements / leaderboards / audit-logs / vehicle-database / me-deployments
-- was never written at all — the pages are dead, not untested. This seed gives
-- those pages rows to render, so the gate can start proving the code that
-- actually matters.
--
-- DESIGN RULES (deliberate, do not "clean up"):
--
--   1. EVERY id AND timestamp IS PINNED. Nothing uses gen_random_uuid() or now().
--      A fixture recapture must be byte-reproducible or it is not a golden — a
--      `now()` here would rewrite half the corpus on every capture run and drown
--      real contract drift in timestamp churn.
--
--   2. §1 REPRODUCES THE PRE-EXISTING GOLDENS EXACTLY. The user, modpack,
--      mission, mission version, event and event-mission below carry the exact
--      ids and timestamps already committed in GET__me.json,
--      GET__modpacks*.json, GET__missions__512d8658-*.json and
--      GET__events__c71a4d1a-*.json. Change a value in §1 and you invalidate a
--      fixture you did not mean to touch.
--
--   3. MORE THAN ONE ROW PER COLLECTION, AND REAL NULLS. A one-row list renders
--      through a different path than a many-row list (no separators, no
--      ordering, no truncation), and a column that is never null never exercises
--      the None arm. So: three servers (one fully reporting, one reporting with
--      null match/time/weather, one with NO status row at all → `status: null`),
--      audit lines with and without an actor, vehicles with and without a
--      profile image, ORBAT slots both claimed and open.
--
--   4. THE DATES ARE FIXED, SO THEY EVENTUALLY GO STALE. /events?scope=upcoming
--      and the dashboard's next_event filter on `start_time > now()`. The
--      upcoming rows here run to 2027-02; past rows sit in 2026-06/07. When
--      "upcoming" stops being upcoming, bump §6/§8 and recapture — do not switch
--      to now(), that breaks rule 1.
--
--   5. SECTION ORDER IS A FOREIGN-KEY ORDER, NOT A NARRATIVE ONE. Every
--      statement here is fed to psql INDIVIDUALLY, IN AUTOCOMMIT by the seed
--      applier, so a row may only name a parent that an
--      EARLIER statement already inserted. `DEFERRABLE INITIALLY DEFERRED` does
--      not help — there is no enclosing transaction to defer to. Two forward
--      references lived here undetected until migration `0019` constrained the
--      columns, and each broke every fresh environment:
--        missions (§1, §6) → matches (§7) → servers (§3) → server_statuses (§7)
--      Before moving a block, check what its ids point at. `\d <table>` lists
--      the constraints; a violation looks like a seed that "suddenly stopped
--      loading" and takes the whole environment with it.
--
-- Idempotent: every INSERT is ON CONFLICT DO UPDATE / DO NOTHING, so re-running
-- converges instead of erroring. Mission versions and mission artifacts are
-- immutable (a trigger refuses any UPDATE), so their inserts are DO NOTHING: a
-- changed value in either reaches only a database that does not hold the row yet.
--
-- Apply order: 0001..0007 migrations (the API runs these on boot) → this file.
-- registry_dev.sql is INDEPENDENT of this file and still owns GET__registry.json.


-- ═══════════════════════════════════════════════════════════════════════════
-- §1  The pre-existing golden entities — pinned so the committed fixtures
--     that already carry real rows keep reproducing byte-for-byte.
-- ═══════════════════════════════════════════════════════════════════════════

-- The dev-login operator. dev-login itself upserts this row and stamps
-- last_login_at/updated_at with the wall clock, so this UPDATE must run AFTER
-- the login that mints the capture token, or GET__me.json will not reproduce.
-- total_deployments / attendance_rate are the denormalized counters that
-- GET /me/deployments reports as total_operations / attendance_rate.
--
-- §7 seeds ELEVEN match_player_stats rows, and only TWO of them belong to
-- this user -- which is why service_history in the captured
-- GET__me__deployments.json golden has exactly two entries. total_deployments is
-- a standalone denormalized career counter; it is NOT derived from the seeded
-- stats and does not agree with any count in this file.
--
-- DO NOT "fix" the 17 to match a row count: it is pinned by
-- apps/website/frontend/tests/fixtures/api/GET__me__deployments.json
-- ("total_operations":17). Changing it here fails that golden.
INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, arma_character,
                   role, is_banned, total_deployments, attendance_rate,
                   last_login_at, created_at, updated_at)
VALUES ('000000000000000001', 'Dev Operator', 'devoperator', '',
        'dev-arma-76561190000000001', '[TBD] Dev Operator', 'admin', false, 17, 92.50,
        '2026-07-15 09:25:39.30779+00', '2026-07-15 09:25:39.30779+00', '2026-07-15 09:25:39.30779+00')
ON CONFLICT (discord_id) DO UPDATE SET
    username = EXCLUDED.username, discord_handle = EXCLUDED.discord_handle,
    avatar_url = EXCLUDED.avatar_url, arma_id = EXCLUDED.arma_id,
    arma_character = EXCLUDED.arma_character, role = EXCLUDED.role,
    total_deployments = EXCLUDED.total_deployments, attendance_rate = EXCLUDED.attendance_rate,
    last_login_at = EXCLUDED.last_login_at, created_at = EXCLUDED.created_at,
    updated_at = EXCLUDED.updated_at;

-- The current modpack. GET__modpacks.json shows exactly one modpack with an
-- EMPTY mods array, so do not add modpack_mods rows here — that fixture is
-- committed and populated and this seed must not contradict it.
INSERT INTO modpacks (id, name, version, total_size_bytes, workshop_url, is_current, created_at)
VALUES ('00000000-0000-4000-a000-000000000001', 'Core Modern Expansion', '2.1', 48532275200,
        'https://steamcommunity.com/sharedfiles/filedetails/?id=123456789', true,
        '2026-07-15 09:25:39.281298+00')
ON CONFLICT (id) DO UPDATE SET
    name = EXCLUDED.name, version = EXCLUDED.version,
    total_size_bytes = EXCLUDED.total_size_bytes, workshop_url = EXCLUDED.workshop_url,
    is_current = EXCLUDED.is_current, created_at = EXCLUDED.created_at;

-- The mission behind GET__missions__512d8658-*.json and the /missions/:id route.
-- json_payload stays {} — the committed detail fixture pins it, and the ORBAT for
-- this mission is materialized directly into orbat_slots in §9 instead.
--
-- briefing IS '' AND NOT NULL, AND THAT IS LOad-BEARING. `missions.briefing` and
-- `missions.thumbnail_url` are both nullable, and every mission query in the
-- codebase COALESCEs them to '' — except the dossier lookup inside get_event
-- (`operations::handlers::event_listing`, `SELECT title, terrain, game_mode, briefing,
-- thumbnail_url FROM missions`), which decodes straight into String. A NULL in
-- either column there takes the ENTIRE Event Hub down with
--   500 "error occurred while decoding column 3: unexpected null"
-- The API's own create path always writes '' so it never hits this, but a seed
-- or a hand-written row does. Empty string is what the API writes, so empty
-- string is what this file writes.
INSERT INTO missions (id, title, author_id, terrain, game_mode, weather, time_of_day,
                      max_players, status, thumbnail_url, briefing, created_at, updated_at)
VALUES ('512d8658-7025-4a70-94e9-a1b44a7aa155', 'Operation Byte Parity', '000000000000000001',
        'everon', 'pve_coop', 'clear', '14:00:00', 32, 'draft', '', '',
        '2026-07-15 13:53:18.945049+00', '2026-07-15 13:53:18.945049+00')
ON CONFLICT (id) DO UPDATE SET
    title = EXCLUDED.title, author_id = EXCLUDED.author_id, terrain = EXCLUDED.terrain,
    game_mode = EXCLUDED.game_mode, weather = EXCLUDED.weather,
    time_of_day = EXCLUDED.time_of_day, max_players = EXCLUDED.max_players,
    status = EXCLUDED.status, thumbnail_url = EXCLUDED.thumbnail_url,
    briefing = EXCLUDED.briefing,
    created_at = EXCLUDED.created_at, updated_at = EXCLUDED.updated_at;

INSERT INTO mission_versions (id, mission_id, semver, json_payload, created_by, created_at)
VALUES ('563e2aa1-b555-4437-be29-80e9d2550d83', '512d8658-7025-4a70-94e9-a1b44a7aa155',
        '0.1.0', '{}'::jsonb, '000000000000000001', '2026-07-15 13:53:18.945049+00')
ON CONFLICT (id) DO NOTHING;

UPDATE missions SET current_version_id = '563e2aa1-b555-4437-be29-80e9d2550d83'
WHERE id = '512d8658-7025-4a70-94e9-a1b44a7aa155';

-- The event + event-mission behind GET__events__c71a4d1a-*.json and the V-suite
-- `eventhub` / `orbat` routes.
INSERT INTO events (id, name_override, start_time, briefing, banner_image_url, status,
                    registration_locked, max_slots, created_by, created_at, updated_at)
VALUES ('c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7', 'Operation Byte Parity Night',
        '2026-08-01 19:00:00+00', NULL, NULL, 'scheduled', false, 0, '000000000000000001',
        '2026-07-15 14:05:44.629713+00', '2026-07-15 14:05:44.629713+00')
ON CONFLICT (id) DO UPDATE SET
    name_override = EXCLUDED.name_override, start_time = EXCLUDED.start_time,
    briefing = EXCLUDED.briefing, banner_image_url = EXCLUDED.banner_image_url,
    status = EXCLUDED.status, registration_locked = EXCLUDED.registration_locked,
    max_slots = EXCLUDED.max_slots, created_by = EXCLUDED.created_by,
    created_at = EXCLUDED.created_at, updated_at = EXCLUDED.updated_at;

INSERT INTO event_missions (id, event_id, mission_id, start_time, created_at, updated_at)
VALUES ('89b1b731-37a8-4926-901a-3c7ff7de5eb3', 'c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7',
        '512d8658-7025-4a70-94e9-a1b44a7aa155', '2026-08-01 19:00:00+00',
        '2026-07-15 14:05:44.629713+00', '2026-07-15 14:05:44.629713+00')
ON CONFLICT (id) DO UPDATE SET
    event_id = EXCLUDED.event_id, mission_id = EXCLUDED.mission_id,
    start_time = EXCLUDED.start_time, created_at = EXCLUDED.created_at,
    updated_at = EXCLUDED.updated_at;


-- ═══════════════════════════════════════════════════════════════════════════
-- §2  Personnel. The leaderboard JOINs users, the ORBAT shows assignee names,
--     and the audit console shows actor names — none of which render off a
--     one-user roster. Roles span all four tiers; one member is banned and one
--     carries warnings so the Personnel roster's flag paths have input.
-- ═══════════════════════════════════════════════════════════════════════════

INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, arma_character,
                   role, is_banned, ban_reason, banned_by, banned_at,
                   total_deployments, attendance_rate, last_login_at, created_at, updated_at)
VALUES
  ('000000000000000002', 'Rhodes', 'rhodes.tbd',
   'https://cdn.discordapp.com/embed/avatars/1.png', '76561190000000002', '[TBD] Cpt. Rhodes',
   'leader', false, NULL, NULL, NULL, 63, 97.20,
   '2026-07-24 18:41:02+00', '2025-11-02 20:14:07+00', '2026-07-24 18:41:02+00'),
  ('000000000000000003', 'Vance', 'vance.tbd',
   'https://cdn.discordapp.com/embed/avatars/2.png', '76561190000000003', '[TBD] Sgt. Vance',
   'mission_maker', false, NULL, NULL, NULL, 48, 88.60,
   '2026-07-25 21:03:55+00', '2026-01-18 12:00:00+00', '2026-07-25 21:03:55+00'),
  -- No arma_id: the identity-link flow has not been run for this member, so
  -- `arma_id` serializes as null on the roster (Option<String>, not omitted).
  ('000000000000000004', 'Okafor', 'okafor.tbd',
   'https://cdn.discordapp.com/embed/avatars/3.png', NULL, '',
   'enlisted', false, NULL, NULL, NULL, 12, 74.30,
   '2026-07-20 19:52:11+00', '2026-04-09 17:30:00+00', '2026-07-20 19:52:11+00'),
  ('000000000000000005', 'Brandt', 'brandt.tbd', '', '76561190000000005', '[TBD] Pvt. Brandt',
   'enlisted', false, NULL, NULL, NULL, 6, 51.00,
   '2026-06-30 22:10:44+00', '2026-05-21 09:05:00+00', '2026-06-30 22:10:44+00'),
  ('000000000000000006', 'Kessler', 'kessler.tbd', '', '76561190000000006', '[TBD] Kessler',
   'enlisted', true, 'Repeated team-killing after two warnings', '000000000000000001',
   '2026-07-11 23:47:19+00', 3, 22.00,
   '2026-07-11 23:12:00+00', '2026-06-02 15:41:00+00', '2026-07-11 23:47:19+00')
ON CONFLICT (discord_id) DO UPDATE SET
    username = EXCLUDED.username, discord_handle = EXCLUDED.discord_handle,
    avatar_url = EXCLUDED.avatar_url, arma_id = EXCLUDED.arma_id,
    arma_character = EXCLUDED.arma_character, role = EXCLUDED.role,
    is_banned = EXCLUDED.is_banned, ban_reason = EXCLUDED.ban_reason,
    banned_by = EXCLUDED.banned_by, banned_at = EXCLUDED.banned_at,
    total_deployments = EXCLUDED.total_deployments, attendance_rate = EXCLUDED.attendance_rate,
    last_login_at = EXCLUDED.last_login_at, created_at = EXCLUDED.created_at,
    updated_at = EXCLUDED.updated_at;

-- Feeds the `warnings` count column on GET /admin/users.
INSERT INTO warnings (id, discord_id, issued_by, reason, created_at) VALUES
  ('00000000-0000-4000-e000-000000000001', '000000000000000006', '000000000000000001',
   'Friendly fire during OP IRON VEIL — first warning', '2026-06-28 21:15:00+00'),
  ('00000000-0000-4000-e000-000000000002', '000000000000000006', '000000000000000002',
   'Left the AO without notifying the squad lead', '2026-07-05 20:02:00+00')
ON CONFLICT (id) DO NOTHING;


-- ═══════════════════════════════════════════════════════════════════════════
-- §3  Servers. GET /servers is `{data:[{...server, status, required_modpack}]}`.
--
--     THE `server_statuses` ROWS ARE IN §7, NOT HERE (rule 5). The
--     primary server's status names a `matches` row, `matches` names a §6
--     mission, and 0019 constrains both — so the status INSERT has to run after
--     both, and it is the last statement of §7. The three servers stay here
--     because they only reference §1's modpack.
--
--     server_fps IS A numeric(5,1) AND THE HANDLER CASTS IT ::float8. A real
--     frame therefore serializes as `58.7`, NOT `58`. The seeded values are
--     deliberately fractional — an integral 60.0 would hide the frontend DTO's
--     `server_fps: i64` from the fixture, which is exactly how that mismatch
--     shipped in the first place.
-- ═══════════════════════════════════════════════════════════════════════════

INSERT INTO servers (id, name, ip, port, required_modpack_id, is_active) VALUES
  ('00000000-0000-4000-d000-000000000001', 'TBD Primary — Everon',
   '203.0.113.24', 2001, '00000000-0000-4000-a000-000000000001', true),
  ('00000000-0000-4000-d000-000000000002', 'TBD Secondary — Arland',
   '203.0.113.25', 2011, '00000000-0000-4000-a000-000000000001', false),
  -- No required modpack, and no server_statuses row at all (§7): this is the
  -- server that renders with `required_modpack` omitted and `status: null`.
  ('00000000-0000-4000-d000-000000000003', 'TBD Staging — Sandbox',
   '198.51.100.7', 2021, NULL, false)
ON CONFLICT (id) DO UPDATE SET
    name = EXCLUDED.name, ip = EXCLUDED.ip, port = EXCLUDED.port,
    required_modpack_id = EXCLUDED.required_modpack_id, is_active = EXCLUDED.is_active;


-- ═══════════════════════════════════════════════════════════════════════════
-- §3b Machine credentials. GET /servers/:id/credentials lists provenance, use and
--     revocation and never a secret; one live runtime credential that has been
--     used, and one revoked host-agent credential so every optional field is
--     present on some row. Only digests are stored; the secrets they digest are
--     not credentials of any real server.
-- ═══════════════════════════════════════════════════════════════════════════

INSERT INTO server_machine_credentials (id, server_id, executor_kind, secret_sha256, label,
                                        created_by, created_at, last_used_at, revoked_at,
                                        revoked_by, revoke_reason)
VALUES
  ('00000000-0000-4000-e000-000000000001', '00000000-0000-4000-d000-000000000001', 'mod_runtime',
   '804c9cdd5f2f2160cdab798c68b82ef0272e145af50753b90e6b3b9c8eec8bdc', 'Primary runtime',
   '000000000000000001', '2026-07-15 14:10:00+00', '2026-07-20 19:00:00+00', NULL, NULL, NULL),
  ('00000000-0000-4000-e000-000000000002', '00000000-0000-4000-d000-000000000001', 'host_agent',
   '17deaff03142fbff3d3853e621b551d61bafb1ee0a1b4eac47f2581d58464c1f', 'Retired host agent',
   '000000000000000001', '2026-07-15 14:12:00+00', NULL, '2026-07-18 10:00:00+00',
   '000000000000000001', 'Rotated after the host rebuild')
ON CONFLICT (id) DO NOTHING;


-- ═══════════════════════════════════════════════════════════════════════════
-- §4  Announcements. The list filters `status='published'`, orders pinned-first
--     then newest, and the dashboard takes the top 3. The draft row proves the
--     filter still excludes; it must never appear in a fixture.
-- ═══════════════════════════════════════════════════════════════════════════

INSERT INTO announcements (id, title, body, snippet, tag, thumbnail_url, author_id, status,
                           is_pinned, pushed_to_discord, discord_message_id,
                           published_at, created_at, updated_at)
VALUES
  ('00000000-0000-4000-1000-000000000001',
   'Modpack 2.1 is mandatory from Saturday',
   E'The Core Modern Expansion has been bumped to **2.1**.\n\nRe-sync before Saturday''s operation — the server will reject 2.0 clients at the loading screen. Total download is roughly 45 GB; the delta from 2.0 is about 3 GB.\n\nIf your launcher stalls, clear `%LOCALAPPDATA%/Arma Reforger/addons` and re-subscribe.',
   'Core Modern Expansion 2.1 is live. Re-sync before Saturday or the server will reject your client.',
   'modpack_update', 'https://cdn.tbd-reforger.example/news/modpack-21.jpg',
   '000000000000000001', 'published', true, true, '1281994523118829569',
   '2026-07-22 17:00:00+00', '2026-07-22 16:41:03+00', '2026-07-22 17:00:00+00'),
  ('00000000-0000-4000-1000-000000000002',
   'OP BYTE PARITY — orders group Friday 20:00Z',
   E'Squad leads and the platoon staff meet in Command one hour before step-off.\n\nBring your own map markers. Radio matrix will be published in the field manual the morning of.',
   'Squad leads in Command at 20:00Z Friday. Radio matrix published the morning of.',
   'event', NULL, '000000000000000002', 'published', false, true, '1281994523118829570',
   '2026-07-24 09:30:00+00', '2026-07-24 09:22:47+00', '2026-07-24 09:30:00+00'),
  -- No snippet and no thumbnail: both COALESCE to '' and are omitted from the
  -- JSON, so the card renderer has to cope with a body-only announcement.
  ('00000000-0000-4000-1000-000000000003',
   'Server maintenance window Sunday 03:00Z',
   E'Primary goes down for roughly forty minutes for a host kernel update. Secondary stays up for anyone who wants to keep flying.',
   NULL, 'update', NULL, '000000000000000001', 'published', false, false, NULL,
   '2026-07-19 12:00:00+00', '2026-07-19 11:58:10+00', '2026-07-19 12:00:00+00'),
  ('00000000-0000-4000-1000-000000000004',
   'Winter campaign — call for mission makers',
   E'We are opening submissions for the winter campaign arc. Three slots, one per theatre.',
   'Submissions open for the winter campaign arc.', 'important', NULL,
   '000000000000000001', 'published', false, false, NULL,
   '2026-07-08 15:45:00+00', '2026-07-08 15:40:00+00', '2026-07-08 15:45:00+00'),
  -- Draft — must be invisible to GET /announcements and to the dashboard feed.
  ('00000000-0000-4000-1000-000000000005',
   'DRAFT — do not publish',
   'Placeholder body for the unpublished-filter proof.', NULL, 'update', NULL,
   '000000000000000001', 'draft', false, false, NULL,
   NULL, '2026-07-25 08:00:00+00', '2026-07-25 08:00:00+00')
ON CONFLICT (id) DO UPDATE SET
    title = EXCLUDED.title, body = EXCLUDED.body, snippet = EXCLUDED.snippet,
    tag = EXCLUDED.tag, thumbnail_url = EXCLUDED.thumbnail_url,
    author_id = EXCLUDED.author_id, status = EXCLUDED.status,
    is_pinned = EXCLUDED.is_pinned, pushed_to_discord = EXCLUDED.pushed_to_discord,
    discord_message_id = EXCLUDED.discord_message_id, published_at = EXCLUDED.published_at,
    created_at = EXCLUDED.created_at, updated_at = EXCLUDED.updated_at;


-- ═══════════════════════════════════════════════════════════════════════════
-- §5  Doctrine wiki + vehicle database.
--     `field-manual` is not a decorative slug: it is the V-suite's `wikislug`
--     route (/wiki/field-manual). Without this row that route renders a 404
--     branch and the golden proves nothing.
-- ═══════════════════════════════════════════════════════════════════════════

INSERT INTO wiki_pages (id, slug, category, title, icon, body_md, nav_order, updated_by, updated_at)
VALUES
  ('00000000-0000-4000-2000-000000000001', 'field-manual', 'Doctrine', 'Field Manual', 'menu_book',
   E'# TBD Field Manual\n\nThe field manual is the single source of truth for how this unit fights. Where a mission briefing contradicts it, the briefing wins for that operation only.\n\n## 1. Chain of command\n\nPlatoon staff issue intent, not instructions. Squad leads own execution inside their assigned boundary.\n\n## 2. Movement\n\n- Default formation is a staggered column on roads, wedge in the open.\n- Bounding overwatch inside 400 m of a suspected contact.\n- Nobody crosses a linear danger area without near-side security set.\n\n## 3. Contact drills\n\nOn contact: return fire, take cover, report. In that order. The contact report is `CONTACT — direction — distance — description`.\n\n## 4. Casualties\n\nStabilise where the casualty falls only if the position is covered. Otherwise drag to cover first. Medics do not move forward of the base of fire.',
   1, '000000000000000001', '2026-07-14 10:12:00+00'),
  ('00000000-0000-4000-2000-000000000002', 'radio-procedure', 'Doctrine', 'Radio Procedure', 'radio',
   E'# Radio Procedure\n\n## Nets\n\n| Net | Users | Channel |\n| --- | --- | --- |\n| Command | Platoon staff + squad leads | 1 |\n| Squad | Inside a squad | 2–5 |\n| Air | Rotary + JTAC | 8 |\n\n## Format\n\nAlways: `<callsign you want> this is <your callsign>, <message>, over.`\n\nBrevity beats politeness. If the net is busy, wait — do not step on a contact report.',
   2, '000000000000000002', '2026-07-06 19:45:00+00'),
  ('00000000-0000-4000-2000-000000000003', 'medical-sop', 'Doctrine', 'Medical SOP', 'medical_services',
   E'# Medical SOP\n\nTourniquet high and tight, then reassess. Morphine only after bleeding is controlled — it masks the shock that tells you the bleeding is not controlled.\n\nEvery rifleman carries two tourniquets. One is not for you.',
   3, '000000000000000002', '2026-06-29 14:20:00+00'),
  -- No icon: the nav has to render a row with an empty icon slot.
  ('00000000-0000-4000-2000-000000000004', 'server-rules', 'Administration', 'Server Rules', NULL,
   E'# Server Rules\n\n1. No team-killing. Two warnings then a ban; see the audit log for precedent.\n2. Modpack must match the announced version.\n3. Zeus is a privilege, not a rank.',
   10, '000000000000000001', '2026-05-30 08:00:00+00')
ON CONFLICT (id) DO UPDATE SET
    slug = EXCLUDED.slug, category = EXCLUDED.category, title = EXCLUDED.title,
    icon = EXCLUDED.icon, body_md = EXCLUDED.body_md, nav_order = EXCLUDED.nav_order,
    updated_by = EXCLUDED.updated_by, updated_at = EXCLUDED.updated_at;

INSERT INTO vehicle_databases (id, name, faction, armor_type, amphibious, primary_threat,
                               profile_image_url)
VALUES
  ('00000000-0000-4000-3000-000000000001', 'BTR-70', 'USSR', 'Light Armour', 'Yes',
   'Autocannon — 14.5 mm KPVT', 'https://cdn.tbd-reforger.example/iff/btr70.png'),
  ('00000000-0000-4000-3000-000000000002', 'M113A3', 'US Army', 'Light Armour', 'No',
   'Heavy MG — M2 .50 cal', 'https://cdn.tbd-reforger.example/iff/m113a3.png'),
  ('00000000-0000-4000-3000-000000000003', 'UAZ-469', 'USSR', 'Unarmoured', 'No',
   'Small arms only', NULL),
  ('00000000-0000-4000-3000-000000000004', 'M998 Humvee', 'US Army', 'Unarmoured', 'No',
   'Small arms only', 'https://cdn.tbd-reforger.example/iff/m998.png'),
  ('00000000-0000-4000-3000-000000000005', 'Mi-8MT', 'USSR', 'Rotary — Transport', NULL,
   'Door guns — 7.62 mm', 'https://cdn.tbd-reforger.example/iff/mi8mt.png'),
  -- Every optional column empty: amphibious, threat and image all drop out.
  ('00000000-0000-4000-3000-000000000006', 'S105 Sedan', 'Civilian', 'Unarmoured', NULL, NULL, NULL)
ON CONFLICT (id) DO UPDATE SET
    name = EXCLUDED.name, faction = EXCLUDED.faction, armor_type = EXCLUDED.armor_type,
    amphibious = EXCLUDED.amphibious, primary_threat = EXCLUDED.primary_threat,
    profile_image_url = EXCLUDED.profile_image_url;


-- ═══════════════════════════════════════════════════════════════════════════
-- §6  Missions. GET /missions default scope is
--     `status='live' OR (author_id = caller AND status <> 'archived')`, so the
--     caller sees every live mission plus their own drafts. GET /approvals is a
--     separate query over `status='pending_approval'` — that queue is empty
--     unless at least one mission sits in that state, which is §6's other job.
--
--     ORDERING (rule 5): this section sits ahead of the matches.
--     `matches` names mission …c000-000000000001 from this block, and 0019
--     constrains `matches.mission_id`, so the missions have to land first.
--
--     thumbnail_url and briefing are '' rather than NULL for the reason spelled
--     out in §1: a NULL in either column 500s GET /events/:id for any event the
--     mission is attached to. They still serialize as ABSENT (both columns are
--     skip_serializing_if String::is_empty), so the "missing thumbnail" and
--     "no briefing" render paths are exercised exactly as a NULL would.
-- ═══════════════════════════════════════════════════════════════════════════

INSERT INTO missions (id, title, author_id, terrain, custom_terrain_name, game_mode, weather,
                      time_of_day, max_players, status, thumbnail_url, briefing,
                      rejection_reason, reviewed_by, reviewed_at, created_at, updated_at)
VALUES
  ('00000000-0000-4000-c000-000000000001', 'Operation Iron Veil', '000000000000000003',
   'arland', NULL, 'pve_coop', 'overcast', '05:30:00', 48, 'live',
   'https://cdn.tbd-reforger.example/missions/iron-veil.jpg',
   E'A Soviet motor rifle company has pushed across the northern bridge and is consolidating around Montignac. Two platoons dismount at the quarry and clear east to west.\n\nNo armour support. Expect BTRs.',
   NULL, '000000000000000001', '2026-06-11 10:02:00+00',
   '2026-06-10 18:22:00+00', '2026-07-23 16:04:00+00'),
  ('00000000-0000-4000-c000-000000000002', 'Operation Static Line', '000000000000000003',
   'everon', NULL, 'pvp', 'clear', '12:00:00', 64, 'live',
   'https://cdn.tbd-reforger.example/missions/static-line.jpg',
   E'Force-on-force over the airfield. Two sides, one objective, ninety minutes.',
   NULL, '000000000000000001', '2026-06-27 09:14:00+00',
   '2026-06-26 20:00:00+00', '2026-07-21 11:30:00+00'),
  -- Custom terrain: exercises the custom_terrain_name branch, which is skipped
  -- entirely when terrain is one of the two built-ins.
  ('00000000-0000-4000-c000-000000000003', 'Operation Harrow Gate', '000000000000000002',
   'custom', 'Kunar Valley (community)', 'zeus', 'dense_fog', '19:45:00', 40, 'live',
   '',
   E'Zeus-run escalation in the valley. The GM owns tempo; squad leads own the ground.',
   NULL, '000000000000000001', '2026-07-02 08:41:00+00',
   '2026-07-01 21:11:00+00', '2026-07-18 09:15:00+00'),
  -- Awaiting review → the only rows GET /approvals can ever return.
  ('00000000-0000-4000-c000-000000000004', 'Operation Cold Anvil', '000000000000000003',
   'everon', NULL, 'pve_coop', 'heavy_rain', '03:15:00', 32, 'pending_approval',
   '',
   E'Night infiltration onto the radar site. Suppressed weapons throughout; the alarm is a mission failure, not a setback.',
   NULL, NULL, NULL, '2026-07-19 22:40:00+00', '2026-07-20 07:05:00+00'),
  ('00000000-0000-4000-c000-000000000005', 'Exercise Paper Tiger', '000000000000000002',
   'arland', NULL, 'pve_coop', 'clear', '10:00:00', 24, 'pending_approval',
   '', '',
   NULL, NULL, NULL, '2026-07-24 13:02:00+00', '2026-07-24 13:02:00+00'),
  -- Rejected: carries rejection_reason + reviewer, which no other row does.
  ('00000000-0000-4000-c000-000000000006', 'Operation Glass House', '000000000000000002',
   'everon', NULL, 'pvp', 'clear', '16:00:00', 20, 'rejected',
   '', E'Urban PvP in Levie.',
   'Slot count does not match the ORBAT — 20 declared, 34 materialized. Resubmit once the template is fixed.',
   '000000000000000001', '2026-07-16 12:30:00+00',
   '2026-07-15 19:00:00+00', '2026-07-16 12:30:00+00')
ON CONFLICT (id) DO UPDATE SET
    title = EXCLUDED.title, author_id = EXCLUDED.author_id, terrain = EXCLUDED.terrain,
    custom_terrain_name = EXCLUDED.custom_terrain_name, game_mode = EXCLUDED.game_mode,
    weather = EXCLUDED.weather, time_of_day = EXCLUDED.time_of_day,
    max_players = EXCLUDED.max_players, status = EXCLUDED.status,
    thumbnail_url = EXCLUDED.thumbnail_url, briefing = EXCLUDED.briefing,
    rejection_reason = EXCLUDED.rejection_reason, reviewed_by = EXCLUDED.reviewed_by,
    reviewed_at = EXCLUDED.reviewed_at, created_at = EXCLUDED.created_at,
    updated_at = EXCLUDED.updated_at;

INSERT INTO mission_versions (id, mission_id, semver, json_payload, editor_notes, created_by, created_at)
VALUES
  ('00000000-0000-4000-8000-000000000001', '00000000-0000-4000-c000-000000000001', '1.2.0',
   '{}'::jsonb, 'Rebalanced the BTR patrol route', '000000000000000003', '2026-07-23 16:04:00+00'),
  ('00000000-0000-4000-8000-000000000002', '00000000-0000-4000-c000-000000000002', '0.9.1',
   '{}'::jsonb, NULL, '000000000000000003', '2026-07-21 11:30:00+00'),
  ('00000000-0000-4000-8000-000000000003', '00000000-0000-4000-c000-000000000003', '2.0.0',
   '{}'::jsonb, NULL, '000000000000000002', '2026-07-18 09:15:00+00'),
  ('00000000-0000-4000-8000-000000000004', '00000000-0000-4000-c000-000000000004', '0.3.0',
   '{}'::jsonb, 'Submitted for review', '000000000000000003', '2026-07-20 07:05:00+00')
ON CONFLICT (id) DO NOTHING;

UPDATE missions SET current_version_id = v.id
FROM (VALUES
    ('00000000-0000-4000-c000-000000000001'::uuid, '00000000-0000-4000-8000-000000000001'::uuid),
    ('00000000-0000-4000-c000-000000000002'::uuid, '00000000-0000-4000-8000-000000000002'::uuid),
    ('00000000-0000-4000-c000-000000000003'::uuid, '00000000-0000-4000-8000-000000000003'::uuid),
    ('00000000-0000-4000-c000-000000000004'::uuid, '00000000-0000-4000-8000-000000000004'::uuid)
) AS v(mission_id, id)
WHERE missions.id = v.mission_id;

-- One bookmark for the caller, so ?scope=bookmarked is not a dead branch and the
-- `bookmarked: true` flag appears on at least one card in the default list.
INSERT INTO mission_bookmarks (discord_id, mission_id)
VALUES ('000000000000000001', '00000000-0000-4000-c000-000000000001')
ON CONFLICT DO NOTHING;


-- ═══════════════════════════════════════════════════════════════════════════
-- §7  Matches + per-player stats, and the live telemetry row that points at one.
--     The matches are the ONLY source for two endpoints: GET /leaderboards reads
--     the leaderboard_totals materialized view built over match_player_stats,
--     and GET /me/deployments builds service_history by joining these rows back
--     to `matches`. Neither can be seeded directly.
--
--     The final statement in this file refreshes the MV; without it the
--     leaderboard stays empty no matter how many stat rows exist.
--
--     ORDERING (rule 5): this section sits after the missions, and
--     `server_statuses` sits here rather than in §3 beside the servers.
--     Both would otherwise be forward references, which 0019 turns into hard errors:
--       * `matches.mission_id` names a §6 mission → §6/§7 swapped.
--       * `server_statuses.current_match_id` names …f000-000000000003 below →
--         the status row moved down here, after the match it points at. The
--         `servers` it also references stay in §3, well above.
-- ═══════════════════════════════════════════════════════════════════════════

INSERT INTO matches (id, source_match_id, event_id, mission_id, terrain, started_at, ended_at,
                     outcome, winning_faction, aar_replay_url, created_at)
VALUES
  ('00000000-0000-4000-f000-000000000001', 'rf-match-20260620-01', NULL,
   '512d8658-7025-4a70-94e9-a1b44a7aa155', 'everon',
   '2026-06-20 19:05:00+00', '2026-06-20 21:48:00+00', 'success', 'BLUFOR',
   'https://cdn.tbd-reforger.example/aar/20260620-01.json', '2026-06-20 21:48:30+00'),
  ('00000000-0000-4000-f000-000000000002', 'rf-match-20260704-01', NULL,
   '00000000-0000-4000-c000-000000000001', 'arland',
   '2026-07-04 19:00:00+00', '2026-07-04 22:12:00+00', 'failure', 'OPFOR',
   '', '2026-07-04 22:12:40+00'),
  -- Still running: no ended_at, outcome pending, no replay. This is the match id
  -- the primary server reports as current_match_id — the `server_statuses`
  -- INSERT that names it is the last statement in this section, below.
  -- '' (not NULL) for winning_faction / aar_replay_url — the canonical empty (0015):
  -- the create path COALESCEs to '', so NOT NULL holds.
  ('00000000-0000-4000-f000-000000000003', 'rf-match-20260726-01', NULL,
   '512d8658-7025-4a70-94e9-a1b44a7aa155', 'everon',
   '2026-07-26 04:32:00+00', NULL, 'pending', '', '', '2026-07-26 04:32:10+00')
ON CONFLICT (id) DO UPDATE SET
    source_match_id = EXCLUDED.source_match_id, event_id = EXCLUDED.event_id,
    mission_id = EXCLUDED.mission_id, terrain = EXCLUDED.terrain,
    started_at = EXCLUDED.started_at, ended_at = EXCLUDED.ended_at,
    outcome = EXCLUDED.outcome, winning_faction = EXCLUDED.winning_faction,
    aar_replay_url = EXCLUDED.aar_replay_url, created_at = EXCLUDED.created_at;

-- kd_ratio is round(sum(kills)/sum(deaths), 2) and command_win_rate is
-- round(command_wins/command_games, 3) — both deliberately land on values that
-- are NOT integers so the numeric formatting on the board is actually exercised.
INSERT INTO match_player_stats (id, match_id, discord_id, arma_id, role_played, kills, deaths,
                                team_kills, longest_kill_m, vehicles_destroyed, is_command,
                                command_win, source_event_id, created_at)
VALUES
  -- Match 1 — successful assault on Everon.
  ('00000000-0000-4000-9000-000000000001', '00000000-0000-4000-f000-000000000001',
   '000000000000000001', 'dev-arma-76561190000000001', 'Platoon Leader',
   9, 2, 0, 412, 1, true, true, 'rf-evt-20260620-01', '2026-06-20 21:48:00+00'),
  ('00000000-0000-4000-9000-000000000002', '00000000-0000-4000-f000-000000000001',
   '000000000000000002', '76561190000000002', 'Squad Leader',
   14, 1, 0, 288, 2, true, true, 'rf-evt-20260620-01', '2026-06-20 21:48:00+00'),
  ('00000000-0000-4000-9000-000000000003', '00000000-0000-4000-f000-000000000001',
   '000000000000000003', '76561190000000003', 'Machine Gunner',
   21, 3, 1, 194, 0, false, NULL, 'rf-evt-20260620-01', '2026-06-20 21:48:00+00'),
  ('00000000-0000-4000-9000-000000000004', '00000000-0000-4000-f000-000000000001',
   '000000000000000004', '76561190000000004', 'Rifleman',
   4, 4, 0, 121, 0, false, NULL, 'rf-evt-20260620-01', '2026-06-20 21:48:00+00'),
  ('00000000-0000-4000-9000-000000000005', '00000000-0000-4000-f000-000000000001',
   '000000000000000005', '76561190000000005', 'Medic',
   1, 5, 0, 42, 0, false, NULL, 'rf-evt-20260620-01', '2026-06-20 21:48:00+00'),
  -- An unlinked player: discord_id NULL. The MV filters these out, so this row
  -- proves telemetry from a non-member does not corrupt the leaderboard.
  ('00000000-0000-4000-9000-000000000006', '00000000-0000-4000-f000-000000000001',
   NULL, '76561190000000999', 'Rifleman',
   2, 6, 0, 88, 0, false, NULL, 'rf-evt-20260620-01', '2026-06-20 21:48:00+00'),

  -- Match 2 — a defeat on Arland. Command loss for the same two leaders, which
  -- is what drags command_win_rate off 1.000 into 0.500.
  ('00000000-0000-4000-9000-000000000007', '00000000-0000-4000-f000-000000000002',
   '000000000000000001', 'dev-arma-76561190000000001', 'Platoon Leader',
   6, 3, 0, 355, 0, true, false, 'rf-evt-20260704-01', '2026-07-04 22:12:00+00'),
  ('00000000-0000-4000-9000-000000000008', '00000000-0000-4000-f000-000000000002',
   '000000000000000002', '76561190000000002', 'Squad Leader',
   11, 2, 0, 640, 1, true, false, 'rf-evt-20260704-01', '2026-07-04 22:12:00+00'),
  ('00000000-0000-4000-9000-000000000009', '00000000-0000-4000-f000-000000000002',
   '000000000000000003', '76561190000000003', 'Grenadier',
   8, 5, 0, 210, 1, false, NULL, 'rf-evt-20260704-01', '2026-07-04 22:12:00+00'),
  ('00000000-0000-4000-9000-000000000010', '00000000-0000-4000-f000-000000000002',
   '000000000000000004', '76561190000000004', 'Automatic Rifleman',
   7, 3, 0, 167, 0, false, NULL, 'rf-evt-20260704-01', '2026-07-04 22:12:00+00'),
  -- Kessler's team-kills — the rows the ban in §2 and the audit trail in §10
  -- both refer to.
  ('00000000-0000-4000-9000-000000000011', '00000000-0000-4000-f000-000000000002',
   '000000000000000006', '76561190000000006', 'Rifleman',
   0, 7, 3, 35, 0, false, NULL, 'rf-evt-20260704-01', '2026-07-04 22:12:00+00')
ON CONFLICT (id) DO UPDATE SET
    match_id = EXCLUDED.match_id, discord_id = EXCLUDED.discord_id,
    arma_id = EXCLUDED.arma_id, role_played = EXCLUDED.role_played,
    kills = EXCLUDED.kills, deaths = EXCLUDED.deaths, team_kills = EXCLUDED.team_kills,
    longest_kill_m = EXCLUDED.longest_kill_m, vehicles_destroyed = EXCLUDED.vehicles_destroyed,
    is_command = EXCLUDED.is_command, command_win = EXCLUDED.command_win,
    source_event_id = EXCLUDED.source_event_id, created_at = EXCLUDED.created_at;

INSERT INTO server_statuses (server_id, is_online, player_count, max_players, server_fps,
                             uptime_seconds, current_match_id, ingame_time, ingame_weather,
                             updated_at)
VALUES
  -- Fully reporting, mid-operation. 58.7 fps is a healthy-but-not-round frame.
  ('00000000-0000-4000-d000-000000000001', true, 47, 64, 58.7, 19_842,
   '00000000-0000-4000-f000-000000000003', '06:42', 'overcast', '2026-07-26 05:00:00+00'),
  -- Online but idle: no match, no simulated clock, no weather. The three nullable
  -- columns COALESCE to '' in the handler and drop out of the JSON entirely.
  ('00000000-0000-4000-d000-000000000002', true, 3, 48, 29.4, 421_066,
   NULL, NULL, NULL, '2026-07-26 04:58:12+00')
ON CONFLICT (server_id) DO UPDATE SET
    is_online = EXCLUDED.is_online, player_count = EXCLUDED.player_count,
    max_players = EXCLUDED.max_players, server_fps = EXCLUDED.server_fps,
    uptime_seconds = EXCLUDED.uptime_seconds, current_match_id = EXCLUDED.current_match_id,
    ingame_time = EXCLUDED.ingame_time, ingame_weather = EXCLUDED.ingame_weather,
    updated_at = EXCLUDED.updated_at;


-- ═══════════════════════════════════════════════════════════════════════════
-- §8  Further operations. GET /events?scope=upcoming filters `start_time >
--     now()`, so the upcoming rows are dated far enough ahead to stay upcoming
--     for a while; the past row exists so ?scope=past is not an empty branch.
--     Rule 4 above: when these dates lapse, bump them and recapture.
-- ═══════════════════════════════════════════════════════════════════════════

INSERT INTO events (id, name_override, start_time, briefing, banner_image_url, status,
                    registration_locked, max_slots, created_by, created_at, updated_at)
VALUES
  ('00000000-0000-4000-7000-000000000001', 'OP IRON VEIL — Main Effort',
   '2026-09-05 19:00:00+00',
   E'Company operation on Arland. Two rifle platoons plus a weapons detachment.\n\nOrders group one hour prior in Command.',
   'https://cdn.tbd-reforger.example/events/iron-veil-banner.jpg',
   'open', false, 48, '000000000000000001',
   '2026-07-20 14:00:00+00', '2026-07-25 09:12:00+00'),
  -- Locked registration + no briefing/banner: the "you cannot sign up" branch.
  ('00000000-0000-4000-7000-000000000002', 'OP STATIC LINE — Force on Force',
   '2026-10-03 18:30:00+00', NULL, NULL,
   'locked', true, 64, '000000000000000001',
   '2026-07-22 10:30:00+00', '2026-07-25 20:00:00+00'),
  -- No name_override: the hub falls back to the attached mission's title.
  ('00000000-0000-4000-7000-000000000003', NULL,
   '2027-02-06 19:00:00+00', E'Winter arc, first serial.', NULL,
   'scheduled', false, 40, '000000000000000002',
   '2026-07-25 16:45:00+00', '2026-07-25 16:45:00+00'),
  -- Completed and in the past — ?scope=past, and nothing else.
  ('00000000-0000-4000-7000-000000000004', 'OP PAPER TIGER — Shakeout',
   '2026-07-04 19:00:00+00', E'Shakeout serial for the new joiners.', NULL,
   'completed', true, 24, '000000000000000001',
   '2026-06-25 12:00:00+00', '2026-07-04 22:30:00+00')
ON CONFLICT (id) DO UPDATE SET
    name_override = EXCLUDED.name_override, start_time = EXCLUDED.start_time,
    briefing = EXCLUDED.briefing, banner_image_url = EXCLUDED.banner_image_url,
    status = EXCLUDED.status, registration_locked = EXCLUDED.registration_locked,
    max_slots = EXCLUDED.max_slots, created_by = EXCLUDED.created_by,
    created_at = EXCLUDED.created_at, updated_at = EXCLUDED.updated_at;

INSERT INTO event_missions (id, event_id, mission_id, start_time, created_at, updated_at)
VALUES
  ('00000000-0000-4000-6000-000000000001', '00000000-0000-4000-7000-000000000001',
   '00000000-0000-4000-c000-000000000001', '2026-09-05 19:00:00+00',
   '2026-07-20 14:01:00+00', '2026-07-20 14:01:00+00'),
  -- Two missions on one event: mission_count = 2 on the list card, which a
  -- single-mission event can never produce.
  ('00000000-0000-4000-6000-000000000002', '00000000-0000-4000-7000-000000000001',
   '00000000-0000-4000-c000-000000000003', '2026-09-05 21:30:00+00',
   '2026-07-20 14:02:00+00', '2026-07-20 14:02:00+00'),
  ('00000000-0000-4000-6000-000000000003', '00000000-0000-4000-7000-000000000002',
   '00000000-0000-4000-c000-000000000002', '2026-10-03 18:30:00+00',
   '2026-07-22 10:31:00+00', '2026-07-22 10:31:00+00'),
  ('00000000-0000-4000-6000-000000000004', '00000000-0000-4000-7000-000000000004',
   '00000000-0000-4000-c000-000000000001', '2026-07-04 19:00:00+00',
   '2026-06-25 12:01:00+00', '2026-06-25 12:01:00+00')
ON CONFLICT (id) DO UPDATE SET
    event_id = EXCLUDED.event_id, mission_id = EXCLUDED.mission_id,
    start_time = EXCLUDED.start_time, created_at = EXCLUDED.created_at,
    updated_at = EXCLUDED.updated_at;
-- Event 00000000-0000-4000-7000-000000000003 intentionally has NO event_missions
-- row: mission_count 0, total_slots 0, percent 0 — the freshly-scheduled state.


-- ═══════════════════════════════════════════════════════════════════════════
-- §9  ORBAT. These rows are what GET /event-missions/:emid/orbat groups into
--     squads, and they are also what turns the event hub's `filled`/`total`/
--     `factions` from zeroes into real fill state.
--
--     Slots are materialized directly rather than via a mission json_payload
--     ORBAT template, because the committed GET__missions__512d8658-*.json pins
--     that mission's json_payload to {} and this seed must not contradict it.
--
--     Mixed claim state on purpose: a full squad, a partly-filled squad, an
--     untouched squad, and one squad under a leader's reservation hold.
-- ═══════════════════════════════════════════════════════════════════════════

INSERT INTO orbat_slots (id, event_mission_id, faction, squad, callsign, role, loadout, tag,
                         slot_index, assigned_to, assigned_at)
VALUES
  -- BLUFOR / Command — fully claimed.
  ('00000000-0000-4000-5000-000000000001', '89b1b731-37a8-4926-901a-3c7ff7de5eb3',
   'BLUFOR', 'Command', 'HAVOC', 'Platoon Leader', 'L85A3 + Optic', 'CMD', 0,
   '000000000000000001', '2026-07-16 09:14:22+00'),
  ('00000000-0000-4000-5000-000000000002', '89b1b731-37a8-4926-901a-3c7ff7de5eb3',
   'BLUFOR', 'Command', 'HAVOC', 'Platoon Sergeant', 'L85A3', NULL, 1,
   '000000000000000002', '2026-07-16 09:15:40+00'),
  ('00000000-0000-4000-5000-000000000003', '89b1b731-37a8-4926-901a-3c7ff7de5eb3',
   'BLUFOR', 'Command', 'HAVOC', 'Radio Operator', 'L85A3 + Long Range Radio', 'RTO', 2,
   '000000000000000003', '2026-07-16 09:16:05+00'),

  -- BLUFOR / Alpha — partly claimed; three of six open.
  ('00000000-0000-4000-5000-000000000004', '89b1b731-37a8-4926-901a-3c7ff7de5eb3',
   'BLUFOR', 'Alpha', 'ALPHA', 'Squad Leader', 'L85A3 + Optic', 'SL', 0,
   '000000000000000004', '2026-07-17 20:01:11+00'),
  ('00000000-0000-4000-5000-000000000005', '89b1b731-37a8-4926-901a-3c7ff7de5eb3',
   'BLUFOR', 'Alpha', 'ALPHA', 'Grenadier', 'L85A3 + UGL', NULL, 1,
   '000000000000000005', '2026-07-17 20:03:47+00'),
  ('00000000-0000-4000-5000-000000000006', '89b1b731-37a8-4926-901a-3c7ff7de5eb3',
   'BLUFOR', 'Alpha', 'ALPHA', 'Automatic Rifleman', 'L110A3', NULL, 2, NULL, NULL),
  ('00000000-0000-4000-5000-000000000007', '89b1b731-37a8-4926-901a-3c7ff7de5eb3',
   'BLUFOR', 'Alpha', 'ALPHA', 'Combat Medic', 'L85A3 + Medical', 'MED', 3, NULL, NULL),
  ('00000000-0000-4000-5000-000000000008', '89b1b731-37a8-4926-901a-3c7ff7de5eb3',
   'BLUFOR', 'Alpha', 'ALPHA', 'Rifleman', 'L85A3', NULL, 4, NULL, NULL),
  ('00000000-0000-4000-5000-000000000009', '89b1b731-37a8-4926-901a-3c7ff7de5eb3',
   'BLUFOR', 'Alpha', 'ALPHA', 'Rifleman (AT)', 'L85A3 + AT4', 'LAT', 5, NULL, NULL),

  -- BLUFOR / Bravo — untouched, and held by a leader (§9 reservation below).
  -- No loadout and no tag on any slot: both COALESCE to '' and drop out.
  ('00000000-0000-4000-5000-000000000010', '89b1b731-37a8-4926-901a-3c7ff7de5eb3',
   'BLUFOR', 'Bravo', 'BRAVO', 'Squad Leader', NULL, NULL, 0, NULL, NULL),
  ('00000000-0000-4000-5000-000000000011', '89b1b731-37a8-4926-901a-3c7ff7de5eb3',
   'BLUFOR', 'Bravo', 'BRAVO', 'Grenadier', NULL, NULL, 1, NULL, NULL),
  ('00000000-0000-4000-5000-000000000012', '89b1b731-37a8-4926-901a-3c7ff7de5eb3',
   'BLUFOR', 'Bravo', 'BRAVO', 'Rifleman', NULL, NULL, 2, NULL, NULL),
  ('00000000-0000-4000-5000-000000000013', '89b1b731-37a8-4926-901a-3c7ff7de5eb3',
   'BLUFOR', 'Bravo', 'BRAVO', 'Rifleman', NULL, NULL, 3, NULL, NULL),

  -- OPFOR / Recon — a second faction, so the hub's `factions` array has two
  -- entries and the ORBAT selector has to render a faction split at all.
  ('00000000-0000-4000-5000-000000000014', '89b1b731-37a8-4926-901a-3c7ff7de5eb3',
   'OPFOR', 'Recon', 'GHOST', 'Team Leader', 'AK-74 + Optic', 'TL', 0, NULL, NULL),
  -- Free seat: registration §9 names this member's earlier seat (…0005), and
  -- idx_orbat_slots_em_assigned (0017: UNIQUE (event_mission_id, assigned_to)
  -- WHERE assigned_to IS NOT NULL) allows one seat per member, so assigned_to
  -- stays NULL here.
  ('00000000-0000-4000-5000-000000000015', '89b1b731-37a8-4926-901a-3c7ff7de5eb3',
   'OPFOR', 'Recon', 'GHOST', 'Designated Marksman', 'SVD', 'DMR', 1,
   NULL, NULL),
  ('00000000-0000-4000-5000-000000000016', '89b1b731-37a8-4926-901a-3c7ff7de5eb3',
   'OPFOR', 'Recon', 'GHOST', 'Scout', 'AKS-74U', NULL, 2, NULL, NULL)
ON CONFLICT (id) DO UPDATE SET
    event_mission_id = EXCLUDED.event_mission_id, faction = EXCLUDED.faction,
    squad = EXCLUDED.squad, callsign = EXCLUDED.callsign, role = EXCLUDED.role,
    loadout = EXCLUDED.loadout, tag = EXCLUDED.tag, slot_index = EXCLUDED.slot_index,
    assigned_to = EXCLUDED.assigned_to, assigned_at = EXCLUDED.assigned_at;

-- A leader's hold on Bravo — populates reserved_by / reserved_by_name on that
-- squad, which is otherwise an unreachable branch of the ORBAT response.
INSERT INTO orbat_reservations (id, event_mission_id, squad, reserved_by, reserved_at)
VALUES ('00000000-0000-4000-4000-000000000001', '89b1b731-37a8-4926-901a-3c7ff7de5eb3',
        'Bravo', '000000000000000002', '2026-07-18 08:30:00+00')
ON CONFLICT (id) DO NOTHING;

-- Registrations. The caller's own row is what makes GET /me/deployments return
-- a non-empty `upcoming` list and the dashboard return a `my_assignment`.
-- Every active reservation references its participant's event allocation; one
-- statement writes both so each commit leaves them consistent.
WITH allocations AS (
    INSERT INTO event_participant_allocations (id, event_id, discord_id, quota_kind, acquired_at)
    VALUES
      ('00000000-0000-4000-a200-000000000001', 'c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7',
       '000000000000000001', 'member', '2026-07-16 09:14:22+00'),
      ('00000000-0000-4000-a200-000000000002', 'c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7',
       '000000000000000002', 'member', '2026-07-16 09:15:40+00'),
      ('00000000-0000-4000-a200-000000000003', 'c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7',
       '000000000000000003', 'member', '2026-07-16 09:16:05+00'),
      ('00000000-0000-4000-a200-000000000004', 'c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7',
       '000000000000000004', 'member', '2026-07-17 20:01:11+00'),
      ('00000000-0000-4000-a200-000000000005', 'c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7',
       '000000000000000005', 'guest', '2026-07-17 20:03:47+00'),
      ('00000000-0000-4000-a200-000000000006', '00000000-0000-4000-7000-000000000001',
       '000000000000000001', 'member', '2026-07-21 18:05:00+00')
    ON CONFLICT (id) DO NOTHING
    RETURNING id
)
INSERT INTO event_registrations (id, event_mission_id, discord_id, slot_id, reservation_state, registered_at, allocation_id)
VALUES
  ('00000000-0000-4000-a100-000000000001', '89b1b731-37a8-4926-901a-3c7ff7de5eb3',
   '000000000000000001', '00000000-0000-4000-5000-000000000001', 'registered',
   '2026-07-16 09:14:22+00', '00000000-0000-4000-a200-000000000001'),
  ('00000000-0000-4000-a100-000000000002', '89b1b731-37a8-4926-901a-3c7ff7de5eb3',
   '000000000000000002', '00000000-0000-4000-5000-000000000002', 'registered',
   '2026-07-16 09:15:40+00', '00000000-0000-4000-a200-000000000002'),
  ('00000000-0000-4000-a100-000000000003', '89b1b731-37a8-4926-901a-3c7ff7de5eb3',
   '000000000000000003', '00000000-0000-4000-5000-000000000003', 'registered',
   '2026-07-16 09:16:05+00', '00000000-0000-4000-a200-000000000003'),
  ('00000000-0000-4000-a100-000000000004', '89b1b731-37a8-4926-901a-3c7ff7de5eb3',
   '000000000000000004', '00000000-0000-4000-5000-000000000004', 'registered',
   '2026-07-17 20:01:11+00', '00000000-0000-4000-a200-000000000004'),
  ('00000000-0000-4000-a100-000000000005', '89b1b731-37a8-4926-901a-3c7ff7de5eb3',
   '000000000000000005', '00000000-0000-4000-5000-000000000005', 'registered',
   '2026-07-17 20:03:47+00', '00000000-0000-4000-a200-000000000005'),
  -- Registered without claiming a slot (slot_id NULL) — a real state the
  -- registration flow produces and the counters have to handle.
  ('00000000-0000-4000-a100-000000000006', '89b1b731-37a8-4926-901a-3c7ff7de5eb3',
   '000000000000000006', NULL, 'waitlisted', '2026-07-19 07:44:00+00', NULL),
  -- Withdrawn: excluded from the `registered` count on the list card.
  ('00000000-0000-4000-a100-000000000007', '00000000-0000-4000-6000-000000000001',
   '000000000000000004', NULL, 'withdrawn', '2026-07-21 18:00:00+00', NULL),
  ('00000000-0000-4000-a100-000000000008', '00000000-0000-4000-6000-000000000001',
   '000000000000000001', NULL, 'registered', '2026-07-21 18:05:00+00',
   '00000000-0000-4000-a200-000000000006')
ON CONFLICT (id) DO UPDATE SET
    event_mission_id = EXCLUDED.event_mission_id, discord_id = EXCLUDED.discord_id,
    slot_id = EXCLUDED.slot_id, reservation_state = EXCLUDED.reservation_state,
    registered_at = EXCLUDED.registered_at, allocation_id = EXCLUDED.allocation_id;

-- The waiting queue orders by the time a participant joined it; pin it to the signup.
UPDATE event_registrations SET queue_entered_at = registered_at
WHERE id::text LIKE '00000000-0000-4000-a100-%' AND queue_entered_at IS DISTINCT FROM registered_at;

-- Event access for the golden operation: TBD members or its managed roster, a partner
-- guild group that alone admits the OPFOR Recon squad, one named seat, and guest places.
-- Every participant is on the roster, so each seeded reservation stays eligible and a
-- policy re-evaluation releases nobody.
INSERT INTO event_groups (id, event_id, name, source, created_by, created_at, updated_at)
VALUES
  ('00000000-0000-4000-b100-000000000001', 'c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7',
   'Byte Parity roster', '{"kind":"managed_roster"}', '000000000000000001',
   '2026-07-15 14:20:00+00', '2026-07-15 14:20:00+00'),
  ('00000000-0000-4000-b100-000000000002', 'c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7',
   'Allied reconnaissance', '{"kind":"partner_guild","guild_id":"100000000000000777","required_role_ids":["200000000000000888"]}',
   '000000000000000001', '2026-07-15 14:21:00+00', '2026-07-15 14:21:00+00')
ON CONFLICT (id) DO NOTHING;

INSERT INTO event_group_roster (group_id, discord_id, added_by, added_at)
SELECT '00000000-0000-4000-b100-000000000001', member.discord_id, '000000000000000001',
       '2026-07-15 14:25:00+00'::timestamptz + make_interval(mins => member.position)
FROM (VALUES ('000000000000000001', 1), ('000000000000000002', 2), ('000000000000000003', 3),
             ('000000000000000004', 4), ('000000000000000005', 5), ('000000000000000006', 6))
     AS member(discord_id, position)
ON CONFLICT (group_id, discord_id) DO NOTHING;

INSERT INTO event_squad_access_policies (event_mission_id, faction, squad, access_policy)
VALUES ('89b1b731-37a8-4926-901a-3c7ff7de5eb3', 'OPFOR', 'Recon',
        '{"grants":[{"conditions":[{"kind":"event_group","group_id":"00000000-0000-4000-b100-000000000002"}]}]}')
ON CONFLICT (event_mission_id, faction, squad) DO UPDATE SET access_policy = EXCLUDED.access_policy;

UPDATE orbat_slots SET access_policy =
    '{"grants":[{"conditions":[{"kind":"named_account","discord_id":"000000000000000006"}]}]}'
WHERE id = '00000000-0000-4000-5000-000000000013';

UPDATE events SET access_revision = 4, access_policy =
    '{"grants":[{"conditions":[{"kind":"tbd_member"}]},{"conditions":[{"kind":"event_group","group_id":"00000000-0000-4000-b100-000000000001"}]}]}'
WHERE id = 'c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7';

UPDATE event_reservation_quota_pools SET seat_limit = 2
WHERE event_id = 'c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7' AND quota_kind = 'guest';


-- ═══════════════════════════════════════════════════════════════════════════
-- §10 Audit log. Explicit ids on a bigserial column, so the sequence has to be
--     dragged past them afterwards or the next real write collides — see the
--     setval at the end of this section.
--
--     Severities span all three enum values; one line has no actor (a system
--     action) and one has no target, which are the two Option/empty arms of the
--     row that a uniform seed would never produce.
-- ═══════════════════════════════════════════════════════════════════════════

INSERT INTO audit_logs (id, severity, actor_id, actor_name, action, message, target_type,
                        target_id, metadata, created_at)
VALUES
  (1, 'info', '000000000000000001', 'Dev Operator', 'mission.approve',
   'Dev Operator approved mission ''Operation Iron Veil''', 'mission',
   '00000000-0000-4000-c000-000000000001',
   '{"semver": "1.2.0", "previous_status": "pending_approval"}'::jsonb,
   '2026-06-11 10:02:00+00'),
  (2, 'info', '000000000000000001', 'Dev Operator', 'mission.approve',
   'Dev Operator approved mission ''Operation Static Line''', 'mission',
   '00000000-0000-4000-c000-000000000002', NULL, '2026-06-27 09:14:00+00'),
  (3, 'warn', '000000000000000002', 'Rhodes', 'user.warn',
   'Rhodes issued a warning to Kessler for friendly fire', 'user',
   '000000000000000006', '{"reason": "friendly fire", "count": 1}'::jsonb,
   '2026-06-28 21:15:00+00'),
  -- No actor: emitted by the telemetry ingest path, not a person.
  (4, 'warn', NULL, '', 'server.fps_drop',
   'Primary server FPS dropped below 20 (17.3) with 61 players connected', 'server',
   '00000000-0000-4000-d000-000000000001',
   '{"server_fps": 17.3, "player_count": 61, "threshold": 20}'::jsonb,
   '2026-07-04 21:03:12+00'),
  (5, 'warn', '000000000000000001', 'Dev Operator', 'mission.reject',
   'Dev Operator rejected mission ''Operation Glass House''', 'mission',
   '00000000-0000-4000-c000-000000000006',
   '{"reason": "ORBAT slot count mismatch"}'::jsonb, '2026-07-16 12:30:00+00'),
  (6, 'crit', '000000000000000001', 'Dev Operator', 'user.ban',
   'Dev Operator banned Kessler — repeated team-killing after two warnings', 'user',
   '000000000000000006',
   '{"warnings": 2, "team_kills": 3, "permanent": true}'::jsonb,
   '2026-07-11 23:47:19+00'),
  (7, 'info', '000000000000000001', 'Dev Operator', 'modpack.publish',
   'Dev Operator published modpack ''Core Modern Expansion'' v2.1', 'modpack',
   '00000000-0000-4000-a000-000000000001',
   '{"version": "2.1", "size_bytes": 48532275200}'::jsonb, '2026-07-22 16:41:03+00'),
  -- No target at all: a login event references nothing but its actor.
  (8, 'info', '000000000000000003', 'Vance', 'auth.login',
   'Vance signed in from a new device', NULL, NULL, NULL, '2026-07-25 21:03:55+00'),
  (9, 'info', '000000000000000002', 'Rhodes', 'orbat.reserve',
   'Rhodes reserved squad ''Bravo'' on Operation Byte Parity Night', 'event_mission',
   '89b1b731-37a8-4926-901a-3c7ff7de5eb3', '{"squad": "Bravo"}'::jsonb,
   '2026-07-18 08:30:00+00'),
  (10, 'info', '000000000000000001', 'Dev Operator', 'announcement.publish',
   'Dev Operator published ''Modpack 2.1 is mandatory from Saturday''', 'announcement',
   '00000000-0000-4000-1000-000000000001', NULL, '2026-07-22 17:00:00+00')
ON CONFLICT (id) DO UPDATE SET
    severity = EXCLUDED.severity, actor_id = EXCLUDED.actor_id,
    actor_name = EXCLUDED.actor_name, action = EXCLUDED.action, message = EXCLUDED.message,
    target_type = EXCLUDED.target_type, target_id = EXCLUDED.target_id,
    metadata = EXCLUDED.metadata, created_at = EXCLUDED.created_at;

-- Explicit ids bypass the sequence; without this the next audit write reuses id 1.
SELECT setval('audit_logs_id_seq', (SELECT max(id) FROM audit_logs), true);


-- ═══════════════════════════════════════════════════════════════════════════
-- §11 Faction library. GET /factions is scoped to the CALLING mission_maker's
--     own rows, so every row here is owned by the dev-login operator — a
--     faction owned by anyone else is invisible to the capture and pointless.
-- ═══════════════════════════════════════════════════════════════════════════

INSERT INTO user_factions (id, owner_id, side, name, doc, created_at, updated_at)
VALUES
  ('00000000-0000-4000-b100-000000000001', '000000000000000001', 'BLUFOR', 'US Army — Light Infantry',
   '{"side":"BLUFOR","name":"US Army — Light Infantry","emblem":"us_army","roles":[{"role":"Squad Leader","tag":"SL","character":"{0B3167BB0FB68110}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_PL.et"},{"role":"Grenadier","character":"{84029128FA6F6BB9}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_GL.et"},{"role":"Combat Medic","tag":"MED","character":"{C9E4FEAF5AAC8D8C}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Medic.et"},{"role":"Rifleman","character":"{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman.et"}],"vehicles":[{"vehicle":"{1F1B4A0A5C8D9E2F}Prefabs/Vehicles/Wheeled/M998/M998_4x4.et","label":"M998 Humvee"},{"vehicle":"{2A2C5B1B6D9EAF30}Prefabs/Vehicles/Wheeled/M113/M113A3.et","label":"M113A3"}]}'::jsonb,
   '2026-06-14 11:00:00+00', '2026-07-12 16:20:00+00'),
  ('00000000-0000-4000-b100-000000000002', '000000000000000001', 'OPFOR', 'USSR — Motor Rifle',
   '{"side":"OPFOR","name":"USSR — Motor Rifle","emblem":"ussr","roles":[{"role":"Team Leader","tag":"TL","character":"{9C1D2E3F4A5B6C7D}Prefabs/Characters/Factions/OPFOR/USSR/Character_USSR_TL.et"},{"role":"Machine Gunner","character":"{8B0C1D2E3F4A5B6C}Prefabs/Characters/Factions/OPFOR/USSR/Character_USSR_MG.et"}],"vehicles":[{"vehicle":"{3B3D6C2C7EAFB041}Prefabs/Vehicles/Wheeled/BTR70/BTR70.et","label":"BTR-70"}]}'::jsonb,
   '2026-06-14 11:30:00+00', '2026-06-14 11:30:00+00'),
  -- No emblem, no vehicles, one role: the minimum viable faction doc, so the
  -- editor is forced through its empty-collection branches.
  ('00000000-0000-4000-b100-000000000003', '000000000000000001', 'INDFOR', 'Local Militia',
   '{"side":"INDFOR","name":"Local Militia","roles":[{"role":"Fighter","character":"{7A9B0C1D2E3F4A5B}Prefabs/Characters/Factions/INDFOR/Militia/Character_Militia_Rifleman.et"}],"vehicles":[]}'::jsonb,
   '2026-07-09 20:15:00+00', '2026-07-09 20:15:00+00')
ON CONFLICT (id) DO UPDATE SET
    owner_id = EXCLUDED.owner_id, side = EXCLUDED.side, name = EXCLUDED.name,
    doc = EXCLUDED.doc, created_at = EXCLUDED.created_at, updated_at = EXCLUDED.updated_at;


-- ═══════════════════════════════════════════════════════════════════════════
-- §12 Rebuild the leaderboard. GET /leaderboards reads the materialized view,
--     never match_player_stats directly — skip this and the board stays empty
--     with a fully populated stats table underneath it.
-- ═══════════════════════════════════════════════════════════════════════════

REFRESH MATERIALIZED VIEW leaderboard_totals;

-- ═══════════════════════════════════════════════════════════════════════════
-- §13 Mission artifacts, reviews and deployments, and the fleet ledger behind
--     them. The two artifacts are exactly what the compiler produced for the two
--     compilable versions below (bytes and digests copied from a compile against
--     this seed), pinned under fixed ids and timestamps so the review, artifact,
--     workspace, deployment and command goldens reproduce byte for byte. Iron Veil
--     carries a rejected review, a thread comment and a conditional approval, and
--     is deployed; Cold Anvil sits in the approvals queue with a pending review,
--     beside Paper Tiger, which predates reviews and has none. The primary server
--     ran the approved artifact in one runtime session; a later in-game
--     deployment failed because no host agent claimed its restart.
-- ═══════════════════════════════════════════════════════════════════════════

INSERT INTO mission_versions (id, mission_id, semver, json_payload, editor_notes, created_by, created_at)
VALUES
  ('00000000-0000-4000-8000-000000000011', '00000000-0000-4000-c000-000000000001', '1.3.0', '{"editor":{"factions":[{"id":"f1","key":"BLUFOR","name":"US Army","squadIds":["sq1"]}],"squads":[{"id":"sq1","factionId":"f1","callsign":"Alpha","name":"A 1-1","slotIds":["s1"]}],"slots":[{"id":"s1","squadId":"sq1","index":0,"role":"SL","position":{"x":4839.2,"y":6620.8,"z":0,"rotation":270}}],"editorLayers":[]}}'::jsonb,
   'Placed the assault section', '000000000000000003', '2026-07-24 10:00:00+00'),
  ('00000000-0000-4000-8000-000000000014', '00000000-0000-4000-c000-000000000004', '0.4.0', '{"editor":{"factions":[{"id":"f1","key":"BLUFOR","name":"US Army","squadIds":["sq1"]}],"squads":[{"id":"sq1","factionId":"f1","callsign":"Alpha","name":"A 1-1","slotIds":["s1"]}],"slots":[{"id":"s1","squadId":"sq1","index":0,"role":"SL","position":{"x":4839.2,"y":6620.8,"z":0,"rotation":270}}],"editorLayers":[]}}'::jsonb,
   'Resubmitted with placed seats', '000000000000000003', '2026-07-24 11:00:00+00')
ON CONFLICT (id) DO NOTHING;

INSERT INTO mission_artifacts (id, mission_id, mission_version_id, version_payload_sha256,
    metadata, metadata_sha256, catalog_sha256, modpack_id, modpack_version, compiler_version,
    schema_version, terrain, document, document_sha256, document_bytes, diagnostics,
    artifact_digest, created_by, created_at)
VALUES
  ('00000000-0000-4000-f000-000000000001', '00000000-0000-4000-c000-000000000001', '00000000-0000-4000-8000-000000000011', 'bb173e5b5b0e64ac00020ce933fa6cf5a9829548d7d10a8cc85a327653517b3e',
   '{"author":"000000000000000003","custom_terrain_name":"","game_mode":"pve_coop","id":"00000000-0000-4000-c000-000000000001","max_players":48,"terrain":"arland","time_of_day":"05:30:00","title":"Operation Iron Veil","weather":"overcast"}'::jsonb, 'ded05d09234b16a42ba8824117c75eb57e48b0aac46537d10cd4dc616453ef3b', '26fa7c98e52915f6f2d476d2373e9cbdc5ad09b2e64ce812e9c7b23c88947b07',
   '00000000-0000-4000-a000-000000000001', '2.1', 'website-map-engine 0.1.0', '1.1', 'arland',
   decode('7b22736368656d6156657273696f6e223a22312e31222c226d657461223a7b226964223a226d736e5f3030303030303030303030303430303063303030303030303030303030303031222c226e616d65223a224f7065726174696f6e2049726f6e205665696c222c22617574686f72223a22303030303030303030303030303030303033222c227465727261696e223a2261726c616e64222c2274656d706c6174654964223a22656469746f725f7631222c22706c6179657252616e6765223a5b312c34385d7d2c22656e7669726f6e6d656e74223a7b226461746554696d65223a22313938392d30362d31345430353a33303a30305a222c2277656174686572507265736574223a226f76657263617374227d2c2266616374696f6e73223a5b7b226b6579223a22626c75666f72222c22646973706c61794e616d65223a2255532041726d79222c227072657365744964223a227072657365743a75735f61726d795f38326e64222c227469636b657473223a307d5d2c226f72626174223a7b22626c75666f72223a7b2267726f757073223a5b7b2263616c6c7369676e223a22416c706861222c2274797065223a227269666c655f7371756164222c22726f6c6573223a5b7b22736c6f74223a22534c222c226b6974223a226b69743a75735f7269666c656d616e222c22636f756e74223a317d5d7d5d7d7d2c22736c6f7473223a5b7b226964223a22626c75666f723a416c7068613a534c3a30222c22756964223a227331222c2266616374696f6e223a22626c75666f72222c2267726f757043616c6c7369676e223a22416c706861222c22726f6c65223a22534c222c226b6974223a226b69743a75735f7269666c656d616e222c2278223a343833392e322c227a223a363632302e382c2268656164696e67446567223a3237302e307d5d2c22726164696f506c616e223a7b226e657473223a5b7b226964223a226e65743a626c75666f725f636d64222c226c6162656c223a2255532041726d7920436f6d6d616e64222c22667265714d487a223a33302e302c2266616374696f6e223a22626c75666f72222c2272616e6765223a226c6f6e67227d2c7b226964223a226e65743a626c75666f725f616c706861222c226c6162656c223a22416c706861222c22667265714d487a223a33302e352c2266616374696f6e223a22626c75666f72227d5d7d2c227a6f6e6573223a5b7b226964223a227a5f737061776e5f626c75666f72222c2274797065223a22737061776e222c2266616374696f6e223a22626c75666f72222c227368617065223a7b22636972636c65223a7b2278223a343833392e322c227a223a363632302e382c2272223a3135302e307d7d7d2c7b226964223a227a5f626f756e6473222c2274797065223a22626f756e64617279222c227368617065223a7b22706f6c79676f6e223a5b5b302e302c302e305d2c5b343039362e302c302e305d2c5b343039362e302c343039362e305d2c5b302e302c343039362e305d5d7d7d5d2c22666c6f77223a7b226272696566696e675365636f6e6473223a3630302c227361666553746172745365636f6e6473223a3330302c2274696d654c696d69745365636f6e6473223a353430302c226a6970223a22756e74696c5f7361666573746172745f656e64227d2c2277696e436f6e646974696f6e73223a7b226d6f6465223a22617474726974696f6e222c22656e644f6e223a5b2274696d655f6c696d6974225d7d7d', 'hex'),
   'be02eddd923ceab2751a2634cd78bec2c49b919c07300742fcfa7b78eef76291', 1271, '[]'::jsonb, '2534548d0f87ee73f27c582279483e82883a5f97f2a6576a43ed913b188e2791', '000000000000000003', '2026-07-24 10:05:00+00'),
  ('00000000-0000-4000-f000-000000000004', '00000000-0000-4000-c000-000000000004', '00000000-0000-4000-8000-000000000014', 'bb173e5b5b0e64ac00020ce933fa6cf5a9829548d7d10a8cc85a327653517b3e',
   '{"author":"000000000000000003","custom_terrain_name":"","game_mode":"pve_coop","id":"00000000-0000-4000-c000-000000000004","max_players":32,"terrain":"everon","time_of_day":"03:15:00","title":"Operation Cold Anvil","weather":"heavy_rain"}'::jsonb, 'f34053e5497da9c897a19d4ba986f2f86f0661e2c2dda3ab21da5960518e2128', '26fa7c98e52915f6f2d476d2373e9cbdc5ad09b2e64ce812e9c7b23c88947b07',
   '00000000-0000-4000-a000-000000000001', '2.1', 'website-map-engine 0.1.0', '1.1', 'everon',
   decode('7b22736368656d6156657273696f6e223a22312e31222c226d657461223a7b226964223a226d736e5f3030303030303030303030303430303063303030303030303030303030303034222c226e616d65223a224f7065726174696f6e20436f6c6420416e76696c222c22617574686f72223a22303030303030303030303030303030303033222c227465727261696e223a22657665726f6e222c2274656d706c6174654964223a22656469746f725f7631222c22706c6179657252616e6765223a5b312c33325d7d2c22656e7669726f6e6d656e74223a7b226461746554696d65223a22313938392d30362d31345430333a31353a30305a222c2277656174686572507265736574223a2268656176795f7261696e227d2c2266616374696f6e73223a5b7b226b6579223a22626c75666f72222c22646973706c61794e616d65223a2255532041726d79222c227072657365744964223a227072657365743a75735f61726d795f38326e64222c227469636b657473223a307d5d2c226f72626174223a7b22626c75666f72223a7b2267726f757073223a5b7b2263616c6c7369676e223a22416c706861222c2274797065223a227269666c655f7371756164222c22726f6c6573223a5b7b22736c6f74223a22534c222c226b6974223a226b69743a75735f7269666c656d616e222c22636f756e74223a317d5d7d5d7d7d2c22736c6f7473223a5b7b226964223a22626c75666f723a416c7068613a534c3a30222c22756964223a227331222c2266616374696f6e223a22626c75666f72222c2267726f757043616c6c7369676e223a22416c706861222c22726f6c65223a22534c222c226b6974223a226b69743a75735f7269666c656d616e222c2278223a343833392e322c227a223a363632302e382c2268656164696e67446567223a3237302e307d5d2c22726164696f506c616e223a7b226e657473223a5b7b226964223a226e65743a626c75666f725f636d64222c226c6162656c223a2255532041726d7920436f6d6d616e64222c22667265714d487a223a33302e302c2266616374696f6e223a22626c75666f72222c2272616e6765223a226c6f6e67227d2c7b226964223a226e65743a626c75666f725f616c706861222c226c6162656c223a22416c706861222c22667265714d487a223a33302e352c2266616374696f6e223a22626c75666f72227d5d7d2c227a6f6e6573223a5b7b226964223a227a5f737061776e5f626c75666f72222c2274797065223a22737061776e222c2266616374696f6e223a22626c75666f72222c227368617065223a7b22636972636c65223a7b2278223a343833392e322c227a223a363632302e382c2272223a3135302e307d7d7d2c7b226964223a227a5f626f756e6473222c2274797065223a22626f756e64617279222c227368617065223a7b22706f6c79676f6e223a5b5b302e302c302e305d2c5b31323830302e302c302e305d2c5b31323830302e302c31323830302e305d2c5b302e302c31323830302e305d5d7d7d5d2c22666c6f77223a7b226272696566696e675365636f6e6473223a3630302c227361666553746172745365636f6e6473223a3330302c2274696d654c696d69745365636f6e6473223a353430302c226a6970223a22756e74696c5f7361666573746172745f656e64227d2c2277696e436f6e646974696f6e73223a7b226d6f6465223a22617474726974696f6e222c22656e644f6e223a5b2274696d655f6c696d6974225d7d7d', 'hex'),
   '815fa594db434d9a8ffa297dda126ddab4a6493d3fa1db1c62525abe4f409739', 1278, '[]'::jsonb, 'bcfb3b1c4109fe03ac0292372a3fdfb86052d44979c2f2638e7e419e7f234577', '000000000000000003', '2026-07-24 11:05:00+00')
ON CONFLICT (id) DO NOTHING;

INSERT INTO mission_reviews (id, mission_id, artifact_id, submitted_by, submitted_at, state, decided_by, decided_at)
VALUES
  ('00000000-0000-4000-f100-000000000001', '00000000-0000-4000-c000-000000000001', '00000000-0000-4000-f000-000000000001', '000000000000000003',
   '2026-07-24 10:05:00+00', 'rejected', '000000000000000001', '2026-07-24 12:00:00+00'),
  ('00000000-0000-4000-f100-000000000002', '00000000-0000-4000-c000-000000000001', '00000000-0000-4000-f000-000000000001', '000000000000000003',
   '2026-07-24 13:00:00+00', 'approved_with_conditions', '000000000000000001', '2026-07-24 15:00:00+00'),
  ('00000000-0000-4000-f100-000000000004', '00000000-0000-4000-c000-000000000004', '00000000-0000-4000-f000-000000000004', '000000000000000003',
   '2026-07-24 11:05:00+00', 'pending', NULL, NULL)
ON CONFLICT (id) DO NOTHING;

INSERT INTO mission_review_comments (id, mission_id, review_id, mission_version_id, artifact_id,
    author_id, kind, body, created_at)
VALUES
  ('00000000-0000-4000-f500-000000000001', '00000000-0000-4000-c000-000000000001', '00000000-0000-4000-f100-000000000001',
   '00000000-0000-4000-8000-000000000011', '00000000-0000-4000-f000-000000000001', '000000000000000001', 'rejection',
   'The extraction helicopter spawns inside the minefield.', '2026-07-24 12:00:00+00'),
  ('00000000-0000-4000-f500-000000000002', '00000000-0000-4000-c000-000000000001', NULL, '00000000-0000-4000-8000-000000000011', '00000000-0000-4000-f000-000000000001', '000000000000000003',
   'comment', 'Moved the helicopter pad north of the quarry.', '2026-07-24 12:30:00+00'),
  ('00000000-0000-4000-f500-000000000003', '00000000-0000-4000-c000-000000000001', '00000000-0000-4000-f100-000000000002',
   '00000000-0000-4000-8000-000000000011', '00000000-0000-4000-f000-000000000001', '000000000000000001', 'approval_conditions',
   'Night rotation only until the BTR patrol is retuned.', '2026-07-24 15:00:00+00')
ON CONFLICT (id) DO NOTHING;

UPDATE missions SET current_version_id = '00000000-0000-4000-8000-000000000011', approved_artifact_id = '00000000-0000-4000-f000-000000000001', status = 'live',
    rejection_reason = '', reviewed_by = '000000000000000001', reviewed_at = '2026-07-24 15:00:00+00',
    updated_at = '2026-07-24 15:00:00+00'
WHERE id = '00000000-0000-4000-c000-000000000001';
UPDATE missions SET current_version_id = '00000000-0000-4000-8000-000000000014', updated_at = '2026-07-24 11:05:00+00'
WHERE id = '00000000-0000-4000-c000-000000000004';

INSERT INTO fleet_scenarios (terrain_key, scenario_id, display_name, updated_by, updated_at)
VALUES
  ('arland', '{1111222233334444}Missions/TBD_Arland.conf', 'Arland', '000000000000000001', '2026-07-24 09:00:00+00'),
  ('everon', '{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf', 'Everon', '000000000000000001', '2026-07-24 09:00:00+00')
ON CONFLICT (terrain_key) DO NOTHING;

INSERT INTO server_machine_credentials (id, server_id, executor_kind, secret_sha256, label,
                                        created_by, created_at, last_used_at)
VALUES ('00000000-0000-4000-e000-000000000003', '00000000-0000-4000-d000-000000000001', 'host_agent', 'c04532ac4e9a8208bdc3121386db93202f39c3c56de4d0b78130ff317e5e21d9', 'Primary host agent',
        '000000000000000001', '2026-07-18 10:05:00+00', '2026-07-24 16:01:00+00')
ON CONFLICT (id) DO NOTHING;

INSERT INTO server_runtime_sessions (id, server_id, credential_id, generation, started_at,
    last_heartbeat_at, last_sequence, ended_at, end_reason, loaded_artifact_id, loaded_artifact_sha256)
VALUES ('00000000-0000-4000-f300-000000000001', '00000000-0000-4000-d000-000000000001', '00000000-0000-4000-e000-000000000001', 1, '2026-07-24 16:04:00+00',
        '2026-07-24 19:55:00+00', 940, '2026-07-24 20:00:00+00', 'ended_by_runtime', '00000000-0000-4000-f000-000000000001', 'be02eddd923ceab2751a2634cd78bec2c49b919c07300742fcfa7b78eef76291')
ON CONFLICT (id) DO NOTHING;

INSERT INTO fleet_commands (id, server_id, executor_kind, action, arguments, idempotent,
    process_changing, requested_by, requested_at, expires_at, state, fencing_token, attempts,
    claimed_by, claimed_at, executing_at, finished_at, outcome, failure_reason)
VALUES
  ('00000000-0000-4000-f200-000000000001', '00000000-0000-4000-d000-000000000001', 'host_agent', 'restart_with_mission', '{"deployment_id": "00000000-0000-4000-f400-000000000001", "artifact_id": "00000000-0000-4000-f000-000000000001", "artifact_sha256": "be02eddd923ceab2751a2634cd78bec2c49b919c07300742fcfa7b78eef76291", "scenario_id": "{1111222233334444}Missions/TBD_Arland.conf"}'::jsonb, false, true,
   '000000000000000001', '2026-07-24 16:00:00+00', '2026-07-24 16:05:00+00', 'succeeded', 1, 1,
   NULL, '2026-07-24 16:00:05+00', '2026-07-24 16:00:06+00', '2026-07-24 16:01:00+00',
   '{"scenario_id": "{1111222233334444}Missions/TBD_Arland.conf", "unit_active_state": "active", "config_path": "/srv/reforger/server-config.json"}'::jsonb, NULL),
  ('00000000-0000-4000-f200-000000000002', '00000000-0000-4000-d000-000000000001', 'mod_runtime', 'broadcast',
   '{"message": "Server restarts in 10 minutes for Operation Iron Veil"}'::jsonb, false, false,
   '000000000000000001', '2026-07-24 15:45:00+00', '2026-07-24 15:50:00+00', 'failed', 1, 1,
   NULL, '2026-07-24 15:45:03+00', '2026-07-24 15:45:04+00',
   '2026-07-24 15:45:05+00', NULL, 'the chat channel was unavailable'),
  -- Queued far into the future so the reconciler never expires it under a capture.
  ('00000000-0000-4000-f200-000000000003', '00000000-0000-4000-d000-000000000001', 'host_agent', 'list_players', '{}'::jsonb, true, false,
   '000000000000000001', '2026-07-25 08:00:00+00', '2027-07-25 08:05:00+00', 'queued', 0, 0,
   NULL, NULL, NULL, NULL, NULL, NULL),
  ('00000000-0000-4000-f200-000000000004', '00000000-0000-4000-d000-000000000001', 'host_agent', 'restart_with_mission', '{"deployment_id": "00000000-0000-4000-f400-000000000002", "artifact_id": "00000000-0000-4000-f000-000000000001", "artifact_sha256": "be02eddd923ceab2751a2634cd78bec2c49b919c07300742fcfa7b78eef76291", "scenario_id": "{1111222233334444}Missions/TBD_Arland.conf"}'::jsonb, false, true,
   '000000000000000001', '2026-07-25 09:00:00+00', '2026-07-25 09:05:00+00', 'expired', 0, 0,
   NULL, NULL, NULL, '2026-07-25 09:05:00+00', NULL, 'no executor completed the command before it expired')
ON CONFLICT (id) DO NOTHING;

-- Iron Veil's main-effort event runs on the primary server, and its event mission carries the
-- one seat the approved artifact compiles (BLUFOR / A 1-1 / position 0 / SL, slot uid s1), so
-- the confirmed deployment binds it; the failed in-game request names no event mission.
UPDATE events SET server_id = '00000000-0000-4000-d000-000000000001'
WHERE id = '00000000-0000-4000-7000-000000000001';

INSERT INTO orbat_slots (id, event_mission_id, faction, squad, callsign, role, loadout, tag,
                         slot_index, assigned_to, assigned_at)
VALUES ('00000000-0000-4000-5000-000000000020', '00000000-0000-4000-6000-000000000001',
        'BLUFOR', 'A 1-1', 'Alpha', 'SL', NULL, NULL, 0, NULL, NULL)
ON CONFLICT (id) DO NOTHING;

INSERT INTO mission_deployments (id, server_id, mission_id, artifact_id, event_mission_id,
    terrain_key, scenario_id, transition, fleet_command_id, requested_by, requested_via,
    requested_at, deadline_at, state, confirmed_runtime_session_id, finished_at, failure_reason)
VALUES
  ('00000000-0000-4000-f400-000000000001', '00000000-0000-4000-d000-000000000001',
   '00000000-0000-4000-c000-000000000001', '00000000-0000-4000-f000-000000000001',
   '00000000-0000-4000-6000-000000000001', 'arland', '{1111222233334444}Missions/TBD_Arland.conf',
   'host_restart', '00000000-0000-4000-f200-000000000001', '000000000000000001', 'web',
   '2026-07-24 16:00:00+00', '2026-07-24 16:20:00+00', 'confirmed',
   '00000000-0000-4000-f300-000000000001', '2026-07-24 16:04:30+00', NULL),
  ('00000000-0000-4000-f400-000000000002', '00000000-0000-4000-d000-000000000001',
   '00000000-0000-4000-c000-000000000001', '00000000-0000-4000-f000-000000000001',
   NULL, 'arland', '{1111222233334444}Missions/TBD_Arland.conf',
   'host_restart', '00000000-0000-4000-f200-000000000004', '000000000000000001', 'game_runtime',
   '2026-07-25 09:00:00+00', '2026-07-25 09:20:00+00', 'failed', NULL, '2026-07-25 09:20:00+00',
   'the restart_with_mission command ended expired: no executor completed the command before it expired')
ON CONFLICT (id) DO NOTHING;

INSERT INTO mission_deployment_slots (deployment_id, orbat_slot_id, slot_uid)
VALUES ('00000000-0000-4000-f400-000000000001', '00000000-0000-4000-5000-000000000020', 's1')
ON CONFLICT DO NOTHING;


-- ═══════════════════════════════════════════════════════════════════════════
-- REPRODUCING THE FIXTURES
--
--   1. createdb, then boot the API against it (it migrates on boot):
--        DATABASE_URL=postgres://tbd:tbd@localhost:5434/<db>?sslmode=disable \
--        JWT_SECRET=<anything> APP_ENV=development PORT=<port> \
--        DISCORD_GUILD_ID=100000000000000001 \
--        cargo run -p website-api --bin api
--      The access-participants golden names the configured guild, so the guild
--      id is part of the recipe.
--      Do NOT capture against a long-running :8080 — that process may be a
--      deleted-inode binary from an unrelated build. Boot your own and know
--      what you are talking to.
--   2. curl the dev-login redirect and take access_token out of the fragment:
--        GET /api/v1/auth/dev-login?role=admin
--   3. psql -f seeds/registry_dev.sql, then psql -f seeds/content_golden.sql.
--      THIS ORDER MATTERS: dev-login stamps the operator's last_login_at with
--      the wall clock, and §1 above pins it back to the committed value.
--   4. GET each path in tests/fixtures/api/_index.tsv with that bearer token
--      and write the body to its fixture file, in index order: the two writes
--      at its end run last, because each changes what the reads before it see.
--      PUT /events/c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7/access-policy restates the
--      seeded policy with {"expected_access_revision": 4, "policy": <the §9 policy>}.
--      POST /event-missions/89b1b731-37a8-4926-901a-3c7ff7de5eb3/waitlist/promote
--      (no body) promotes only while the operation accepts registrations and its
--      waiter is available: the seeded waiter (…006) is banned and the operation
--      has passed, so that golden is captured after moving the operation and its
--      mission 30 days ahead and lifting the waiter's ban in the capture database
--      only. It promotes the waiter into the first free seat (…5000-000000000006).
--   5. §13's artifact rows are the compiler's output for its two versions. A
--      change to the compiler, the document schema, the catalog or a modpack
--      changes their bytes and digests: seed a fresh database up to §12, insert
--      the two §13 versions as the missions' current versions (Iron Veil as a
--      draft), POST /missions/:id/submit for both, and copy each resulting
--      mission_artifacts row — document as hex, digests, metadata, diagnostics
--      — into §13 under its pinned id and timestamp.
--
-- GET__registry.json is NOT reproducible this way and is not captured by this
-- recipe: registry_dev.sql lets Postgres generate the registry_items ids, so a
-- re-seed produces different uuids on every run. That fixture stays as
-- committed until someone pins those ids.
-- ═══════════════════════════════════════════════════════════════════════════
