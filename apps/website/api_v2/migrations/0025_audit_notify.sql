-- Audit rows written by row triggers, and the NOTIFY the admin stream waits on.
--
-- Three actions write their audit rows through triggers rather than through the handlers —
-- event create (`INSERT INTO events`), mission soft-delete (`UPDATE missions SET deleted_at =
-- now()`) and slot kick (`UPDATE event_registrations SET slot_id = NULL`) — which also means the
-- rows cannot drift from the write path: whatever sets the column writes the row, the REST
-- handler included. And every `audit_logs` insert announces itself, so the admin stream pushes
-- instead of polling the table.
--
-- 1. `audit_logs_notify` — AFTER INSERT ON audit_logs → `pg_notify('audit_log', id)`. Every audit
--    row announces itself, whether `administration::services::audit_writer::write_audit` or a
--    trigger below inserted it. `administration::services::audit_notifier` holds the one
--    `LISTEN audit_log` connection per pool and the SSE stream fetches `id > last_id` on each
--    announcement; its 2 s poll survives only as the fallback while that connection is down.
-- 2. `audit_event_created` — AFTER INSERT ON events → `event.create`, actor = `created_by`.
-- 3. `audit_mission_deleted` — AFTER UPDATE OF deleted_at ON missions, on the NULL → NOT NULL edge
--    only (a repeated soft-delete or a restore writes nothing) → `mission.delete`.
-- 4. `audit_slot_kicked` — AFTER UPDATE OF slot_id ON event_registrations, on the NOT NULL → NULL
--    edge only → `event.slot_kick`. Taking a seat writes nothing; a withdraw is a DELETE and never
--    fires it; deleting the ORBAT slot reaches the same edge through 0018's `ON DELETE SET NULL`
--    and is audited too — the occupant lost the seat either way.
--
-- Actor: only the events row carries who acted (`created_by`). `delete_mission` and `clear_slot`
-- stamp nothing the trigger could read, so those rows carry actor_id NULL / actor_name '' and name
-- their trigger in `metadata` — an honest "unrecorded" rather than a fabricated "system".
--
-- Everything is CREATE OR REPLACE / DROP IF EXISTS so a re-run of this file is a no-op.


CREATE OR REPLACE FUNCTION public.audit_logs_notify() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    PERFORM pg_notify('audit_log', NEW.id::text);
    RETURN NULL;
END
$$;

DROP TRIGGER IF EXISTS audit_logs_notify ON public.audit_logs;
CREATE TRIGGER audit_logs_notify
    AFTER INSERT ON public.audit_logs
    FOR EACH ROW EXECUTE FUNCTION public.audit_logs_notify();

-- ── 2. The one writer the three row triggers share ─────────────────────────────────────────────
-- Mirrors `audit_writer::write_audit`'s INSERT column for column; `actor_name` is resolved from
-- `users.username` the way the handlers do, '' when there is no actor.

CREATE OR REPLACE FUNCTION public.audit_trigger_write(
    p_severity    public.audit_severity,
    p_actor_id    text,
    p_action      text,
    p_message     text,
    p_target_type text,
    p_target_id   text,
    p_metadata    jsonb
) RETURNS void
LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO public.audit_logs
        (severity, actor_id, actor_name, action, message, target_type, target_id, metadata, created_at)
    VALUES (
        p_severity,
        p_actor_id,
        COALESCE((SELECT u.username FROM public.users u WHERE u.discord_id = p_actor_id), ''),
        p_action,
        p_message,
        p_target_type,
        p_target_id,
        p_metadata,
        now()
    );
END
$$;

-- ── 3. event.create ────────────────────────────────────────────────────────────────────────────

CREATE OR REPLACE FUNCTION public.audit_event_created() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    PERFORM public.audit_trigger_write(
        'info',
        NEW.created_by,
        'event.create',
        format('Event %s created', COALESCE(NULLIF(NEW.name_override, ''), NEW.id::text)),
        'event',
        NEW.id::text,
        jsonb_build_object(
            'trigger', 'audit_event_created',
            'start_time', NEW.start_time,
            'status', NEW.status::text
        )
    );
    RETURN NULL;
END
$$;

DROP TRIGGER IF EXISTS audit_event_created ON public.events;
CREATE TRIGGER audit_event_created
    AFTER INSERT ON public.events
    FOR EACH ROW EXECUTE FUNCTION public.audit_event_created();

-- ── 4. mission.delete ──────────────────────────────────────────────────────────────────────────

CREATE OR REPLACE FUNCTION public.audit_mission_deleted() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    PERFORM public.audit_trigger_write(
        'warn',
        NULL,
        'mission.delete',
        format('Mission "%s" soft-deleted', NEW.title),
        'mission',
        NEW.id::text,
        jsonb_build_object(
            'trigger', 'audit_mission_deleted',
            'author_id', NEW.author_id,
            'deleted_at', NEW.deleted_at
        )
    );
    RETURN NULL;
END
$$;

DROP TRIGGER IF EXISTS audit_mission_deleted ON public.missions;
CREATE TRIGGER audit_mission_deleted
    AFTER UPDATE OF deleted_at ON public.missions
    FOR EACH ROW
    WHEN (OLD.deleted_at IS NULL AND NEW.deleted_at IS NOT NULL)
    EXECUTE FUNCTION public.audit_mission_deleted();

-- ── 5. event.slot_kick ─────────────────────────────────────────────────────────────────────────

CREATE OR REPLACE FUNCTION public.audit_slot_kicked() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_player text;
    v_slot   text;
BEGIN
    SELECT u.username INTO v_player
      FROM public.users u WHERE u.discord_id = NEW.discord_id;
    -- NULL when the slot row itself is being deleted (the ON DELETE SET NULL path).
    SELECT COALESCE(NULLIF(s.callsign, ''), s.role) INTO v_slot
      FROM public.orbat_slots s WHERE s.id = OLD.slot_id;
    PERFORM public.audit_trigger_write(
        'warn',
        NULL,
        'event.slot_kick',
        format('%s removed from slot %s',
               COALESCE(v_player, NEW.discord_id),
               COALESCE(v_slot, OLD.slot_id::text)),
        'event_mission',
        NEW.event_mission_id::text,
        jsonb_build_object(
            'trigger', 'audit_slot_kicked',
            'registration_id', NEW.id::text,
            'discord_id', NEW.discord_id,
            'slot_id', OLD.slot_id::text
        )
    );
    RETURN NULL;
END
$$;

DROP TRIGGER IF EXISTS audit_slot_kicked ON public.event_registrations;
CREATE TRIGGER audit_slot_kicked
    AFTER UPDATE OF slot_id ON public.event_registrations
    FOR EACH ROW
    WHEN (OLD.slot_id IS NOT NULL AND NEW.slot_id IS NULL)
    EXECUTE FUNCTION public.audit_slot_kicked();
