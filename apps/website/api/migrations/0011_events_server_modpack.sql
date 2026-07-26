-- T-260: per-event server + modpack binding.
--
-- The Event Hub chip (`event_hub.rs`) fetches `GET /modpacks/current`, so every operation
-- renders the same global pack regardless of which server it runs on. The `events` table
-- (0001_initial_schema.sql:207-221) has no `server_id` / `modpack_id` columns, and
-- handlers/events.rs + models/event.rs had zero such fields — create/update/get could not
-- carry a per-event binding even if the SPA asked.
--
-- FILENAME vs WAVE-PLAN OWNS: wave_plan.tsv still lists `0008_events_server_modpack.sql`,
-- but versions 0008–0010 are already shipped (`orbat_slot_faction`, `role_played_not_null`,
-- `backfill_aar_replay_url_scheme`). sqlx versions must be unique, so this lands as 0011.
--
-- House style matches `servers.required_modpack_id`: nullable uuid columns with **no**
-- FOREIGN KEY. The whole schema is FK-free today (T-262); adding REFERENCES here would be a
-- unilateral exception. Existence is enforced in the handler (same advisory check as
-- `handlers/servers.rs::require_modpack`), not by the database.
--
-- SAFE ON EXISTING ROWS: both columns default to NULL, so every pre-existing event keeps
-- working — Hub simply has no per-event ids until an admin PATCHes them.

ALTER TABLE public.events
    ADD COLUMN IF NOT EXISTS server_id uuid,
    ADD COLUMN IF NOT EXISTS modpack_id uuid;
