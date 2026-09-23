-- The fleet command ledger. An operator's intent is recorded before any executor can act on it;
-- an executor claims a command under a lease and a fencing token, reports that the effect is
-- starting, and reports its outcome. A lapsed lease returns a command to the queue unless its
-- effect may have started, in which case a non-idempotent command becomes indeterminate.
CREATE TABLE public.fleet_commands (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id uuid NOT NULL REFERENCES public.servers(id) ON DELETE RESTRICT,
    executor_kind text NOT NULL CHECK (executor_kind IN ('host_agent', 'mod_runtime')),
    action text NOT NULL
        CHECK (action IN ('start', 'stop', 'restart', 'list_players', 'broadcast', 'kick')),
    arguments jsonb NOT NULL DEFAULT '{}'::jsonb CHECK (jsonb_typeof(arguments) = 'object'),
    idempotent boolean NOT NULL,
    process_changing boolean NOT NULL,
    requested_by text NOT NULL REFERENCES public.users(discord_id),
    requested_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    expires_at timestamptz NOT NULL,
    state text NOT NULL DEFAULT 'queued' CHECK (state IN (
        'queued', 'claimed', 'executing', 'succeeded', 'failed', 'expired', 'cancelled', 'indeterminate')),
    fencing_token bigint NOT NULL DEFAULT 0 CHECK (fencing_token >= 0),
    attempts integer NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    claimed_by uuid REFERENCES public.server_machine_credentials(id) ON DELETE RESTRICT,
    claimed_at timestamptz,
    lease_expires_at timestamptz,
    executing_at timestamptz,
    finished_at timestamptz,
    outcome jsonb CHECK (outcome IS NULL OR jsonb_typeof(outcome) = 'object'),
    failure_reason text CHECK (failure_reason IS NULL OR length(btrim(failure_reason)) > 0),
    CHECK (expires_at > requested_at),
    CHECK ((state IN ('claimed', 'executing')) = (claimed_by IS NOT NULL AND lease_expires_at IS NOT NULL)),
    CHECK ((state IN ('succeeded', 'failed', 'expired', 'cancelled', 'indeterminate')) = (finished_at IS NOT NULL)),
    CHECK (state <> 'executing' OR executing_at IS NOT NULL),
    CHECK (state <> 'succeeded' OR executing_at IS NOT NULL),
    CHECK (state NOT IN ('failed', 'expired', 'cancelled', 'indeterminate') OR failure_reason IS NOT NULL)
);
CREATE INDEX fleet_commands_claimable
    ON public.fleet_commands (server_id, executor_kind, requested_at, id) WHERE state = 'queued';
CREATE INDEX fleet_commands_active_leases
    ON public.fleet_commands (lease_expires_at) WHERE state IN ('claimed', 'executing');
CREATE INDEX fleet_commands_queued_expiry
    ON public.fleet_commands (expires_at) WHERE state = 'queued';
-- At most one process-changing command per server is claimed or executing at a time.
CREATE UNIQUE INDEX fleet_commands_one_process_change_per_server
    ON public.fleet_commands (server_id) WHERE process_changing AND state IN ('claimed', 'executing');
CREATE INDEX fleet_commands_by_server
    ON public.fleet_commands (server_id, requested_at DESC, id DESC);
