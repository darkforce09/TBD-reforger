-- Per-event server + modpack binding.
--
-- The Event Hub chip fetches `GET /modpacks/current`, so without these columns every
-- operation renders the same global pack regardless of which server it runs on: `events`
-- had no `server_id` / `modpack_id`, so create/update/get could not carry a per-event
-- binding even if the SPA asked.
--
-- House style matches `servers.required_modpack_id`: nullable uuid columns. Existence is
-- checked in the handler (`server_infrastructure::handlers::server_registry::require_modpack`)
-- and the foreign keys arrive with 0018.
--
-- SAFE ON EXISTING ROWS: both columns default to NULL, so every pre-existing event keeps
-- working — the Hub simply has no per-event ids until an admin PATCHes them.


ALTER TABLE public.events
    ADD COLUMN IF NOT EXISTS server_id uuid,
    ADD COLUMN IF NOT EXISTS modpack_id uuid;
