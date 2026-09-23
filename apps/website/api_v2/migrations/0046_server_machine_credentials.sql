-- A machine credential authenticates exactly one executor of exactly one registered server.
-- Only the SHA-256 of the secret is stored; the secret itself is shown once when issued.
-- Revocation is permanent, attributed and explained; each credential is revoked independently.
CREATE TABLE public.server_machine_credentials (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id uuid NOT NULL REFERENCES public.servers(id) ON DELETE RESTRICT,
    executor_kind text NOT NULL CHECK (executor_kind IN ('host_agent', 'mod_runtime')),
    secret_sha256 text NOT NULL UNIQUE CHECK (secret_sha256 ~ '^[0-9a-f]{64}$'),
    label text NOT NULL CHECK (length(btrim(label)) > 0 AND octet_length(label) <= 128),
    created_by text NOT NULL REFERENCES public.users(discord_id),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    last_used_at timestamptz,
    revoked_at timestamptz,
    revoked_by text REFERENCES public.users(discord_id),
    revoke_reason text CHECK (revoke_reason IS NULL
        OR (length(btrim(revoke_reason)) > 0 AND octet_length(revoke_reason) <= 512)),
    CHECK ((revoked_at IS NULL) = (revoked_by IS NULL)),
    CHECK ((revoked_at IS NULL) = (revoke_reason IS NULL)),
    CHECK (revoked_at IS NULL OR revoked_at >= created_at),
    UNIQUE (id, server_id)
);
CREATE INDEX server_machine_credentials_by_server
    ON public.server_machine_credentials (server_id, created_at, id);
