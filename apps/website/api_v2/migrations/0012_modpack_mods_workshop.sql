-- Columns so each `modpack_mods` row can express a Reforger `game.mods[]` entry.
--
-- The initial schema gave the table only name / is_key_dependency / sort_order. Reforger
-- server configs need at least modId + name (`game.mods[]` in the server config), and
-- optionally a version pin. Local Workbench GUIDs are distinct from Workshop modIds, so
-- both are kept and a renderer can choose.
--
-- Naming: snake_case to match `modpacks.workshop_url` and the registry schema's `workshopId`
-- (DB column `workshop_id`). Empty string default (not NULL) matches the crate's
-- COALESCE-to-'' read pattern for optional text (`workshop_url`, vehicle fields).
--
-- SAFE ON EXISTING ROWS: all three columns default to '', so earlier rows stay
-- readable; GETs keep working. The foreign keys arrive with 0018.


ALTER TABLE public.modpack_mods
    ADD COLUMN IF NOT EXISTS workshop_id text NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS mod_guid text NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS version text NOT NULL DEFAULT '';
