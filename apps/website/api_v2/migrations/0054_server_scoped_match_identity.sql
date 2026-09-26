-- A game runtime registers each match before it reports about it. The registration binds the
-- match to the server whose machine credential registered it, so a source match identity is
-- unique per server rather than across the fleet. Results arrive as numbered revisions: the
-- stored revision and the SHA-256 of its report decide whether a later report is a retry, a
-- conflict, a stale message or a correction. Matches recorded before registration existed keep
-- server_id NULL and their global source identity.
ALTER TABLE public.matches
    ADD COLUMN server_id uuid REFERENCES public.servers(id) ON DELETE RESTRICT,
    ADD COLUMN registered_runtime_session_id uuid
        REFERENCES public.server_runtime_sessions(id) ON DELETE RESTRICT,
    ADD COLUMN registration_sha256 text CHECK (registration_sha256 ~ '^[0-9a-f]{64}$'),
    ADD COLUMN revision bigint NOT NULL DEFAULT 0 CHECK (revision >= 0),
    ADD COLUMN report_sha256 text CHECK (report_sha256 ~ '^[0-9a-f]{64}$'),
    ADD COLUMN event_count bigint NOT NULL DEFAULT 0 CHECK (event_count >= 0),
    ADD CONSTRAINT matches_registration_complete CHECK (
        (server_id IS NULL) = (registration_sha256 IS NULL)
        AND (server_id IS NULL) = (registered_runtime_session_id IS NULL)
        AND (server_id IS NULL OR source_match_id IS NOT NULL)),
    ADD CONSTRAINT matches_revision_has_report CHECK (
        server_id IS NULL OR (revision = 0) = (report_sha256 IS NULL));

DROP INDEX public.idx_matches_source_match_id;
CREATE UNIQUE INDEX matches_registered_source ON public.matches (server_id, source_match_id)
    WHERE server_id IS NOT NULL;
CREATE UNIQUE INDEX matches_unregistered_source ON public.matches (source_match_id)
    WHERE server_id IS NULL;

-- The registration is permanent and revisions only move forward.
CREATE FUNCTION public.guard_match_revision() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.server_id IS DISTINCT FROM OLD.server_id
        OR NEW.registered_runtime_session_id IS DISTINCT FROM OLD.registered_runtime_session_id
        OR NEW.registration_sha256 IS DISTINCT FROM OLD.registration_sha256
        OR (OLD.server_id IS NOT NULL AND NEW.source_match_id IS DISTINCT FROM OLD.source_match_id)
    THEN
        RAISE EXCEPTION 'a match registration never changes' USING ERRCODE = 'check_violation';
    END IF;
    IF NEW.revision < OLD.revision THEN
        RAISE EXCEPTION 'match revision % cannot follow revision %', NEW.revision, OLD.revision
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER match_revision_guard BEFORE UPDATE ON public.matches
    FOR EACH ROW EXECUTE FUNCTION public.guard_match_revision();
