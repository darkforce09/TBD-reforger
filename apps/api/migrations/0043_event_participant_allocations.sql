-- One active quota allocation represents one distinct event participant. Every active mission
-- reservation of that participant references it; waitlisted and withdrawn registrations hold none.
-- Allocations acquired before this migration cannot identify the pool they consumed, so they
-- record legacy_unclassified: they count toward the event total and toward no pool.
-- Reservation, group and slot writers remain quiesced throughout the coordinated cutover.
SET LOCAL lock_timeout = '30s';
LOCK TABLE public.users, public.events, public.event_missions, public.orbat_slots,
    public.event_registrations, public.event_groups, public.event_group_roster
    IN SHARE ROW EXCLUSIVE MODE;

-- A group or roster entry is authored either by a manager or by a named system transition.
ALTER TABLE public.event_groups ALTER COLUMN created_by DROP NOT NULL,
    ADD COLUMN system_origin text CHECK (system_origin IS NULL OR length(btrim(system_origin)) > 0),
    ADD CONSTRAINT event_groups_single_provenance CHECK ((created_by IS NULL) <> (system_origin IS NULL));
ALTER TABLE public.event_group_roster ALTER COLUMN added_by DROP NOT NULL,
    ADD COLUMN system_origin text CHECK (system_origin IS NULL OR length(btrim(system_origin)) > 0),
    ADD CONSTRAINT event_group_roster_single_provenance CHECK ((added_by IS NULL) <> (system_origin IS NULL));

CREATE TABLE public.event_participant_allocations (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id uuid NOT NULL REFERENCES public.events(id) ON DELETE CASCADE,
    discord_id text NOT NULL REFERENCES public.users(discord_id) ON DELETE CASCADE,
    quota_kind text NOT NULL CHECK (quota_kind IN ('member', 'guest', 'open', 'legacy_unclassified')),
    acquired_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    released_at timestamptz,
    release_reason text CHECK (release_reason IS NULL OR length(btrim(release_reason)) > 0),
    CHECK ((released_at IS NULL) = (release_reason IS NULL)),
    CHECK (released_at IS NULL OR released_at >= acquired_at)
);
CREATE UNIQUE INDEX event_participant_allocations_one_active
    ON public.event_participant_allocations (event_id, discord_id) WHERE released_at IS NULL;
CREATE INDEX event_participant_allocations_active_by_account
    ON public.event_participant_allocations (discord_id) WHERE released_at IS NULL;

-- History records which allocation each reservation transition consumed.
ALTER TABLE public.event_registrations ADD COLUMN allocation_id uuid
    REFERENCES public.event_participant_allocations(id) ON DELETE RESTRICT;
ALTER TABLE public.event_registration_history ADD COLUMN allocation_id uuid
    REFERENCES public.event_participant_allocations(id) ON DELETE SET NULL;
CREATE INDEX event_registrations_allocation ON public.event_registrations (allocation_id)
    WHERE allocation_id IS NOT NULL;
CREATE OR REPLACE FUNCTION public.record_registration_history() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' OR (OLD.reservation_state, OLD.slot_id, OLD.release_reason, OLD.allocation_id)
        IS DISTINCT FROM (NEW.reservation_state, NEW.slot_id, NEW.release_reason, NEW.allocation_id) THEN
        INSERT INTO public.event_registration_history
            (registration_id, reservation_state, slot_id, release_reason, allocation_id)
            VALUES (NEW.id, NEW.reservation_state, NEW.slot_id, NEW.release_reason, NEW.allocation_id);
    END IF;
    RETURN NULL;
END;
$$;

