-- A deployment runs one approved mission artifact on one server. The fleet registers the
-- scenario it runs for each terrain; a deployment records its validated selection, the slot
-- bindings of its event mission's ORBAT to the artifact's compiled slots, the fleet command
-- that performs its transition, and the runtime session that confirmed the artifact it loaded.
CREATE TABLE public.fleet_scenarios (
    terrain_key text PRIMARY KEY CHECK (terrain_key ~ '^[a-z][a-z0-9_]{0,63}$'),
    scenario_id text NOT NULL CHECK (scenario_id ~ '^\{[0-9A-F]{16}\}[A-Za-z0-9_./-]+\.conf$'),
    display_name text NOT NULL CHECK (length(btrim(display_name)) BETWEEN 1 AND 128),
    updated_by text NOT NULL REFERENCES public.users(discord_id),
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp()
);

ALTER TABLE public.fleet_commands DROP CONSTRAINT fleet_commands_action_check;
ALTER TABLE public.fleet_commands ADD CONSTRAINT fleet_commands_action_check CHECK (action IN (
    'start', 'stop', 'restart', 'list_players', 'broadcast', 'kick', 'load_mission',
    'restart_with_mission'));

-- What a runtime reported loading when it started its session.
ALTER TABLE public.server_runtime_sessions
    ADD COLUMN loaded_artifact_id uuid REFERENCES public.mission_artifacts(id) ON DELETE RESTRICT,
    ADD COLUMN loaded_artifact_sha256 text CHECK (loaded_artifact_sha256 ~ '^[0-9a-f]{64}$'),
    ADD CONSTRAINT server_runtime_sessions_loaded_artifact_complete
        CHECK ((loaded_artifact_id IS NULL) = (loaded_artifact_sha256 IS NULL));

CREATE TABLE public.mission_deployments (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id uuid NOT NULL REFERENCES public.servers(id) ON DELETE RESTRICT,
    mission_id uuid NOT NULL REFERENCES public.missions(id) ON DELETE RESTRICT,
    artifact_id uuid NOT NULL REFERENCES public.mission_artifacts(id) ON DELETE RESTRICT,
    event_mission_id uuid REFERENCES public.event_missions(id) ON DELETE RESTRICT,
    terrain_key text NOT NULL,
    scenario_id text NOT NULL,
    transition text NOT NULL CHECK (transition IN ('scenario_restart', 'host_restart')),
    fleet_command_id uuid NOT NULL UNIQUE REFERENCES public.fleet_commands(id) ON DELETE RESTRICT,
    requested_by text NOT NULL REFERENCES public.users(discord_id),
    requested_via text NOT NULL CHECK (requested_via IN ('web', 'game_runtime')),
    requested_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    deadline_at timestamptz NOT NULL,
    state text NOT NULL DEFAULT 'requested'
        CHECK (state IN ('requested', 'confirmed', 'failed', 'cancelled')),
    confirmed_runtime_session_id uuid REFERENCES public.server_runtime_sessions(id) ON DELETE RESTRICT,
    finished_at timestamptz,
    failure_reason text CHECK (failure_reason IS NULL OR length(btrim(failure_reason)) > 0),
    CHECK (deadline_at > requested_at),
    CHECK ((state = 'requested') = (finished_at IS NULL)),
    CHECK ((state = 'confirmed') = (confirmed_runtime_session_id IS NOT NULL)),
    CHECK ((state IN ('failed', 'cancelled')) = (failure_reason IS NOT NULL))
);
CREATE UNIQUE INDEX mission_deployments_one_in_flight_per_server
    ON public.mission_deployments (server_id) WHERE state = 'requested';
CREATE INDEX mission_deployments_by_server
    ON public.mission_deployments (server_id, requested_at DESC, id DESC);
CREATE INDEX mission_deployments_in_flight_deadline
    ON public.mission_deployments (deadline_at) WHERE state = 'requested';

CREATE TABLE public.mission_deployment_slots (
    deployment_id uuid NOT NULL REFERENCES public.mission_deployments(id) ON DELETE RESTRICT,
    orbat_slot_id uuid NOT NULL REFERENCES public.orbat_slots(id) ON DELETE RESTRICT,
    slot_uid text NOT NULL CHECK (length(slot_uid) > 0),
    PRIMARY KEY (deployment_id, orbat_slot_id),
    UNIQUE (deployment_id, slot_uid)
);
CREATE INDEX mission_deployment_slots_by_seat ON public.mission_deployment_slots (orbat_slot_id);
