-- 04_leaderboard_mv.sql
-- Runs AFTER AutoMigrate. The Global Leaderboards (5 tabs) are ORDER BY variants
-- over this aggregate. Refreshed (CONCURRENTLY) by the leaderboard service after
-- each match ingest. The UNIQUE index on discord_id is required for CONCURRENTLY.

CREATE MATERIALIZED VIEW IF NOT EXISTS leaderboard_totals AS
SELECT
    s.discord_id,
    SUM(s.kills)                          AS kills,
    SUM(s.deaths)                         AS deaths,
    CASE WHEN SUM(s.deaths) = 0 THEN SUM(s.kills)::numeric
         ELSE ROUND(SUM(s.kills)::numeric / SUM(s.deaths), 2) END AS kd_ratio,
    SUM(s.team_kills)                     AS team_kills,
    MAX(s.longest_kill_m)                 AS longest_kill_m,
    SUM(s.vehicles_destroyed)             AS vehicles_destroyed,
    COUNT(DISTINCT s.match_id)            AS missions_played,
    COUNT(*) FILTER (WHERE s.command_win) AS command_wins,
    NULLIF(COUNT(*) FILTER (WHERE s.is_command), 0) AS command_games,
    CASE WHEN COUNT(*) FILTER (WHERE s.is_command) = 0 THEN 0
         ELSE ROUND(
            COUNT(*) FILTER (WHERE s.command_win)::numeric
            / COUNT(*) FILTER (WHERE s.is_command), 3) END AS command_win_rate
FROM match_player_stats s
WHERE s.discord_id IS NOT NULL
GROUP BY s.discord_id;

CREATE UNIQUE INDEX IF NOT EXISTS idx_leaderboard_discord
    ON leaderboard_totals (discord_id);
