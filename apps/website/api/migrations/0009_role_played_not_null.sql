-- T-228 (follow-up, from T-325's finding): close the `models` / column NULL mismatch at the
-- schema, for the one column where it can be closed from `migrations/` alone.
--
-- `models::MatchPlayerStat.role_played` is a non-optional `String` against a NULLABLE column
-- (0001_initial_schema.sql:256). Nothing 500s today only because deployments.rs:127 wraps the
-- read in `COALESCE(role_played, '')`. Drop that COALESCE in a probe and the row fails to
-- decode with `unexpected null; try decoding as an Option` — the mismatch is real, it is just
-- currently held down by one handler remembering to paper over it.
--
-- WHY NOT NULL RATHER THAN `Option<String>`. The wire cannot express the distinction that
-- `Option` would add. `DeploymentRecord` marks these fields
-- `skip_serializing_if = "String::is_empty"`, so `''` already serializes as an ABSENT KEY —
-- the same encoding `None` would produce. The committed golden proves it:
-- frontend/tests/fixtures/api/GET__me__deployments.json `service_history[0]` has no
-- `aar_replay_url` key at all, and it was generated from a row whose column is NULL.
-- So `None` and `''` are two spellings of one observable state, and collapsing them onto `''`
-- keeps the wire byte-identical while making the type honest.
--
-- WHY THIS COLUMN IS SAFE TO CONSTRAIN (checked against the operator's live database, not
-- assumed):
--   * 0 of 11 `match_player_stats` rows have a NULL `role_played`.
--   * The only writer cannot produce one — `PlayerStatInput.role_played` is a plain `String`,
--     not an `Option` (telemetry.rs:310), bound directly at telemetry.rs:401.
--   * seeds/content_golden.sql inserts a real role string in all 11 rows; none is NULL.
-- The backfill below is therefore a no-op on today's data. It stays because a migration has to
-- be correct against any database that reaches it, not just this one — a dev DB with a
-- hand-inserted NULL must not turn `SET NOT NULL` into a failed boot (bin/api.rs:26).
--
-- Backfill FIRST, then constrain, and sqlx runs the file in one transaction, so a database
-- that does have NULLs is either fully migrated or untouched — never constrained-and-failing.
-- Both statements are naturally replayable: the UPDATE matches zero rows on a second run, and
-- `SET DEFAULT` / `SET NOT NULL` are no-ops when already in force.

UPDATE match_player_stats SET role_played = '' WHERE role_played IS NULL;

ALTER TABLE public.match_player_stats
    ALTER COLUMN role_played SET DEFAULT '',
    ALTER COLUMN role_played SET NOT NULL;

-- ── NOT CONSTRAINED HERE, AND NOT FOR SEMANTIC REASONS ────────────────────────────────────
-- `matches.winning_faction` and `matches.aar_replay_url` were the other two columns in the
-- request. Semantically they belong here: the mod OMITS `winning_faction` from the payload
-- when there is no winner (TBD_ResultsReporter.c:18,546), `outcome` already carries
-- pending/failure/aborted, and the create path at telemetry.rs:519 ALREADY writes
-- `COALESCE($8, '')` — `''` is the canonical "no winner" on the only write path there is.
--
-- They are blocked on a file this slice does not own. seeds/content_golden.sql:358-369 inserts
-- literal NULL into `aar_replay_url` (2 rows) and `winning_faction` (1 row), under
-- `ON CONFLICT (id) DO UPDATE SET … = EXCLUDED.…` (:374-375). That makes BOTH halves fail:
--   * WITH `NOT NULL`, the next `psql < seeds/content_golden.sql` dies on the INSERT at :355.
--   * WITHOUT it, a backfill here is self-defeating — re-running the seed writes the NULLs
--     straight back over it. The seed is where the operator's live NULLs came from.
-- So the seed must change (three `NULL` → `''`) in the same landing as the constraint. When it
-- does, this is the whole migration:
--
--   UPDATE matches SET winning_faction = '' WHERE winning_faction IS NULL;
--   UPDATE matches SET aar_replay_url  = '' WHERE aar_replay_url  IS NULL;
--   ALTER TABLE public.matches
--       ALTER COLUMN winning_faction SET DEFAULT '', ALTER COLUMN winning_faction SET NOT NULL,
--       ALTER COLUMN aar_replay_url  SET DEFAULT '', ALTER COLUMN aar_replay_url  SET NOT NULL;
--
-- `match_player_stats.command_win` is deliberately left NULLABLE and is not part of this:
-- NULL there is a genuine third state ("not a command slot / not adjudicated"), distinct from
-- `false`, and it is typed `Option<bool>` on both sides already (telemetry.rs:307-308).
