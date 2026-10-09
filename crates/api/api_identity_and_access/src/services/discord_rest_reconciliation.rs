//! Cluster-coordinated REST reconciliation with durable leases and rate-limit backoff.
//!
//! **Role:** refreshes one due Discord membership per call: claims its lease, spends the shared
//! request budget, reads the member from Discord, records the observation or the failure, and
//! reports how the request ended.
//! **Position:** `api_identity_and_access` service; driven by the membership reconciliation worker
//! (`api_background_workers::discord_membership_reconciler`) and requested by the OAuth and
//! administration hooks through [`request_account_refresh`]; reads Discord through
//! [`api_discord::discord_client::DiscordService`].
//! **Signals & state:** the `discord_membership_snapshots` leases and the singleton
//! `discord_rest_schedule` row in PostgreSQL; every request that reaches Discord adds one to
//! `tbd_discord_reconcile_outcomes_total` in the application state's metrics registry
//! ([`AppState::metrics_registry`]) and writes one `discord_reconciliation` log line.
//! **Invariants:** at most 25 Discord requests per second across replicas; an observation or a
//! failure is recorded only under the lease that requested it; a failure never changes the
//! recorded membership; the log line names no account, guild id or token.

use super::discord_membership_cache::{
    MembershipRefreshLease, accept_membership_observation, claim_membership_refresh,
    record_membership_failure,
};
use api_http_layer::observability::metrics_registry::{DiscordReconcileOutcome, Registry};
use api_identifiers::{DiscordGuildId, DiscordUserId};
use api_state::AppState;
use sqlx::PgPool;

/// `guild_scope` of a reconciliation of the configured main guild.
const MAIN_GUILD_SCOPE: &str = "main";
/// `guild_scope` of a reconciliation of any other guild: an event's partner guild.
const PARTNER_GUILD_SCOPE: &str = "partner";

