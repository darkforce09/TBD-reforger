-- Verification timestamps distinguish authoritative empty membership from unavailable data.
CREATE TABLE public.discord_membership_snapshots (
    discord_id text NOT NULL REFERENCES public.users(discord_id) ON DELETE CASCADE,
    guild_id text NOT NULL CHECK (guild_id <> ''),
    membership_status text NOT NULL DEFAULT 'unknown'
        CHECK (membership_status IN ('unknown', 'member', 'nonmember')),
    verified_at timestamptz,
    revision bigint NOT NULL DEFAULT 0 CHECK (revision >= 0),
    next_refresh_at timestamptz NOT NULL DEFAULT now(),
    lease_expires_at timestamptz,
    lease_token uuid,
    last_error text,
    PRIMARY KEY (discord_id, guild_id),
    CHECK (membership_status = 'unknown' OR verified_at IS NOT NULL),
    CHECK ((lease_token IS NULL) = (lease_expires_at IS NULL))
);
CREATE INDEX discord_membership_refresh_due
    ON public.discord_membership_snapshots (next_refresh_at);

ALTER TABLE public.user_discord_roles ADD COLUMN guild_id text NOT NULL DEFAULT '';
CREATE INDEX user_discord_roles_guild ON public.user_discord_roles (discord_id, guild_id);

-- Overrides extend an existing verified snapshot; they do not invent Discord roles.
CREATE TABLE public.discord_membership_grace_overrides (
    discord_id text NOT NULL,
    guild_id text NOT NULL,
    authorized_by text NOT NULL REFERENCES public.users(discord_id),
    reason text NOT NULL CHECK (length(btrim(reason)) > 0),
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL,
    PRIMARY KEY (discord_id, guild_id),
    FOREIGN KEY (discord_id, guild_id)
        REFERENCES public.discord_membership_snapshots(discord_id, guild_id) ON DELETE CASCADE,
    CHECK (expires_at > created_at AND expires_at <= created_at + interval '48 hours')
);
