-- T-152: operator-authored reusable faction library. One row per faction; the full
-- faction document (side, name, role templates with optional SlotLoadout v2, vehicle
-- pool) lives in `doc` jsonb validated against faction-library.schema.json on write.
-- side/name are projected into columns for listing/uniqueness; owner_id = discord_id
-- (same key as missions.author_id).
CREATE TABLE IF NOT EXISTS public.user_factions (
    id uuid DEFAULT gen_random_uuid() NOT NULL PRIMARY KEY,
    owner_id text NOT NULL,
    side text NOT NULL,
    name text NOT NULL,
    doc jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_user_factions_owner_name
    ON public.user_factions USING btree (owner_id, name);
CREATE INDEX IF NOT EXISTS idx_user_factions_owner_side
    ON public.user_factions USING btree (owner_id, side);
