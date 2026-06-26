-- 02_campaign_refactor.sql
-- Runs BEFORE GORM AutoMigrate. Transitions the 1:1 Event<->Mission model to the
-- Campaign container model: an Event now holds many missions via event_missions,
-- and ORBAT slots / registrations hang off event_mission_id instead of event_id.
--
-- Clean cutover (no backfill): drop the legacy columns/indexes and clear the
-- affected tables so AutoMigrate can add the new NOT NULL event_mission_id column
-- and create event_missions. All guards are idempotent and tolerate a fresh DB
-- where the tables do not exist yet.

-- events: drop the strict 1:1 mission_id column.
DO $$ BEGIN
    IF to_regclass('public.events') IS NOT NULL THEN
        ALTER TABLE events DROP COLUMN IF EXISTS mission_id;
    END IF;
END $$;

-- orbat_slots: drop legacy unique index + event_id, clear rows for the NOT NULL add.
DO $$ BEGIN
    IF to_regclass('public.orbat_slots') IS NOT NULL THEN
        DROP INDEX IF EXISTS idx_orbat_slot;
        IF EXISTS (SELECT 1 FROM information_schema.columns
                   WHERE table_name = 'orbat_slots' AND column_name = 'event_id') THEN
            TRUNCATE TABLE orbat_slots;
            ALTER TABLE orbat_slots DROP COLUMN event_id;
        END IF;
    END IF;
END $$;

-- event_registrations: drop legacy unique index + event_id, clear rows likewise.
DO $$ BEGIN
    IF to_regclass('public.event_registrations') IS NOT NULL THEN
        DROP INDEX IF EXISTS idx_reg_unique;
        IF EXISTS (SELECT 1 FROM information_schema.columns
                   WHERE table_name = 'event_registrations' AND column_name = 'event_id') THEN
            TRUNCATE TABLE event_registrations;
            ALTER TABLE event_registrations DROP COLUMN event_id;
        END IF;
    END IF;
END $$;
