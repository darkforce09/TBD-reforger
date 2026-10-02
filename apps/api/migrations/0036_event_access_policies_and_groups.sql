-- Explicit event policy is required; NULL child policies inherit and empty grants deny.
ALTER TABLE public.events
    ADD COLUMN access_policy jsonb NOT NULL DEFAULT '{"grants":[{"conditions":[{"kind":"tbd_member"}]}]}'::jsonb,
    ADD COLUMN access_revision bigint NOT NULL DEFAULT 0 CHECK (access_revision >= 0),
    ADD CONSTRAINT events_access_policy_object CHECK (jsonb_typeof(access_policy) = 'object');

ALTER TABLE public.orbat_slots
    ADD COLUMN access_policy jsonb,
    ADD CONSTRAINT orbat_slots_access_policy_object
        CHECK (access_policy IS NULL OR jsonb_typeof(access_policy) = 'object');

CREATE TABLE public.event_squad_access_policies (
    event_mission_id uuid NOT NULL REFERENCES public.event_missions(id) ON DELETE CASCADE,
    faction text NOT NULL CHECK (length(btrim(faction)) > 0 AND octet_length(faction) <= 128),
    squad text NOT NULL CHECK (length(btrim(squad)) > 0 AND octet_length(squad) <= 128),
    access_policy jsonb NOT NULL CHECK (jsonb_typeof(access_policy) = 'object'),
    PRIMARY KEY (event_mission_id, faction, squad)
);

CREATE TABLE public.event_groups (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id uuid NOT NULL REFERENCES public.events(id) ON DELETE CASCADE,
    name text NOT NULL CHECK (length(btrim(name)) > 0 AND octet_length(name) <= 128),
    source jsonb NOT NULL CHECK (jsonb_typeof(source) = 'object'),
    created_by text NOT NULL REFERENCES public.users(discord_id),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    deleted_at timestamptz
);
CREATE INDEX event_groups_by_event ON public.event_groups(event_id, id) WHERE deleted_at IS NULL;

-- Roster provenance is manager-authored. Partner-guild groups ignore these rows entirely.
CREATE TABLE public.event_group_roster (
    group_id uuid NOT NULL REFERENCES public.event_groups(id) ON DELETE CASCADE,
    discord_id text NOT NULL REFERENCES public.users(discord_id),
    added_by text NOT NULL REFERENCES public.users(discord_id),
    added_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    removed_at timestamptz,
    PRIMARY KEY (group_id, discord_id)
);
