-- A runtime session is one boot of a server's game runtime, fenced by a per-server generation.
-- Starting a session ends the server's previous one; heartbeats carry the generation and a
-- strictly increasing sequence, so a stale runtime or a delayed message can never overwrite the
-- live state of a newer one.
CREATE TABLE public.server_runtime_sessions (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id uuid NOT NULL REFERENCES public.servers(id) ON DELETE RESTRICT,
    credential_id uuid NOT NULL,
    generation bigint NOT NULL CHECK (generation > 0),
    started_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    last_heartbeat_at timestamptz,
    last_sequence bigint NOT NULL DEFAULT 0 CHECK (last_sequence >= 0),
    ended_at timestamptz,
    end_reason text CHECK (end_reason IS NULL
        OR end_reason IN ('superseded', 'expired', 'ended_by_runtime', 'credential_revoked')),
    CHECK ((ended_at IS NULL) = (end_reason IS NULL)),
    CHECK (ended_at IS NULL OR ended_at >= started_at),
    UNIQUE (server_id, generation),
    -- The authenticating credential belongs to the same server as the session.
    FOREIGN KEY (credential_id, server_id)
        REFERENCES public.server_machine_credentials (id, server_id) ON DELETE RESTRICT
);
CREATE UNIQUE INDEX server_runtime_sessions_one_open
    ON public.server_runtime_sessions (server_id) WHERE ended_at IS NULL;
CREATE INDEX server_runtime_sessions_open_by_credential
    ON public.server_runtime_sessions (credential_id) WHERE ended_at IS NULL;
