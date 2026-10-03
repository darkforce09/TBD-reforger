-- Game ballistics: the immutable catalog versions the fire-mission solver runs on, the inputs a
-- catalog-model fire mission is solved from, its per-gun solutions, and a foreign key on
-- `fire_missions.event_id`.
--
-- A catalog version is the weapon and shell data exported from one game build together with the
-- calibration bundle that proved the flight model against the game. A stored fire mission pins
-- `(catalog_id, catalog_version)`, so the version must mean the same bytes for as long as any row
-- names it: the trigger below refuses every update and delete, and the same catalog bytes cannot
-- be stored twice under one catalog id.
CREATE TABLE public.ballistics_catalogs (
    catalog_id text NOT NULL
        CHECK (catalog_id ~ '^[a-z0-9]+([_-][a-z0-9]+)*$' AND length(catalog_id) <= 64),
    catalog_version integer NOT NULL CHECK (catalog_version >= 1),
    title text NOT NULL CHECK (length(title) > 0),
    game_build text NOT NULL CHECK (game_build ~ '^[0-9]+(\.[0-9]+){1,3}$'),
    export_generation_id text NOT NULL CHECK (export_generation_id ~ '^[0-9A-F]{16}$'),
    catalog_sha256 text NOT NULL CHECK (catalog_sha256 ~ '^[0-9a-f]{64}$'),
    calibration_sha256 text NOT NULL CHECK (calibration_sha256 ~ '^[0-9a-f]{64}$'),
    catalog_document jsonb NOT NULL,
    calibration_document jsonb NOT NULL,
    validation_report jsonb NOT NULL,
    uploaded_by text NOT NULL,
    uploaded_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (catalog_id, catalog_version),
    UNIQUE (catalog_id, catalog_sha256)
);

CREATE FUNCTION public.refuse_ballistics_catalog_change() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'ballistics catalogs are immutable' USING ERRCODE = '23514';
END;
$$;
CREATE TRIGGER ballistics_catalogs_are_immutable
    BEFORE UPDATE OR DELETE ON public.ballistics_catalogs
    FOR EACH ROW EXECUTE FUNCTION public.refuse_ballistics_catalog_change();

-- The inputs of a catalog-model fire mission. Every column is nullable with no default: a row
-- stored before catalogs has none of them, and a default would claim a value the row never
-- recorded. `fire_missions_catalog_model_all_or_none` keeps a row either wholly legacy or
-- carrying every required input (catalog, weapon, shell, target height and its source, the
-- weapon's mils per circle, the solver revision); the optional inputs (the operator's charge,
-- the wind, the burst height, the fuze time, the lead gun's dispersion) exist only on a
-- catalog-model row. The wind is a speed and a direction or neither.
ALTER TABLE public.fire_missions
    ADD COLUMN catalog_id text,
    ADD COLUMN catalog_version integer,
    ADD COLUMN weapon_id text,
    ADD COLUMN shell_id text,
    ADD COLUMN charge_rings smallint CHECK (charge_rings >= 0),
    ADD COLUMN target_height_m double precision,
    ADD COLUMN target_height_source text CHECK (target_height_source IN ('dem', 'manual')),
    ADD COLUMN wind_speed_m_s double precision CHECK (wind_speed_m_s >= 0),
    ADD COLUMN wind_from_deg double precision CHECK (wind_from_deg >= 0 AND wind_from_deg < 360),
    ADD COLUMN burst_height_m double precision,
    ADD COLUMN fuze_time_s double precision CHECK (fuze_time_s >= 0),
    ADD COLUMN mils_per_circle integer CHECK (mils_per_circle >= 1),
    ADD COLUMN dispersion jsonb CHECK (jsonb_typeof(dispersion) = 'object'),
    ADD COLUMN solver_revision text,
    ADD COLUMN detached_event_id uuid,
    ADD CONSTRAINT fire_missions_catalog_fkey FOREIGN KEY (catalog_id, catalog_version)
        REFERENCES public.ballistics_catalogs (catalog_id, catalog_version),
    ADD CONSTRAINT fire_missions_catalog_model_all_or_none CHECK (
        num_nulls(catalog_id, catalog_version, weapon_id, shell_id, target_height_m,
                  target_height_source, mils_per_circle, solver_revision) = 0
        OR num_nonnulls(catalog_id, catalog_version, weapon_id, shell_id, charge_rings,
                        target_height_m, target_height_source, wind_speed_m_s, wind_from_deg,
                        burst_height_m, fuze_time_s, mils_per_circle, dispersion,
                        solver_revision) = 0
    ),
    ADD CONSTRAINT fire_missions_wind_all_or_none
        CHECK (num_nulls(wind_speed_m_s, wind_from_deg) IN (0, 2)),
    ADD CONSTRAINT fire_missions_attached_or_detached
        CHECK (event_id IS NULL OR detached_event_id IS NULL);

-- `event_id` has carried no foreign key, so a row may name an event that does not exist. Such a
-- row keeps the id it recorded in `detached_event_id` and leaves every event's list; then the
-- key holds for every row. The save route maps the key's violation to `404 event not found`. A
-- fire mission outlives its event: deleting the event clears the pointer and keeps the row.
UPDATE public.fire_missions AS fire_mission
   SET detached_event_id = fire_mission.event_id,
       event_id = NULL
 WHERE fire_mission.event_id IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM public.events AS event WHERE event.id = fire_mission.event_id);

ALTER TABLE public.fire_missions
    ADD CONSTRAINT fire_missions_event_id_fkey FOREIGN KEY (event_id)
        REFERENCES public.events (id) ON DELETE SET NULL;

-- One row per gun of a catalog-model fire mission, with the server's solution for it. The
-- elevation, charge and time of flight are the solution and are null together, when no charge
-- solves for that gun. Deleting the fire mission deletes its guns.
CREATE TABLE public.fire_mission_guns (
    fire_mission_id uuid NOT NULL REFERENCES public.fire_missions (id) ON DELETE CASCADE,
    gun_index smallint NOT NULL CHECK (gun_index >= 0),
    label text NOT NULL CHECK (length(label) > 0),
    x double precision NOT NULL,
    y double precision NOT NULL,
    height_m double precision NOT NULL,
    height_source text NOT NULL CHECK (height_source IN ('dem', 'manual')),
    azimuth_mils double precision NOT NULL CHECK (azimuth_mils >= 0),
    elevation_mils double precision,
    charge_rings smallint CHECK (charge_rings >= 0),
    time_of_flight_s double precision CHECK (time_of_flight_s >= 0),
    PRIMARY KEY (fire_mission_id, gun_index),
    CONSTRAINT fire_mission_guns_solution_all_or_none
        CHECK (num_nulls(elevation_mils, charge_rings, time_of_flight_s) IN (0, 3))
);
