-- T-331: close two NULL holes that were diagnosed but blocked on seeds/migrations.
--
-- 1) matches.winning_faction / matches.aar_replay_url
--    DDL already drafted as a comment at the foot of 0009_role_played_not_null.sql
--    (:56-60). It could not land there because seeds/content_golden.sql inserted
--    literal NULL into those columns under ON CONFLICT DO UPDATE — a backfill-only
--    migration was self-defeating. The seed now writes '' (three edits). Apply the
--    constraint here in the same landing.
--    Semantically '' is canonical: the mod omits winning_faction when there is no
--    winner; telemetry.rs create already COALESCE($8, ''). Do NOT touch
--    match_player_stats.command_win (NULL is a real third state).
--
-- 2) missions.created_at / missions.updated_at
--    0001_initial_schema.sql:374-375 declared them nullable with no DEFAULT, unlike
--    0003_registry_compat / 0006_user_factions (DEFAULT now() NOT NULL). Hand-written
--    INSERTs that omit the columns store NULL; T-330 needed a three-link COALESCE
--    terminating in the Go zero time because both ends can be NULL. Backfill, then
--    DEFAULT now() + NOT NULL.
--
-- 3) Partial unique on orbat_slots(event_mission_id, assigned_to) WHERE assigned_to
--    IS NOT NULL was deferred here at T-331: content_golden double-seat was already
--    fixed, but tests/events.rs still seeded legacy two-seat state for T-318 recovery,
--    and a partial unique cannot be DEFERRABLE. That index landed later in
--    0017_orbat_slots_assigned_partial_unique.sql (T-511), which also retired the
--    two-seat seed.

UPDATE matches SET winning_faction = '' WHERE winning_faction IS NULL;
UPDATE matches SET aar_replay_url = '' WHERE aar_replay_url IS NULL;

ALTER TABLE public.matches
    ALTER COLUMN winning_faction SET DEFAULT '',
    ALTER COLUMN winning_faction SET NOT NULL,
    ALTER COLUMN aar_replay_url SET DEFAULT '',
    ALTER COLUMN aar_replay_url SET NOT NULL;

UPDATE missions
SET created_at = COALESCE(created_at, updated_at, now())
WHERE created_at IS NULL;

UPDATE missions
SET updated_at = COALESCE(updated_at, created_at, now())
WHERE updated_at IS NULL;

ALTER TABLE public.missions
    ALTER COLUMN created_at SET DEFAULT now(),
    ALTER COLUMN created_at SET NOT NULL,
    ALTER COLUMN updated_at SET DEFAULT now(),
    ALTER COLUMN updated_at SET NOT NULL;
