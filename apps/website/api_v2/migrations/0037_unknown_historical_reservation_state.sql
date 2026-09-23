-- Historical attendance does not establish the reservation state that preceded it.
ALTER TYPE public.registration_state ADD VALUE IF NOT EXISTS 'legacy_unknown';
