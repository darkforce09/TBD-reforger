-- 03_indexes.sql
-- Runs AFTER GORM AutoMigrate. Covers index types AutoMigrate cannot express
-- from struct tags: JSONB GIN indexes and partial indexes. All idempotent.

-- GIN index for querying marker/vehicle contents inside the 2D-editor payload.
CREATE INDEX IF NOT EXISTS idx_mission_payload_gin
    ON mission_versions USING GIN (json_payload);

-- Partial index: active users by role (roster / authz lookups skip soft-deleted).
CREATE INDEX IF NOT EXISTS idx_users_role_active
    ON users (role) WHERE deleted_at IS NULL;

-- Partial index: the published announcement feed, newest first.
CREATE INDEX IF NOT EXISTS idx_ann_published
    ON announcements (published_at DESC)
    WHERE status = 'published' AND deleted_at IS NULL;

-- Partial index: open (unconsumed) identity link codes per user.
CREATE INDEX IF NOT EXISTS idx_link_codes_open
    ON identity_link_codes (discord_id) WHERE consumed_at IS NULL;
