-- Two NULL holes closed at the schema.
--
-- 1) matches.winning_faction / matches.aar_replay_url
--    0009 explains why '' is canonical: the mod omits winning_faction when there is no
--    winner, and the create path already COALESCEs to ''. It could not constrain them
--    because the golden seed of the time inserted literal NULL into those columns under
--    ON CONFLICT DO UPDATE — a backfill-only migration was self-defeating. The seed now
--    writes '', so the constraint lands here in the same landing.
--    Do NOT touch match_player_stats.command_win (NULL is a real third state).
--
-- 2) missions.created_at / missions.updated_at
--    The initial schema declared them nullable with no DEFAULT, unlike the later tables
--    (DEFAULT now() NOT NULL). Hand-written INSERTs that omit the columns store NULL, and
--    every reader needed a COALESCE chain because both ends could be NULL. Backfill, then
--    DEFAULT now() + NOT NULL.
--
-- 3) The partial unique index on orbat_slots(event_mission_id, assigned_to) WHERE assigned_to
--    IS NOT NULL is 0017's: a partial unique index cannot be DEFERRABLE, and the seats a
--    populated database already double-booked have to be freed in the same transaction that
--    creates it.


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
