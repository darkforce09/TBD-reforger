-- Durable requests to re-evaluate an event's reservations after a change that happened outside
-- the event's lock scope: membership observations, bans, account deletion, and pool opening times.
-- Producers only upsert this row, so they never acquire event locks while holding account locks.
-- A worker leases due rows and runs a separate event-first transaction; a request that changes
-- while leased keeps its row for another pass.
CREATE TABLE public.event_reservation_reevaluations (
    event_id uuid PRIMARY KEY REFERENCES public.events(id) ON DELETE CASCADE,
    due_at timestamptz NOT NULL,
    revision bigint NOT NULL DEFAULT 1 CHECK (revision > 0),
    requested_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    lease_token uuid,
    lease_expires_at timestamptz,
    attempts integer NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    last_error text,
    CHECK ((lease_token IS NULL) = (lease_expires_at IS NULL))
);
CREATE INDEX event_reservation_reevaluations_due ON public.event_reservation_reevaluations (due_at);