/// Seed unknown accounts without inferring membership from a preexisting role cache.
pub async fn enroll_accounts(pool: &PgPool, guild_id: &DiscordGuildId) -> sqlx::Result<()> {
    if guild_id.as_str().trim().is_empty() {
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO discord_membership_snapshots(discord_id, guild_id)
        SELECT discord_id, $1 FROM users WHERE deleted_at IS NULL ON CONFLICT DO NOTHING",
    )
    .bind(guild_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Admit at most 25 requests per second across replicas after obtaining the account lease.
/// Dispatch follows immediately; network timing still depends on fair OS scheduling.
async fn reserve_request_budget(
    pool: &PgPool,
    lease: &MembershipRefreshLease,
) -> sqlx::Result<bool> {
    let mut tx = pool.begin().await?;
    let admitted = sqlx::query("UPDATE discord_rest_schedule SET next_request_at = clock_timestamp() + interval '40 milliseconds'
        WHERE singleton AND next_request_at <= clock_timestamp() AND EXISTS (
            SELECT 1 FROM discord_membership_snapshots WHERE discord_id = $1 AND guild_id = $2
            AND revision = $3 AND lease_token = $4 AND lease_expires_at > clock_timestamp())")
        .bind(&lease.discord_id).bind(&lease.guild_id).bind(lease.revision).bind(lease.lease_token)
        .execute(&mut *tx).await?.rows_affected() == 1;
    if !admitted {
        sqlx::query(
            "UPDATE discord_membership_snapshots SET lease_token = NULL, lease_expires_at = NULL,
            next_refresh_at = GREATEST(next_refresh_at, clock_timestamp(),
                (SELECT next_request_at FROM discord_rest_schedule WHERE singleton))
            WHERE discord_id = $1 AND guild_id = $2 AND revision = $3 AND lease_token = $4",
        )
        .bind(&lease.discord_id)
        .bind(&lease.guild_id)
        .bind(lease.revision)
        .bind(lease.lease_token)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(admitted)
}

/// Refreshes the one membership whose refresh is due: claims its lease, reserves request budget,
/// reads the member from Discord and records the observation or the failure under that lease.
/// A request that reached Discord and was recorded is then counted and logged once (see
/// `report_outcome`). Answers `false` when nothing is due, the lease is taken or the budget is
/// spent.
pub async fn reconcile_one(state: &AppState) -> sqlx::Result<bool> {
    if state.cfg.discord_bot_token.is_empty() {
        return Ok(false);
    }
    let due: Option<(DiscordUserId, DiscordGuildId)> = sqlx::query_as("SELECT s.discord_id, s.guild_id FROM discord_membership_snapshots s JOIN users u USING(discord_id)
        WHERE u.deleted_at IS NULL AND next_refresh_at <= clock_timestamp() AND (lease_expires_at IS NULL OR lease_expires_at <= clock_timestamp())
        ORDER BY next_refresh_at, s.discord_id, s.guild_id LIMIT 1")
        .fetch_optional(&state.pool).await?;
    let Some((discord_id, guild_id)) = due else {
        return Ok(false);
    };
    let Some(lease) = claim_membership_refresh(&state.pool, &discord_id, &guild_id, false).await?
    else {
        return Ok(false);
    };
    if !reserve_request_budget(&state.pool, &lease).await? {
        return Ok(false);
    }
    let fetched = state
        .discord
        .fetch_member_with_bot(&state.cfg.discord_bot_token, &guild_id, &discord_id)
        .await;
    let (outcome, retry_after_ms) = match fetched {
        Ok(member) => {
            let recorded = accept_membership_observation(
                &state.pool,
                &lease,
                member.as_ref(),
                &state.cfg.discord_guild_id,
            )
            .await?;
            (observation_outcome(recorded, member.is_some()), None)
        }
        Err(failure) => {
            if failure.rate_limited {
                sqlx::query("UPDATE discord_rest_schedule SET next_request_at = GREATEST(next_request_at, clock_timestamp() + $1::bigint * interval '1 millisecond') WHERE singleton")
                    .bind(failure.retry_after.num_milliseconds()).execute(&state.pool).await?;
            }
            record_membership_failure(&state.pool, &lease, failure.retry_after, failure.reason)
                .await?;
            (
                failure.outcome(),
                Some(failure.retry_after.num_milliseconds()),
            )
        }
    };
    let guild_scope = if lease.guild_id == state.cfg.discord_guild_id {
        MAIN_GUILD_SCOPE
    } else {
        PARTNER_GUILD_SCOPE
    };
    report_outcome(
        &state.metrics_registry,
        outcome,
        guild_scope,
        retry_after_ms,
        lease.revision,
    );
    Ok(true)
}

/// The outcome of a Discord answer: recorded as a member or a non-member, or discarded because
/// the lease no longer held when the observation was written.
fn observation_outcome(recorded: bool, is_member: bool) -> DiscordReconcileOutcome {
    match (recorded, is_member) {
        (false, _) => DiscordReconcileOutcome::LeaseLost,
        (true, true) => DiscordReconcileOutcome::Member,
        (true, false) => DiscordReconcileOutcome::Nonmember,
    }
}

/// Counts one finished request in `metrics` ([`Registry::record_discord_reconcile_outcome`]) and
/// writes its `discord_reconciliation` log line: `outcome`, `guild_scope` (`main` or `partner`),
/// the snapshot `revision` the lease was claimed at, and `retry_after_ms`, present on a failure
/// only.
fn report_outcome(
    metrics: &Registry,
    outcome: DiscordReconcileOutcome,
    guild_scope: &'static str,
    retry_after_ms: Option<i64>,
    revision: i64,
) {
    metrics.record_discord_reconcile_outcome(outcome);
    let outcome = outcome.label();
    tracing::info!(
        target: "discord_reconciliation",
        outcome,
        guild_scope,
        retry_after_ms,
        revision,
        "Discord membership reconciliation finished"
    );
}

/// OAuth refresh and administration hooks request reconciliation without asserting affiliation.
pub async fn request_account_refresh(
    pool: &PgPool,
    discord_id: &DiscordUserId,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE discord_membership_snapshots SET next_refresh_at = LEAST(next_refresh_at, now()) WHERE discord_id = $1")
        .bind(discord_id).execute(pool).await?;
    Ok(())
}