-- An active reservation references its participant's own active allocation, and an active
-- allocation always has an active reservation or an occupied seat in the same event.
CREATE FUNCTION public.participant_allocation_is_consistent(target_event uuid, target_account text)
RETURNS boolean LANGUAGE sql STABLE AS $$
    SELECT NOT EXISTS (
        SELECT 1 FROM public.event_registrations registration
        JOIN public.event_missions mission ON mission.id = registration.event_mission_id
        LEFT JOIN public.event_participant_allocations allocation ON allocation.id = registration.allocation_id
        WHERE mission.event_id = target_event AND registration.discord_id = target_account
            AND registration.reservation_state IN ('registered', 'legacy_unknown')
            AND (allocation.id IS NULL OR allocation.released_at IS NOT NULL
                OR allocation.event_id <> target_event OR allocation.discord_id <> target_account))
    AND (NOT EXISTS (
            SELECT 1 FROM public.event_participant_allocations allocation
            WHERE allocation.event_id = target_event AND allocation.discord_id = target_account
                AND allocation.released_at IS NULL)
        OR EXISTS (
            SELECT 1 FROM public.event_registrations registration
            JOIN public.event_missions mission ON mission.id = registration.event_mission_id
            WHERE mission.event_id = target_event AND registration.discord_id = target_account
                AND registration.reservation_state IN ('registered', 'legacy_unknown'))
        OR EXISTS (
            SELECT 1 FROM public.orbat_slots slot
            JOIN public.event_missions mission ON mission.id = slot.event_mission_id
            WHERE mission.event_id = target_event AND slot.assigned_to = target_account))
$$;
CREATE FUNCTION public.validate_participant_allocation() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    target_event uuid;
    accounts text[];
BEGIN
    IF TG_TABLE_NAME = 'event_participant_allocations' THEN
        target_event := NEW.event_id;
        accounts := ARRAY[NEW.discord_id];
    ELSE
        SELECT mission.event_id INTO target_event
        FROM public.event_missions mission WHERE mission.id = NEW.event_mission_id;
        IF TG_TABLE_NAME = 'event_registrations' THEN
            accounts := ARRAY[NEW.discord_id];
        ELSIF TG_OP = 'UPDATE' THEN
            accounts := ARRAY[NEW.assigned_to, OLD.assigned_to];
        ELSE
            accounts := ARRAY[NEW.assigned_to];
        END IF;
    END IF;
    IF target_event IS NOT NULL AND EXISTS (
        SELECT 1 FROM unnest(accounts) AS account
        WHERE account IS NOT NULL
            AND NOT public.participant_allocation_is_consistent(target_event, account)) THEN
        RAISE EXCEPTION 'an active event participant requires exactly its own active quota allocation'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- Data changes follow every column and function definition: PostgreSQL refuses to alter a table
-- while deferred constraint checks of earlier statements are still pending.
-- Two active reservations can never claim one seat. Existing duplicates keep the claim that the
-- seat itself records, then the earliest signup; the others become seatless place holds.
CREATE TEMPORARY TABLE repaired_slot_claims ON COMMIT DROP AS
SELECT ranked.id FROM (
    SELECT registration.id, row_number() OVER (
        PARTITION BY registration.event_mission_id, registration.slot_id
        ORDER BY (slot.assigned_to IS NOT DISTINCT FROM registration.discord_id) DESC,
            registration.registered_at, registration.id) AS claim_rank
    FROM public.event_registrations registration
    JOIN public.orbat_slots slot ON slot.id = registration.slot_id
    WHERE registration.reservation_state IN ('registered', 'legacy_unknown')
) ranked WHERE ranked.claim_rank > 1;
UPDATE public.event_registrations SET slot_id = NULL
WHERE id IN (SELECT id FROM repaired_slot_claims);

CREATE TEMPORARY TABLE backfilled_participants ON COMMIT DROP AS
SELECT mission.event_id, participant.discord_id, min(participant.since) AS since
FROM (
    SELECT registration.event_mission_id, registration.discord_id, registration.registered_at AS since
    FROM public.event_registrations registration
    WHERE registration.reservation_state IN ('registered', 'legacy_unknown')
    UNION ALL
    SELECT slot.event_mission_id, slot.assigned_to, COALESCE(slot.assigned_at, clock_timestamp())
    FROM public.orbat_slots slot WHERE slot.assigned_to IS NOT NULL
) participant
JOIN public.event_missions mission ON mission.id = participant.event_mission_id
GROUP BY mission.event_id, participant.discord_id;
INSERT INTO public.event_participant_allocations (event_id, discord_id, quota_kind, acquired_at)
SELECT event_id, discord_id, 'legacy_unclassified', since FROM backfilled_participants;
UPDATE public.event_registrations registration SET allocation_id = allocation.id
FROM public.event_missions mission, public.event_participant_allocations allocation
WHERE mission.id = registration.event_mission_id AND allocation.event_id = mission.event_id
    AND allocation.discord_id = registration.discord_id AND allocation.released_at IS NULL
    AND registration.reservation_state IN ('registered', 'legacy_unknown');

