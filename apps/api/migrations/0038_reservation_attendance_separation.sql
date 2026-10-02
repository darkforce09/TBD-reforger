-- Reservation and participation writers remain quiesced throughout the coordinated cutover.
SET LOCAL lock_timeout = '30s';
LOCK TABLE public.users, public.matches, public.match_player_stats,
    public.event_missions, public.event_registrations IN SHARE ROW EXCLUSIVE MODE;

ALTER TABLE public.matches ADD COLUMN finalized_at timestamptz;
-- finalized_at records when the database first observes a terminal report, not gameplay end time.
UPDATE public.matches SET finalized_at = clock_timestamp()
WHERE outcome <> 'pending';

ALTER TABLE public.event_registrations RENAME COLUMN state TO legacy_state;
ALTER TABLE public.event_registrations
    ALTER COLUMN legacy_state DROP DEFAULT,
    ALTER COLUMN legacy_state DROP NOT NULL,
    ADD COLUMN reservation_state public.registration_state NOT NULL DEFAULT 'registered',
    ADD COLUMN attendance_state public.registration_state,
    ADD COLUMN legacy_attendance_state public.registration_state,
    ADD COLUMN withdrawn_at timestamptz,
    ADD COLUMN queue_entered_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    ADD CONSTRAINT registration_reservation_state CHECK (
        reservation_state IN ('registered', 'waitlisted', 'withdrawn', 'legacy_unknown')),
    ADD CONSTRAINT registration_attendance_state CHECK (attendance_state IN ('attended', 'no_show')),
    ADD CONSTRAINT registration_legacy_attendance_state CHECK (legacy_attendance_state IN ('attended', 'no_show'));
UPDATE public.event_registrations SET
    reservation_state = CASE WHEN legacy_state IN ('registered', 'waitlisted', 'withdrawn')
        THEN legacy_state ELSE 'legacy_unknown'::public.registration_state END,
    attendance_state = CASE WHEN legacy_state IN ('attended', 'no_show') THEN legacy_state END,
    legacy_attendance_state = CASE WHEN legacy_state IN ('attended', 'no_show') THEN legacy_state END,
    queue_entered_at = registered_at;
ALTER TABLE public.event_registrations ADD COLUMN state public.registration_state
    GENERATED ALWAYS AS (COALESCE(attendance_state, reservation_state)) STORED;

CREATE TABLE public.event_registration_participation (
    registration_id uuid NOT NULL REFERENCES public.event_registrations(id) ON DELETE RESTRICT,
    match_id uuid NOT NULL REFERENCES public.matches(id) ON DELETE RESTRICT,
    arma_id text NOT NULL CHECK (length(btrim(arma_id)) > 0),
    observed_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (registration_id, match_id, arma_id)
);
CREATE INDEX event_registration_participation_match ON public.event_registration_participation(match_id);
INSERT INTO public.event_registration_participation (registration_id, match_id, arma_id)
SELECT DISTINCT registration.id, result.match_id, result.arma_id
FROM public.event_registrations registration
JOIN public.event_missions mission ON mission.id = registration.event_mission_id
JOIN public.matches match_row ON match_row.event_id = mission.event_id AND match_row.mission_id = mission.mission_id
JOIN public.match_player_stats result ON result.match_id = match_row.id AND result.discord_id = registration.discord_id
WHERE registration.legacy_state = 'attended' AND match_row.finalized_at IS NOT NULL;
UPDATE public.event_registrations registration SET legacy_attendance_state = NULL
WHERE EXISTS (SELECT 1 FROM public.event_registration_participation participation
    WHERE participation.registration_id = registration.id);

INSERT INTO public.audit_logs (severity, actor_id, actor_name, action, message, target_type, target_id, metadata, created_at)
VALUES ('info', NULL, 'system', 'operations.reservation_attendance_separated',
    'Preserved historical observations and separated reservation status from attendance with match provenance',
    'migration', '0038', jsonb_build_object('migration_version', 38), clock_timestamp());

ALTER TABLE public.matches ADD CONSTRAINT match_finalization_state
    CHECK ((outcome = 'pending') = (finalized_at IS NULL));
