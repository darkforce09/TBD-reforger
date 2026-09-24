-- Seed: discord_roles — maps Discord guild role snowflakes to web permission tiers.
--
-- This is DATA, not schema, so it is NOT run by the boot migration pipeline.
-- Apply it explicitly after the DB is up and migrated:
--
--   cargo xtask db seed                       # applies this file via the compose db
--   # or: podman exec -i tbd_reforger_db psql -U tbd -d tbd_reforger < apps/website/api_v2/seeds/discord_roles.sql
--
-- How resolution works (`identity_and_access::services::discord_role_sync::resolve_role`):
--   * A user's web role is the mapped_role of their HIGHEST-priority matching row.
--   * mapped_role NULL = cosmetic (no permission grant).
--   * A user with no matching mapped row falls back to 'enlisted' (the default),
--     so the 'Player' -> 'enlisted' row below is documentation, not a behaviour change.
--
-- The role IDs below are specific to the TBD Discord guild (DISCORD_GUILD_ID in
-- .env). For a different guild, replace them with that guild's role snowflakes.
-- Idempotent: re-running updates name/mapped_role/priority in place.
--
-- Squad Leader / leader (priority 30): NOT seeded — no real guild role id is
-- committed in-repo (see documentation_v2/runbooks/local_development.md §5). After a login,
-- read snowflakes from user_discord_roles and INSERT the real mapping:
--
--   INSERT INTO discord_roles (discord_role_id, name, mapped_role, priority)
--   VALUES ('<real-squad-leader-snowflake>', 'Squad Leader', 'leader', 30)
--   ON CONFLICT (discord_role_id) DO UPDATE
--     SET name = EXCLUDED.name,
--         mapped_role = EXCLUDED.mapped_role,
--         priority = EXCLUDED.priority;
--
-- Then POST /api/v1/admin/roles/sync (or wait for the nightly resync).
--
-- The DELETE below clears the placeholder snowflake 1517290000000000000 an earlier
-- version of this seed inserted for Squad Leader/leader, so a seed refresh does not
-- leave it lingering.

DELETE FROM discord_roles WHERE discord_role_id = '1517290000000000000';

INSERT INTO discord_roles (discord_role_id, name, mapped_role, priority) VALUES
  ('1517285898817896559', 'Command Staff', 'admin',         100),
  ('1517286228851032115', 'Mission Maker', 'mission_maker',  50),
  ('1517293152195711036', 'Player',        'enlisted',       10)
ON CONFLICT (discord_role_id) DO UPDATE
  SET name        = EXCLUDED.name,
      mapped_role = EXCLUDED.mapped_role,
      priority    = EXCLUDED.priority;
