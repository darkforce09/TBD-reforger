-- +goose Up
-- Phase 1: missions, identity linking, ORBAT slot assignments.

CREATE TABLE missions (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    schema_version TEXT NOT NULL DEFAULT '1.0',
    content JSONB NOT NULL,
    content_hash TEXT NOT NULL,
    author_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    published_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_missions_published_at ON missions(published_at DESC);

ALTER TABLE events ADD COLUMN mission_id TEXT;

CREATE TABLE game_identities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    identity_id TEXT NOT NULL UNIQUE,
    platform TEXT NOT NULL DEFAULT 'pc',
    linked_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE link_codes (
    code CHAR(6) PRIMARY KEY,
    identity_id TEXT NOT NULL,
    event_id UUID REFERENCES events(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ
);

CREATE INDEX idx_link_codes_expires ON link_codes(expires_at);

CREATE TABLE event_slot_assignments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    slot_id TEXT NOT NULL,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    assigned_by UUID REFERENCES users(id) ON DELETE SET NULL,
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (event_id, slot_id)
);

CREATE UNIQUE INDEX idx_event_slot_assignments_user
    ON event_slot_assignments(event_id, user_id)
    WHERE user_id IS NOT NULL;

CREATE INDEX idx_event_slot_assignments_event ON event_slot_assignments(event_id);

-- +goose Down
DROP INDEX IF EXISTS idx_event_slot_assignments_event;
DROP INDEX IF EXISTS idx_event_slot_assignments_user;
DROP TABLE IF EXISTS event_slot_assignments;
DROP TABLE IF EXISTS link_codes;
DROP TABLE IF EXISTS game_identities;
ALTER TABLE events DROP COLUMN IF EXISTS mission_id;
DROP INDEX IF EXISTS idx_missions_published_at;
DROP TABLE IF EXISTS missions;
