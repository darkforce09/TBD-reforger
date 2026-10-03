-- Validate both sides of a result move and synchronize existing attendance aggregates.
SET LOCAL lock_timeout = '30s';
LOCK TABLE public.users, public.matches, public.match_player_stats,
    public.event_missions, public.event_registrations, public.event_registration_participation
    IN SHARE ROW EXCLUSIVE MODE;
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
        SELECT array_agg(registration_id) INTO registration_ids
        FROM public.event_registration_participation WHERE match_id = NEW.id;
    ELSIF TG_TABLE_NAME = 'event_missions' THEN
        SELECT array_agg(id) INTO registration_ids FROM public.event_registrations WHERE event_mission_id = NEW.id;
    ELSE
        SELECT array_agg(registration_id) INTO registration_ids
        FROM public.event_registration_participation
        WHERE match_id = CASE WHEN TG_OP = 'DELETE' THEN OLD.match_id ELSE NEW.match_id END
           OR (TG_OP = 'UPDATE' AND match_id = OLD.match_id);
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

UPDATE public.users account SET attendance_rate = (
    SELECT COALESCE(ROUND(100.0 * count(*) FILTER (WHERE registration.attendance_state = 'attended')
        / NULLIF(count(*), 0), 2), 0)
    FROM public.event_registrations registration
    JOIN public.event_missions mission ON mission.id = registration.event_mission_id
    WHERE registration.discord_id = account.discord_id AND mission.start_time <= now()
        AND registration.attendance_state IS NOT NULL
);
INSERT INTO public.audit_logs (severity, actor_id, actor_name, action, message, target_type, target_id, metadata)
VALUES ('info', NULL, 'system', 'operations.attendance_aggregates_recomputed',
    'Derived attendance rates from decided observations; undecided signups do not imply absence',
    'migration', '0040', jsonb_build_object('migration_version', 40));
