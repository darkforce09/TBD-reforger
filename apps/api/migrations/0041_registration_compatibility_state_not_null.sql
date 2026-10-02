-- At least reservation_state is present, so its compatibility projection is never null.
ALTER TABLE public.event_registrations ALTER COLUMN state SET NOT NULL;