CREATE FUNCTION public.enforce_match_finalization() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND OLD.finalized_at IS NOT NULL THEN
        IF NEW.outcome = 'pending' OR NEW.finalized_at IS DISTINCT FROM OLD.finalized_at THEN
            RAISE EXCEPTION 'match finalization cannot regress or change its observation time' USING ERRCODE = '23514';
        END IF;
    END IF;
    IF NEW.outcome <> 'pending' AND NEW.finalized_at IS NULL THEN
        NEW.finalized_at := clock_timestamp();
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER match_finalization BEFORE INSERT OR UPDATE ON public.matches
    FOR EACH ROW EXECUTE FUNCTION public.enforce_match_finalization();

-- Deferred checks inspect the completed transaction, allowing a correction to replace provenance.
CREATE FUNCTION public.validate_registration_participation() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE registration_ids uuid[];
BEGIN
    IF TG_TABLE_NAME = 'event_registrations' THEN
        registration_ids := ARRAY[NEW.id];
    ELSIF TG_TABLE_NAME = 'event_registration_participation' THEN
        registration_ids := CASE WHEN TG_OP = 'DELETE' THEN ARRAY[OLD.registration_id]
            WHEN TG_OP = 'UPDATE' THEN ARRAY[OLD.registration_id, NEW.registration_id]
            ELSE ARRAY[NEW.registration_id] END;
    ELSIF TG_TABLE_NAME = 'matches' THEN
        SELECT array_agg(registration_id) INTO registration_ids
        FROM public.event_registration_participation WHERE match_id = NEW.id;
    ELSIF TG_TABLE_NAME = 'event_missions' THEN
        SELECT array_agg(id) INTO registration_ids FROM public.event_registrations WHERE event_mission_id = NEW.id;
    ELSE
        SELECT array_agg(registration_id) INTO registration_ids
        FROM public.event_registration_participation
        WHERE match_id = CASE WHEN TG_OP = 'DELETE' THEN OLD.match_id ELSE NEW.match_id END;
    END IF;
    IF EXISTS (
        SELECT 1 FROM public.event_registration_participation participation
        JOIN public.event_registrations registration ON registration.id = participation.registration_id
        JOIN public.event_missions mission ON mission.id = registration.event_mission_id
        JOIN public.matches match_row ON match_row.id = participation.match_id
        WHERE participation.registration_id = ANY(registration_ids) AND (
            match_row.finalized_at IS NULL OR match_row.event_id IS DISTINCT FROM mission.event_id
            OR match_row.mission_id IS DISTINCT FROM mission.mission_id OR NOT EXISTS (
                SELECT 1 FROM public.match_player_stats result WHERE result.match_id = match_row.id
                AND result.arma_id = participation.arma_id))) THEN
        RAISE EXCEPTION 'participation requires finalized facts for the exact event and mission' USING ERRCODE = '23514';
    END IF;
    IF EXISTS (
        SELECT 1 FROM public.event_registrations registration
        WHERE registration.id = ANY(registration_ids) AND registration.attendance_state IS DISTINCT FROM
            CASE WHEN EXISTS (SELECT 1 FROM public.event_registration_participation participation
                WHERE participation.registration_id = registration.id)
            THEN 'attended'::public.registration_state ELSE registration.legacy_attendance_state END) THEN
        RAISE EXCEPTION 'attendance must agree with accepted participation or preserved legacy evidence' USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;
CREATE CONSTRAINT TRIGGER registration_participation_check
    AFTER INSERT OR UPDATE ON public.event_registrations DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION public.validate_registration_participation();
CREATE CONSTRAINT TRIGGER participation_registration_check
    AFTER INSERT OR UPDATE OR DELETE ON public.event_registration_participation DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION public.validate_registration_participation();
CREATE CONSTRAINT TRIGGER match_participation_check
    AFTER UPDATE ON public.matches DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION public.validate_registration_participation();
CREATE CONSTRAINT TRIGGER result_participation_check
    AFTER UPDATE OR DELETE ON public.match_player_stats DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION public.validate_registration_participation();
CREATE CONSTRAINT TRIGGER mission_participation_check
    AFTER UPDATE ON public.event_missions DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION public.validate_registration_participation();
