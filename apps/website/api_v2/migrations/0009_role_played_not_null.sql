-- `match_player_stats.role_played` is NOT NULL with DEFAULT ''.
--
-- `MatchPlayerStat.role_played` is a non-optional `String` against what the initial schema
-- declared as a NULLABLE column. Nothing 500s only because every reader wraps the column in
-- `COALESCE(role_played, '')`; drop that COALESCE in a probe and the row fails to decode with
-- `unexpected null; try decoding as an Option` — the mismatch is real, it is just held down by
-- one handler remembering to paper over it.
--
-- WHY NOT NULL RATHER THAN `Option<String>`. The wire cannot express the distinction that
-- `Option` would add. The service record marks these fields
-- `skip_serializing_if = "String::is_empty"`, so `''` already serializes as an ABSENT KEY —
-- the same encoding `None` would produce. The committed golden proves it:
-- `GET__me__deployments.json` `service_history[0]` has no `aar_replay_url` key at all, and it
-- was generated from a row whose column is NULL. So `None` and `''` are two spellings of one
-- observable state, and collapsing them onto `''` keeps the wire byte-identical while making the
-- type honest.
--
-- WHY THIS COLUMN IS SAFE TO CONSTRAIN: the only writer cannot produce a NULL —
-- `PlayerStatInput.role_played` is a plain `String`, not an `Option`, bound directly — and
-- `seeds/content_golden.sql` inserts a real role string in every row. The backfill below stays
-- because a migration has to be correct against any database that reaches it, not just this
-- one — a dev DB with a hand-inserted NULL must not turn `SET NOT NULL` into a failed boot.
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
-- `matches.winning_faction` and `matches.aar_replay_url` follow the same rule: the mod OMITS
-- `winning_faction` from the payload when there is no winner, `outcome` already carries
-- pending/failure/aborted, and the create path ALREADY writes `COALESCE($8, '')` — `''` is the
-- canonical "no winner" on the only write path there is.
--
-- They cannot be constrained by this file alone: the golden seed of the time inserted literal
-- NULL into both columns under `ON CONFLICT (id) DO UPDATE SET … = EXCLUDED.…`, which makes
-- BOTH halves fail — WITH `NOT NULL` the next seed load dies on that INSERT, and WITHOUT it a
-- backfill is self-defeating because re-running the seed writes the NULLs straight back. The
-- seed change (three `NULL` → `''`) and the constraint land together in 0015.
--
-- `match_player_stats.command_win` is deliberately left NULLABLE and is not part of this:
-- NULL there is a genuine third state ("not a command slot / not adjudicated"), distinct from
-- `false`, and it is typed `Option<bool>` on both sides already.
