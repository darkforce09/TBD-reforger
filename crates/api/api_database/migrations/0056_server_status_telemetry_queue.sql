-- The last outbound telemetry queue reading a game runtime reported in a heartbeat: entries
-- waiting, capacity, entries dropped since the queue was created, the age of the oldest waiting
-- entry, and when the reading arrived. All five are set together or not at all.
ALTER TABLE public.server_statuses
    ADD COLUMN telemetry_queue_backlog bigint,
    ADD COLUMN telemetry_queue_capacity bigint,
    ADD COLUMN telemetry_queue_dropped_total bigint,
    ADD COLUMN telemetry_queue_oldest_age_seconds bigint,
    ADD COLUMN telemetry_queue_reported_at timestamptz,
    ADD CONSTRAINT server_statuses_telemetry_queue_complete CHECK (num_nonnulls(
        telemetry_queue_backlog, telemetry_queue_capacity, telemetry_queue_dropped_total,
        telemetry_queue_oldest_age_seconds, telemetry_queue_reported_at) IN (0, 5)),
    ADD CONSTRAINT server_statuses_telemetry_queue_bounds CHECK (
        telemetry_queue_backlog >= 0 AND telemetry_queue_capacity >= 0
        AND telemetry_queue_dropped_total >= 0 AND telemetry_queue_oldest_age_seconds >= 0
        AND telemetry_queue_backlog <= telemetry_queue_capacity);
