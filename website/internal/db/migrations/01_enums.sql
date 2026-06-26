-- 01_enums.sql
-- Runs BEFORE GORM AutoMigrate. AutoMigrate does not manage Postgres ENUM types,
-- so they must exist before the model columns that reference them are created.
-- Each block is idempotent (safe to re-run).

DO $$ BEGIN
    CREATE TYPE user_role AS ENUM ('enlisted','mission_maker','admin');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

-- 'leader' added after the initial release; idempotent so existing DBs pick it up.
ALTER TYPE user_role ADD VALUE IF NOT EXISTS 'leader';

DO $$ BEGIN
    CREATE TYPE mission_status AS ENUM ('draft','pending_approval','live','rejected','archived');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

DO $$ BEGIN
    CREATE TYPE terrain_type AS ENUM ('everon','arland','custom');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

DO $$ BEGIN
    CREATE TYPE game_mode AS ENUM ('pve_coop','pvp','zeus');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

DO $$ BEGIN
    CREATE TYPE weather_type AS ENUM ('clear','overcast','heavy_rain','dense_fog');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

DO $$ BEGIN
    CREATE TYPE event_status AS ENUM ('scheduled','open','locked','live','completed','cancelled');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

DO $$ BEGIN
    CREATE TYPE registration_state AS ENUM ('registered','waitlisted','withdrawn','attended','no_show');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

DO $$ BEGIN
    CREATE TYPE mission_outcome AS ENUM ('success','failure','aborted','pending');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

DO $$ BEGIN
    CREATE TYPE announcement_status AS ENUM ('draft','published','archived');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

DO $$ BEGIN
    CREATE TYPE announcement_tag AS ENUM ('update','event','modpack_update','important');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

DO $$ BEGIN
    CREATE TYPE audit_severity AS ENUM ('info','warn','crit');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

DO $$ BEGIN
    CREATE TYPE leave_status AS ENUM ('pending','approved','denied');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;
