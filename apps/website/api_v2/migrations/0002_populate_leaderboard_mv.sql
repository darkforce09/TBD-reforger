-- Populate `leaderboard_totals` once.
--
-- The initial schema creates the materialized view `WITH NO DATA`, which errors on the first
-- SELECT ("materialized view has not been populated"). Telemetry ingest keeps it fresh afterwards
-- with REFRESH MATERIALIZED VIEW CONCURRENTLY (`command_center::services::leaderboard_view`).

REFRESH MATERIALIZED VIEW leaderboard_totals;
