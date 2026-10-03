-- Every event owns exactly three reservation pools. Member and guest participants use their own
-- pool first and fall back to the open pool only after the open pool opens. A zero limit closes a
-- pool; a NULL limit leaves it uncapped. Opening times are absolute UTC instants.
CREATE TABLE public.event_reservation_quota_pools (
    event_id uuid NOT NULL REFERENCES public.events(id) ON DELETE CASCADE,
    quota_kind text NOT NULL CHECK (quota_kind IN ('member', 'guest', 'open')),
    seat_limit bigint CHECK (seat_limit IS NULL OR seat_limit BETWEEN 0 AND 4294967295),
    opens_at timestamptz NOT NULL,
    PRIMARY KEY (event_id, quota_kind)
);

-- New events default to verified TBD members: the member pool is uncapped and open from creation,
-- while guest and open pools grant no places until a manager configures them.
CREATE FUNCTION public.create_default_reservation_quota_pools() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO public.event_reservation_quota_pools (event_id, quota_kind, seat_limit, opens_at)
    SELECT NEW.id, pool.quota_kind, pool.seat_limit, COALESCE(NEW.created_at, clock_timestamp())
    FROM (VALUES ('member', NULL::bigint), ('guest', 0::bigint), ('open', 0::bigint))
        AS pool(quota_kind, seat_limit);
    RETURN NULL;
END;
$$;
CREATE TRIGGER default_reservation_quota_pools AFTER INSERT ON public.events
    FOR EACH ROW EXECUTE FUNCTION public.create_default_reservation_quota_pools();

INSERT INTO public.event_reservation_quota_pools (event_id, quota_kind, seat_limit, opens_at)
SELECT event_row.id, pool.quota_kind, pool.seat_limit, COALESCE(event_row.created_at, clock_timestamp())
FROM public.events event_row
CROSS JOIN (VALUES ('member', NULL::bigint), ('guest', 0::bigint), ('open', 0::bigint))
    AS pool(quota_kind, seat_limit);
