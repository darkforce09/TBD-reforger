-- Seed: discord_roles — maps Discord guild role snowflakes to web permission tiers.
--
-- This is DATA, not schema, so it is NOT run by the boot migration pipeline
-- (internal/db/db.go). Apply it explicitly after the DB is up and migrated:
--
--   make seed                       # applies this file via the compose db
--   # or: podman exec -i tbd_reforger_db psql -U tbd -d tbd_reforger < internal/db/seeds/discord_roles.sql
--
-- How resolution works (see internal/services/role_sync.go::resolveRole):
--   * A user's web role is the mapped_role of their HIGHEST-priority matching row.
--   * mapped_role NULL = cosmetic (no permission grant).
--   * A user with no matching mapped row falls back to 'enlisted' (the default),
--     so the 'Player' -> 'enlisted' row below is documentation, not a behaviour change.
--
-- The role IDs below are specific to the TBD Discord guild (DISCORD_GUILD_ID in
-- .env). For a different guild, replace them with that guild's role snowflakes.
-- Idempotent: re-running updates name/mapped_role/priority in place.

INSERT INTO discord_roles (discord_role_id, name, mapped_role, priority) VALUES
  ('1517285898817896559', 'Command Staff', 'admin',         100),
  ('1517286228851032115', 'Mission Maker', 'mission_maker',  50),
  -- Example squad-leader mapping; replace the snowflake with the real guild role id.
  ('1517290000000000000', 'Squad Leader',  'leader',         30),
  ('1517293152195711036', 'Player',        'enlisted',       10)
ON CONFLICT (discord_role_id) DO UPDATE
  SET name        = EXCLUDED.name,
      mapped_role = EXCLUDED.mapped_role,
      priority    = EXCLUDED.priority;
