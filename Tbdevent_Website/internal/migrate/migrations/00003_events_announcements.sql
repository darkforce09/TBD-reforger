-- +goose Up
CREATE TABLE events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    slug TEXT UNIQUE NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    map_name TEXT NOT NULL DEFAULT '',
    starts_at TIMESTAMPTZ NOT NULL,
    ends_at TIMESTAMPTZ,
    status TEXT NOT NULL DEFAULT 'draft',
    max_players INT,
    signups_open BOOLEAN NOT NULL DEFAULT FALSE,
    published BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

CREATE TABLE announcements (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    pinned BOOLEAN NOT NULL DEFAULT FALSE,
    published BOOLEAN NOT NULL DEFAULT FALSE,
    published_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

CREATE TABLE event_registrations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'registered',
    signed_up_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    cancelled_at TIMESTAMPTZ,
    UNIQUE (event_id, user_id)
);

CREATE INDEX idx_events_starts_at ON events(starts_at);
CREATE INDEX idx_events_published ON events(published);
CREATE INDEX idx_announcements_published ON announcements(published);
CREATE INDEX idx_event_registrations_event_id ON event_registrations(event_id);
CREATE INDEX idx_event_registrations_user_id ON event_registrations(user_id);

INSERT INTO events (id, title, slug, description, map_name, starts_at, ends_at, status, max_players, signups_open, published) VALUES
    ('b0000000-0000-4000-8000-000000000001', 'TBD PvP Event #1', 'tbd-pvp-1',
     E'Competitive PvP event on Everon. Sign up via Discord to reserve your slot.',
     'Everon', NOW() + INTERVAL '14 days', NOW() + INTERVAL '14 days' + INTERVAL '4 hours',
     'published', 40, TRUE, TRUE);

INSERT INTO announcements (title, body, pinned, published, published_at) VALUES
    ('Sign-ups are open!', E'Registration is now open for the next TBD PvP event. Head to **Events** to sign up.', TRUE, TRUE, NOW()),
    ('Event rules updated', E'Please review the updated rules before the next event.', FALSE, TRUE, NOW() - INTERVAL '2 days'),
    ('Welcome to the new event hub', E'This website is now the central place for all TBD PvP event information and sign-ups.', FALSE, TRUE, NOW() - INTERVAL '5 days');

-- +goose Down
DROP TABLE IF EXISTS event_registrations;
DROP TABLE IF EXISTS announcements;
DROP TABLE IF EXISTS events;
