-- History records each reservation transition and preserves the slot before a release.
ALTER TABLE public.event_missions ADD COLUMN deleted_at timestamptz;
ALTER TABLE public.event_registrations ADD COLUMN release_reason text;
CREATE TABLE public.event_registration_history (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    registration_id uuid NOT NULL REFERENCES public.event_registrations(id) ON DELETE RESTRICT,
    reservation_state public.registration_state NOT NULL,
    slot_id uuid,
    release_reason text,
    recorded_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE INDEX registration_history_registration ON public.event_registration_history(registration_id, id);
INSERT INTO public.event_registration_history (registration_id, reservation_state, slot_id, release_reason)
    SELECT id, reservation_state, slot_id, release_reason FROM public.event_registrations;
CREATE FUNCTION public.record_registration_history() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' OR (OLD.reservation_state, OLD.slot_id, OLD.release_reason)
        IS DISTINCT FROM (NEW.reservation_state, NEW.slot_id, NEW.release_reason) THEN
        INSERT INTO public.event_registration_history (registration_id, reservation_state, slot_id, release_reason)
            VALUES (NEW.id, NEW.reservation_state, NEW.slot_id, NEW.release_reason);
    END IF;
    RETURN NULL;
END;
$$;
CREATE TRIGGER registration_history AFTER INSERT OR UPDATE ON public.event_registrations
    FOR EACH ROW EXECUTE FUNCTION public.record_registration_history();
