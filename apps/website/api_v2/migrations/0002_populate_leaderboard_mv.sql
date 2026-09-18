-- T-145 Phase 1 addendum.
--
-- `pg_dump --schema-only` emits a materialized view as `CREATE MATERIALIZED VIEW …
-- WITH NO DATA` — i.e. UNPOPULATED, which errors on the first SELECT
-- ("materialized view has not been populated"). The Go pipeline created it populated
-- (`CREATE MATERIALIZED VIEW … AS SELECT …`), so this behavioral difference is invisible
-- to a schema-only pg_dump diff (both dumps show WITH NO DATA) but breaks the
-- leaderboard reads. Populate it once here; telemetry ingest keeps it fresh via
-- REFRESH MATERIALIZED VIEW CONCURRENTLY (db::refresh_leaderboard).
REFRESH MATERIALIZED VIEW leaderboard_totals;