-- Check the rewritten rows now, then install the constraints that guard every later write.
SET CONSTRAINTS ALL IMMEDIATE;
CREATE UNIQUE INDEX event_registrations_one_active_claim_per_slot
    ON public.event_registrations (event_mission_id, slot_id)
    WHERE slot_id IS NOT NULL AND reservation_state IN ('registered', 'legacy_unknown');
ALTER TABLE public.event_registrations ADD CONSTRAINT registration_allocation_matches_reservation
    CHECK ((reservation_state IN ('registered', 'legacy_unknown')) = (allocation_id IS NOT NULL));
CREATE CONSTRAINT TRIGGER registration_allocation_check
    AFTER INSERT OR UPDATE ON public.event_registrations DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION public.validate_participant_allocation();
CREATE CONSTRAINT TRIGGER slot_allocation_check
    AFTER INSERT OR UPDATE OF assigned_to ON public.orbat_slots DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION public.validate_participant_allocation();
CREATE CONSTRAINT TRIGGER allocation_participant_check
    AFTER INSERT OR UPDATE ON public.event_participant_allocations DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION public.validate_participant_allocation();

-- Enforcing the default TBD-member policy must not silently evict people who joined upcoming
-- events under the earlier open rules. Each affected event receives a visible roster group and
-- an explicit grant for it; managers remove either when they choose.
CREATE TEMPORARY TABLE enforcement_participants ON COMMIT DROP AS
SELECT DISTINCT mission.event_id, participant.discord_id
FROM (
    SELECT event_mission_id, discord_id FROM public.event_registrations
    WHERE reservation_state IN ('registered', 'waitlisted', 'legacy_unknown')
    UNION SELECT event_mission_id, assigned_to FROM public.orbat_slots WHERE assigned_to IS NOT NULL
) participant
JOIN public.event_missions mission ON mission.id = participant.event_mission_id AND mission.deleted_at IS NULL
JOIN public.events event_row ON event_row.id = mission.event_id
WHERE event_row.deleted_at IS NULL AND event_row.status NOT IN ('completed', 'cancelled');
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM public.events event_row
        WHERE event_row.id IN (SELECT event_id FROM enforcement_participants)
            AND jsonb_array_length(event_row.access_policy -> 'grants') >= 32) THEN
        RAISE EXCEPTION 'an event access policy has no room for the enforcement grant';
    END IF;
END;
$$;
CREATE TEMPORARY TABLE enforcement_groups (id uuid PRIMARY KEY, event_id uuid NOT NULL) ON COMMIT DROP;
WITH created AS (
    INSERT INTO public.event_groups (event_id, name, source, system_origin)
    SELECT DISTINCT event_id, 'Participants before access enforcement',
        '{"kind":"managed_roster"}'::jsonb, 'access_enforcement_migration_0043'
    FROM enforcement_participants
    RETURNING id, event_id)
INSERT INTO enforcement_groups (id, event_id) SELECT id, event_id FROM created;
INSERT INTO public.event_group_roster (group_id, discord_id, system_origin)
SELECT grp.id, participant.discord_id, 'access_enforcement_migration_0043'
FROM enforcement_groups grp JOIN enforcement_participants participant USING (event_id);
UPDATE public.events event_row SET
    access_policy = jsonb_set(event_row.access_policy, '{grants}',
        (event_row.access_policy -> 'grants') || jsonb_build_array(jsonb_build_object('conditions',
            jsonb_build_array(jsonb_build_object('kind', 'event_group', 'group_id', grp.id::text))))),
    access_revision = event_row.access_revision + 1
FROM enforcement_groups grp WHERE grp.event_id = event_row.id;

INSERT INTO public.audit_logs (severity, actor_id, actor_name, action, message, target_type, target_id, metadata, created_at)
SELECT 'info', NULL, 'system', 'operations.participant_allocations_introduced',
    'Recorded one unclassified allocation per existing participant, repaired duplicate seat claims, '
        'and kept existing upcoming participants eligible through a visible roster group',
    'migration', '0043', jsonb_build_object('migration_version', 43,
        'allocations', (SELECT count(*) FROM backfilled_participants),
        'repaired_slot_claims', (SELECT count(*) FROM repaired_slot_claims),
        'enforcement_groups', (SELECT count(*) FROM enforcement_groups),
        'enforcement_roster_entries', (SELECT count(*) FROM enforcement_participants)),
    clock_timestamp();
