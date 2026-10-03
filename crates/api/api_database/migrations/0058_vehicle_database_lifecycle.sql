-- The lifecycle of a vehicle database entry: when it was created and last changed and by whom,
-- and its soft deletion. A deleted entry keeps its row with `deleted_at` and `deleted_by` set and
-- disappears from every read. The stamps name members by Discord id and, like every other actor
-- stamp, carry no foreign key, so an entry outlives the member named in it.
--
-- Entries that predate these columns keep them null: nothing recorded when or by whom they were
-- written. The columns are added without defaults so no existing row receives a fabricated stamp,
-- and only then do `created_at` and `updated_at` default to the insert time of new rows.
ALTER TABLE public.vehicle_databases
    ADD COLUMN created_at timestamptz,
    ADD COLUMN updated_at timestamptz,
    ADD COLUMN created_by text,
    ADD COLUMN updated_by text,
    ADD COLUMN deleted_at timestamptz,
    ADD COLUMN deleted_by text;

ALTER TABLE public.vehicle_databases
    ALTER COLUMN created_at SET DEFAULT now(),
    ALTER COLUMN updated_at SET DEFAULT now();
