-- One-off backfill for accounts that linked before link-confirm claimed their match rows.
--
-- Link-confirm is deliberately FORWARD-ONLY: `identity_and_access::handlers::
-- arma_link_confirmation` claims `match_player_stats.discord_id IS NULL` rows at the moment an
-- account links. Accounts linked before that claim existed keep orphan rows forever, so
-- `users.total_deployments` and `leaderboard_totals` stay permanently short (measured: three
-- pre-link ops read as total_deployments=1 / leaderboard kills=2 against a true 4 / 22).
--
-- This migration is the missing one-shot. It mirrors link-confirm's SQL, then the same
-- derived-number refresh that link-confirm does post-commit — entirely in SQL, so a migration
-- runner never calls into Rust.
--
-- ── CLAIM RULE (intentional) ─────────────────────────────────────────────────────────
-- Join is on arma_id alone → rows go to whoever holds that Steam id NOW. Unlink makes
-- sequential ownership legitimate; a historical owner who unlinked already had their rows
-- released. Soft-deleted users are excluded (`deleted_at IS NULL`) so the claim matches the
-- ingest resolver (`WHERE arma_id = $1 AND deleted_at IS NULL`), not a dead account.
--
-- ── PADDED users.arma_id (deliberate NORMALIZE) ──────────────────────────────────────
-- An earlier link-confirm could store arma_id UNTRIMMED while ingest binds the trimmed id.
-- Those accounts read as linked but miss every match join. `btrim` (and NULLIF empty) runs
-- in the SAME migration BEFORE the claim so (a) the claim join hits and (b) future ingest
-- resolves them. Residual after this step: none for whitespace padding. UNIQUE(arma_id)
-- fails loudly if two padded variants collide — preferred over silently leaving orphans.
--
-- ── OUT OF SCOPE ─────────────────────────────────────────────────────────────────────
-- Attendance flip (`BACKFILL_ATTENDANCE` / event_registrations → attended): a registration
-- was always that discord_id's, and the pre-flip state was never recorded, so no attended
-- rows are invented here. `attendance_rate` is still recomputed from EXISTING registration
-- states (same as `recompute_user_stats`), because total_deployments moved and the
-- denormalized pair is one function.
--
-- `match_player_stats.command_win` NULL semantics (0014) are untouched.
--
-- ── IDEMPOTENCY ──────────────────────────────────────────────────────────────────────
-- Safe on a DB with zero claimable rows. Safe to re-run by hand: `discord_id IS NULL`
-- is empty after a successful pass; the normalize WHERE is empty after trim; recompute
-- / REFRESH are pure refreshes. sqlx applies the file in one transaction.

UPDATE public.users
   SET arma_id = NULLIF(btrim(arma_id), ''),
       updated_at = now()
 WHERE arma_id IS NOT NULL
   AND arma_id <> btrim(arma_id);

-- 2) Claim orphan match_player_stats for currently-linked accounts (link-confirm's claim, plus
--    the deleted_at exclusion).
UPDATE public.match_player_stats AS s
   SET discord_id = u.discord_id
  FROM public.users AS u
 WHERE u.arma_id = s.arma_id
   AND u.deleted_at IS NULL
   AND u.arma_id IS NOT NULL
   AND u.arma_id <> ''
   AND s.discord_id IS NULL;

-- 3) Recompute denormalized user stats for every currently-linked user. Broader than
--    "rows_affected" on purpose: a prior half-applied attempt (claim without recompute)
--    still corrects totals; linked-user cardinality is tiny. Mirrors
--    `command_center::services::user_stats::recompute_user_stats` (deployments +
--    attendance_rate from EXISTING registrations — no attendance flip).
UPDATE public.users AS u
   SET total_deployments = sub.deployments,
       attendance_rate = CASE
         WHEN sub.past_registered > 0
           THEN ((sub.attended::float8 / sub.past_registered::float8) * 100.0)::numeric
         ELSE 0::numeric
       END
  FROM (
    SELECT u2.discord_id,
           (SELECT count(DISTINCT mps.match_id)
              FROM public.match_player_stats AS mps
             WHERE mps.discord_id = u2.discord_id) AS deployments,
           (SELECT count(*)::bigint
              FROM public.event_registrations AS er
             WHERE er.discord_id = u2.discord_id
               AND er.state::text = 'attended') AS attended,
           (SELECT count(*)::bigint
              FROM public.event_registrations AS er
              JOIN public.event_missions AS em ON em.id = er.event_mission_id
             WHERE er.discord_id = u2.discord_id
               AND em.start_time <= now()) AS past_registered
      FROM public.users AS u2
     WHERE u2.arma_id IS NOT NULL
       AND u2.arma_id <> ''
       AND u2.deleted_at IS NULL
  ) AS sub
 WHERE u.discord_id = sub.discord_id;

-- 4) Leaderboard reads match_player_stats.discord_id directly (0014). Refresh once.
--    Non-concurrent: migrations run inside a transaction; CONCURRENTLY cannot.
REFRESH MATERIALIZED VIEW public.leaderboard_totals;
