-- Historical gameplay attribution follows the current exact Arma identity owner.
-- API and background writers remain quiesced until this transaction commits and
-- every instance uses the identity-serialization protocol.
SET LOCAL lock_timeout = '30s';

-- Readers remain available; ownership, player facts, and attendance populations
-- cannot change while attribution and its derived aggregates are reconciled.
LOCK TABLE public.users, public.match_player_stats,
    public.event_missions, public.event_registrations IN SHARE ROW EXCLUSIVE MODE;

DO $$
DECLARE
    repaired_rows bigint;
BEGIN
    WITH ownership AS (
        SELECT stats.id, owner.discord_id
        FROM public.match_player_stats AS stats
        LEFT JOIN public.users AS owner
            ON owner.arma_id = stats.arma_id
            AND owner.deleted_at IS NULL
            AND btrim(owner.arma_id) <> ''
    )
    UPDATE public.match_player_stats AS stats
    SET discord_id = ownership.discord_id
    FROM ownership
    WHERE stats.id = ownership.id
        AND stats.discord_id IS DISTINCT FROM ownership.discord_id;

    GET DIAGNOSTICS repaired_rows = ROW_COUNT;

    UPDATE public.users AS account
    SET total_deployments = (
        SELECT count(DISTINCT match_id)
        FROM public.match_player_stats
        WHERE discord_id = account.discord_id
    ),
    attendance_rate = (
        SELECT COALESCE(
            ROUND(100.0 * count(*) FILTER (WHERE registration.state::text = 'attended')
                / NULLIF(count(*), 0), 2),
            0
        )
        FROM public.event_registrations AS registration
        JOIN public.event_missions AS mission ON mission.id = registration.event_mission_id
        WHERE registration.discord_id = account.discord_id
            AND mission.start_time <= now()
    );

    PERFORM pg_advisory_xact_lock(hashtextextended('tbd.leaderboard_totals.refresh', 0));
    REFRESH MATERIALIZED VIEW public.leaderboard_totals;

    INSERT INTO public.audit_logs (
        severity, actor_id, actor_name, action, message, target_type, target_id,
        metadata, created_at
    ) VALUES (
        'info', NULL, 'system', 'identity.historical_attribution_reconciled',
        'Reconciled historical gameplay ownership and derived aggregates; factual authorship remains unchanged',
        'migration', '0035',
        jsonb_build_object('migration_version', 35, 'repaired_player_rows', repaired_rows),
        clock_timestamp()
    );
END;
$$;
