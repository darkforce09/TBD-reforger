-- A live occupancy is one player life in one ORBAT slot during one runtime session. It is
-- independent of reservations: releasing a reservation never ends a life. A slot holds at most
-- one open life, so no replacement spawns into an occupied slot, and a player holds at most one
-- open life per session. A retried deployment of the same life finds its original decision.
CREATE TABLE public.live_slot_occupancies (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    runtime_session_id uuid NOT NULL REFERENCES public.server_runtime_sessions(id) ON DELETE RESTRICT,
    event_mission_id uuid NOT NULL REFERENCES public.event_missions(id) ON DELETE RESTRICT,
    orbat_slot_id uuid NOT NULL REFERENCES public.orbat_slots(id) ON DELETE RESTRICT,
    arma_id text NOT NULL CHECK (length(btrim(arma_id)) > 0 AND octet_length(arma_id) <= 128),
    discord_id text NOT NULL REFERENCES public.users(discord_id),
    player_life_id text NOT NULL
        CHECK (length(btrim(player_life_id)) > 0 AND octet_length(player_life_id) <= 128),
    authorized_by text NOT NULL CHECK (authorized_by IN ('reservation', 'open_slot_policy')),
    started_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    ended_at timestamptz,
    end_reason text CHECK (end_reason IS NULL OR end_reason IN (
        'life_ended', 'session_superseded', 'session_expired', 'session_ended', 'credential_revoked')),
    CHECK ((ended_at IS NULL) = (end_reason IS NULL)),
    CHECK (ended_at IS NULL OR ended_at >= started_at),
    UNIQUE (runtime_session_id, player_life_id)
);
CREATE UNIQUE INDEX live_slot_occupancies_one_open_life_per_slot
    ON public.live_slot_occupancies (orbat_slot_id) WHERE ended_at IS NULL;
CREATE UNIQUE INDEX live_slot_occupancies_one_open_life_per_player
    ON public.live_slot_occupancies (runtime_session_id, arma_id) WHERE ended_at IS NULL;
CREATE INDEX live_slot_occupancies_open_by_session
    ON public.live_slot_occupancies (runtime_session_id) WHERE ended_at IS NULL;
CREATE INDEX live_slot_occupancies_by_event_mission
    ON public.live_slot_occupancies (event_mission_id, started_at);
