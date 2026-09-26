-- Detailed combat, medical and vehicle events of a registered match. The game runtime assigns
-- each event a stable id and a sequence in capture order; both are unique within the match and
-- the sequence is the order of every read. An event is a fact: it is never updated.
-- match_event_totals counts stored events per identity, kind and participant role; the ingest
-- transaction increments it only for rows it actually inserted, so a retry never counts twice.
CREATE TABLE public.match_events (
    match_id uuid NOT NULL REFERENCES public.matches(id) ON DELETE CASCADE,
    event_id text NOT NULL CHECK (event_id ~ '^[A-Za-z0-9._:-]{1,64}$'),
    sequence bigint NOT NULL CHECK (sequence >= 1),
    kind text NOT NULL CHECK (kind IN ('combat.kill', 'combat.death', 'medical.incapacitated',
        'medical.revived', 'vehicle.destroyed', 'vehicle.entered', 'vehicle.exited')),
    mission_time_ms bigint NOT NULL CHECK (mission_time_ms >= 0),
    occurred_at timestamptz NOT NULL,
    actor_arma_id text CHECK (octet_length(actor_arma_id) BETWEEN 1 AND 128),
    subject_arma_id text CHECK (octet_length(subject_arma_id) BETWEEN 1 AND 128),
    payload jsonb NOT NULL CHECK (jsonb_typeof(payload) = 'object'),
    payload_sha256 text NOT NULL CHECK (payload_sha256 ~ '^[0-9a-f]{64}$'),
    recorded_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (match_id, event_id),
    UNIQUE (match_id, sequence)
);

CREATE FUNCTION public.refuse_match_event_update() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'a match event is never updated' USING ERRCODE = 'check_violation';
END;
$$;
CREATE TRIGGER match_events_are_immutable BEFORE UPDATE ON public.match_events
    FOR EACH ROW EXECUTE FUNCTION public.refuse_match_event_update();

CREATE TABLE public.match_event_totals (
    match_id uuid NOT NULL REFERENCES public.matches(id) ON DELETE CASCADE,
    arma_id text NOT NULL CHECK (octet_length(arma_id) BETWEEN 1 AND 128),
    kind text NOT NULL CHECK (kind IN ('combat.kill', 'combat.death', 'medical.incapacitated',
        'medical.revived', 'vehicle.destroyed', 'vehicle.entered', 'vehicle.exited')),
    participant_role text NOT NULL CHECK (participant_role IN ('actor', 'subject')),
    event_count bigint NOT NULL CHECK (event_count >= 1),
    PRIMARY KEY (match_id, arma_id, kind, participant_role)
);
CREATE INDEX match_event_totals_arma_id ON public.match_event_totals (arma_id);
