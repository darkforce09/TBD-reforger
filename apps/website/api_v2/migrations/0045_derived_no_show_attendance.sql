-- Attendance derives from finalized facts only. Participation in a finalized exact-match result, or
-- a preserved legacy attended observation, means attended. Otherwise a reservation that was active
-- when a non-aborted match for its exact event mission was first observed as finalized means
-- no_show. Otherwise the preserved legacy observation applies. Scheduled time alone decides nothing.
-- Match, attendance and reservation writers remain quiesced throughout the coordinated cutover.
SET LOCAL lock_timeout = '30s';
LOCK TABLE public.users, public.matches, public.match_player_stats, public.event_missions,
    public.event_registrations, public.event_registration_participation IN SHARE ROW EXCLUSIVE MODE;

CREATE INDEX matches_finalized_event_mission ON public.matches (event_id, mission_id)
    WHERE finalized_at IS NOT NULL;

-- The reservation state recorded by the append-only history at an instant; NULL before the signup.
CREATE FUNCTION public.reservation_state_at(target_registration uuid, observed_at timestamptz)
RETURNS public.registration_state LANGUAGE sql STABLE AS $$
    SELECT history.reservation_state FROM public.event_registration_history history
    WHERE history.registration_id = target_registration AND history.recorded_at <= observed_at
    ORDER BY history.recorded_at DESC, history.id DESC
    LIMIT 1
$$;

CREATE FUNCTION public.derived_attendance_state(target_registration uuid)
RETURNS public.registration_state LANGUAGE sql STABLE AS $$
    SELECT CASE
        WHEN EXISTS (SELECT 1 FROM public.event_registration_participation participation
            WHERE participation.registration_id = registration.id)
            OR registration.legacy_attendance_state = 'attended'
        THEN 'attended'::public.registration_state
        WHEN EXISTS (
            SELECT 1 FROM public.event_missions mission
            JOIN public.matches match_row ON match_row.event_id = mission.event_id
                AND match_row.mission_id = mission.mission_id
            WHERE mission.id = registration.event_mission_id
                AND match_row.finalized_at IS NOT NULL AND match_row.outcome <> 'aborted'
                AND public.reservation_state_at(registration.id, match_row.finalized_at)
                    IN ('registered', 'legacy_unknown'))
        THEN 'no_show'::public.registration_state
        ELSE registration.legacy_attendance_state END
    FROM public.event_registrations registration
    WHERE registration.id = target_registration
$$;

-- A match change affects both its participants and every registration of its old and new
-- exact event mission, whose attendance obligations follow the match association.
CREATE OR REPLACE FUNCTION public.validate_registration_participation() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE registration_ids uuid[];
BEGIN
    IF TG_TABLE_NAME = 'event_registrations' THEN
        registration_ids := ARRAY[NEW.id];
    ELSIF TG_TABLE_NAME = 'event_registration_participation' THEN
        registration_ids := CASE WHEN TG_OP = 'DELETE' THEN ARRAY[OLD.registration_id]
            WHEN TG_OP = 'UPDATE' THEN ARRAY[OLD.registration_id, NEW.registration_id]
            ELSE ARRAY[NEW.registration_id] END;
    ELSIF TG_TABLE_NAME = 'matches' THEN
        SELECT array_agg(DISTINCT participation.registration_id) INTO registration_ids
        FROM public.event_registration_participation participation WHERE participation.match_id = NEW.id;
        registration_ids := COALESCE(registration_ids, ARRAY[]::uuid[]) || ARRAY(
            SELECT registration.id FROM public.event_registrations registration
            JOIN public.event_missions mission ON mission.id = registration.event_mission_id
            WHERE mission.event_id = NEW.event_id AND mission.mission_id = NEW.mission_id);
        IF TG_OP = 'UPDATE' THEN
            registration_ids := registration_ids || ARRAY(
                SELECT registration.id FROM public.event_registrations registration
                JOIN public.event_missions mission ON mission.id = registration.event_mission_id
                WHERE mission.event_id = OLD.event_id AND mission.mission_id = OLD.mission_id);
        END IF;
    ELSIF TG_TABLE_NAME = 'event_missions' THEN
        SELECT array_agg(id) INTO registration_ids FROM public.event_registrations WHERE event_mission_id = NEW.id;
    ELSIF TG_OP = 'UPDATE' THEN
        SELECT array_agg(registration_id) INTO registration_ids
        FROM public.event_registration_participation
        WHERE match_id = NEW.match_id OR match_id = OLD.match_id;
    ELSE
        SELECT array_agg(registration_id) INTO registration_ids
        FROM public.event_registration_participation WHERE match_id = OLD.match_id;
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
        WHERE registration.id = ANY(registration_ids)
            AND registration.attendance_state IS DISTINCT FROM public.derived_attendance_state(registration.id)) THEN
        RAISE EXCEPTION 'attendance must agree with finalized participation, reservation obligations or preserved legacy evidence'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;
CREATE CONSTRAINT TRIGGER match_insert_participation_check
    AFTER INSERT ON public.matches DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION public.validate_registration_participation();

CREATE TEMPORARY TABLE derived_attendance_changes ON COMMIT DROP AS
SELECT registration.id, registration.discord_id FROM public.event_registrations registration
WHERE registration.attendance_state IS DISTINCT FROM public.derived_attendance_state(registration.id);
UPDATE public.event_registrations registration
SET attendance_state = public.derived_attendance_state(registration.id)
WHERE registration.id IN (SELECT id FROM derived_attendance_changes);
UPDATE public.users account SET attendance_rate = (
    SELECT COALESCE(ROUND(100.0 * count(*) FILTER (WHERE registration.attendance_state = 'attended')
        / NULLIF(count(*), 0), 2), 0)
    FROM public.event_registrations registration
    JOIN public.event_missions mission ON mission.id = registration.event_mission_id
    WHERE registration.discord_id = account.discord_id AND mission.start_time <= statement_timestamp()
        AND registration.attendance_state IS NOT NULL)
WHERE account.discord_id IN (SELECT discord_id FROM derived_attendance_changes);

INSERT INTO public.audit_logs (severity, actor_id, actor_name, action, message, target_type, target_id, metadata, created_at)
SELECT 'info', NULL, 'system', 'operations.no_show_derived_from_finalized_facts',
    'Derived no-show from reservations active at match finalization; scheduled time alone decides nothing',
    'migration', '0045', jsonb_build_object('migration_version', 45,
        'registrations', (SELECT count(*) FROM derived_attendance_changes),
        'accounts', (SELECT count(DISTINCT discord_id) FROM derived_attendance_changes)),
    clock_timestamp();
