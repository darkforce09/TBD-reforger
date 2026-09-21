-- Absent `match_player_stats` counters are NULL ("not measured"), not 0.
--
-- The initial schema declared the counter columns NOT NULL DEFAULT 0 / DEFAULT false, so a
-- counters-absent INSERT — which deliberately omits those columns — materialised zeros on
-- FIRST insert. Stored 0 ≡ scored 0. leaderboard_totals SUMmed those zeros; an identity-only
-- row beside a full report poisoned the aggregate (measured: true deaths 4 → kd wrong when the
-- absent half claimed deaths=0).
--
-- The UPDATE half of ingest already holds (absent counters → the statement does not name the
-- columns). This migration fixes the INSERT half at the schema: absent = NULL.
--
-- BACKFILL DECISION: leave existing DEFAULT-0 rows as 0.
-- Under the old schema every written 0 claimed "measured, and it was none" — a genuine scored
-- zero cannot be told from an identity-only insert after the fact. Rewriting historical zeros
-- to NULL would invent "not measured" for rows that may have been real zero scorelines. New
-- identity-only inserts write NULL going forward.
--
-- `deaths` stays inside the optional counters block, never in the required identity core.


ALTER TABLE public.match_player_stats
    ALTER COLUMN kills DROP NOT NULL,
    ALTER COLUMN kills DROP DEFAULT,
    ALTER COLUMN deaths DROP NOT NULL,
    ALTER COLUMN deaths DROP DEFAULT,
    ALTER COLUMN team_kills DROP NOT NULL,
    ALTER COLUMN team_kills DROP DEFAULT,
    ALTER COLUMN longest_kill_m DROP NOT NULL,
    ALTER COLUMN longest_kill_m DROP DEFAULT,
    ALTER COLUMN vehicles_destroyed DROP NOT NULL,
    ALTER COLUMN vehicles_destroyed DROP DEFAULT,
    ALTER COLUMN is_command DROP NOT NULL,
    ALTER COLUMN is_command DROP DEFAULT;

-- Recreate leaderboard_totals NULL-aware: SUM/MAX ignore NULL; kd_ratio is NULL when
-- no row has a measured deaths reading (do not treat unmeasured as denominator 0).
DROP MATERIALIZED VIEW IF EXISTS public.leaderboard_totals;

CREATE MATERIALIZED VIEW public.leaderboard_totals AS
 SELECT discord_id,
    COALESCE(sum(kills), (0)::numeric) AS kills,
    COALESCE(sum(deaths), (0)::numeric) AS deaths,
        CASE
            WHEN (count(deaths) FILTER (WHERE (deaths IS NOT NULL)) = 0) THEN NULL::numeric
            WHEN (sum(deaths) = (0)::numeric) THEN COALESCE(sum(kills), (0)::numeric)
            ELSE round((COALESCE(sum(kills), (0)::numeric) / sum(deaths)), 2)
        END AS kd_ratio,
    COALESCE(sum(team_kills), (0)::numeric) AS team_kills,
    COALESCE(max(longest_kill_m), (0)::bigint) AS longest_kill_m,
    COALESCE(sum(vehicles_destroyed), (0)::numeric) AS vehicles_destroyed,
    count(DISTINCT match_id) AS missions_played,
    count(*) FILTER (WHERE command_win) AS command_wins,
    NULLIF(count(*) FILTER (WHERE (is_command IS TRUE)), 0) AS command_games,
        CASE
            WHEN (count(*) FILTER (WHERE (is_command IS TRUE)) = 0) THEN (0)::numeric
            ELSE round(((count(*) FILTER (WHERE command_win))::numeric / (count(*) FILTER (WHERE (is_command IS TRUE)))::numeric), 3)
        END AS command_win_rate
   FROM public.match_player_stats s
  WHERE (discord_id IS NOT NULL)
  GROUP BY discord_id
  WITH NO DATA;

CREATE UNIQUE INDEX idx_leaderboard_discord ON public.leaderboard_totals USING btree (discord_id);

REFRESH MATERIALIZED VIEW public.leaderboard_totals;
