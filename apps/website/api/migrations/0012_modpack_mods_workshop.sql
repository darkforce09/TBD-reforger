-- T-271: columns so each modpack_mods row can express a Reforger `game.mods[]` entry.
--
-- Live schema (0001_initial_schema.sql:381-387) only had name / is_key_dependency /
-- sort_order. Reforger server configs need at least modId + name (see
-- scripts/mod/tbd-staging-server.config.json `game.mods[]` and deploy-staging.sh), and
-- optionally a version pin. Local Workbench GUIDs are distinct from Workshop modIds
-- (docs/mod/STAGING-SERVER.md) — keep both so a future renderer (T-288) can choose.
--
-- Naming: snake_case to match `modpacks.workshop_url` and registry schema `workshopId`
-- (DB column `workshop_id`). Empty string default (not NULL) matches the crate's
-- COALESCE-to-'' read pattern for optional text (`workshop_url`, vehicle fields).
--
-- SAFE ON EXISTING ROWS: all three columns default to '', so pre-T-271 rows stay
-- readable; GETs keep working. No FK added (schema is FK-free — T-262 / T-260 style).

ALTER TABLE public.modpack_mods
    ADD COLUMN IF NOT EXISTS workshop_id text NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS mod_guid text NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS version text NOT NULL DEFAULT '';
