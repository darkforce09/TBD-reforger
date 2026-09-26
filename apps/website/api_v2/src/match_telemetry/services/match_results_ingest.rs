//! The transaction that decides and applies one results revision of a registered match.
//!
//! **Role:** locks the match, decides the revision, and for an applied revision writes the facts,
//! reconciles attendance, audits unlinked identities and recomputes every derived statistic
//! before committing.
//! **Position:** called by `handlers::match_results` with the machine caller and a validated
//! [`MatchResultsRevision`].
//! **Signals & state:** one transaction per call.
//! **Invariants:** lock order is the match row, the obligated registrants' attachments, the
//! identities (sorted), the accounts (sorted), then the leaderboard refresh last; a duplicate,
//! stale, conflicting or finalized-regressing revision writes nothing; an applied revision and
//! its derived statistics commit together, so no reader sees one without the other.

use serde_json::json;

use super::ingest_parsing::AUDIT_UNLINKED_ID_SAMPLE;
use super::match_revision_write::{delete_removed_lines, write_match_row, write_player_line};
use super::registered_match::lock_registered_match;
use super::results_revision::{RevisionDecision, decide_revision};
use crate::administration::services::required_audit::append_system_audit;
use crate::command_center::services::leaderboard_view::refresh_leaderboard_on_connection;
use crate::command_center::services::user_stats::recompute_user_stats_on_connection;
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::identity_and_access::services::identity_ownership::{lock_accounts, lock_identities};
use crate::match_telemetry::models::MissionOutcome;
use crate::match_telemetry::models::match_results_revision::{
    MatchResultsAnswer, MatchResultsRevision,
};
use crate::match_telemetry::models::telemetry_refusal::telemetry_conflict;
use crate::operations::services::participation_attribution::{
    lock_obligated_registrants, prior_match_accounts, reconcile_match,
};
use crate::server_infrastructure::services::machine_credentials::MachineCaller;

/// Refusal code of a revision older than the stored one.
pub const STALE_REVISION: &str = "STALE_REVISION";
/// Refusal code of the stored revision number with another report.
pub const REVISION_CONFLICT: &str = "REVISION_CONFLICT";
/// Refusal code of a revision that would return a finalized match to pending.
pub const MATCH_FINALIZED: &str = "MATCH_FINALIZED";

/// The owning account of each submitted line, in submission order.
async fn resolve_owners(
    connection: &mut sqlx::PgConnection,
    revision: &MatchResultsRevision,
) -> Result<Vec<Option<String>>, ApiError> {
    let mut owners = Vec::with_capacity(revision.players.len());
    for line in &revision.players {
        owners.push(
            sqlx::query_scalar(
                "SELECT discord_id FROM users WHERE arma_id = $1 AND deleted_at IS NULL",
            )
            .bind(line.arma_id.trim())
            .fetch_optional(&mut *connection)
            .await?,
        );
    }
    Ok(owners)
}

fn answer(
    match_id: uuid::Uuid,
    revision: &MatchResultsRevision,
    owners: &[Option<String>],
    applied: bool,
) -> MatchResultsAnswer {
    let mut unlinked_arma_ids: Vec<String> = Vec::new();
    for (line, owner) in revision.players.iter().zip(owners) {
        let arma_id = line.arma_id.trim().to_owned();
        if owner.is_none() && !unlinked_arma_ids.contains(&arma_id) {
            unlinked_arma_ids.push(arma_id);
        }
    }
    let unlinked = owners.iter().filter(|owner| owner.is_none()).count();
    MatchResultsAnswer {
        match_id,
        revision: revision.revision,
        applied,
        players: revision.players.len(),
        linked: revision.players.len() - unlinked,
        unlinked,
        unlinked_arma_ids,
    }
}

/// Decide `revision` against the match the caller's server registered, applying it when it is new.
pub async fn ingest_results_revision(
    state: &AppState,
    caller: &MachineCaller,
    revision: &MatchResultsRevision,
) -> Result<MatchResultsAnswer, ApiError> {
    let mut tx = state.pool.begin().await?;
    let stored =
        lock_registered_match(&mut tx, caller.server_id, &revision.source_match_id).await?;
    match decide_revision(
        stored.revision,
        stored.report_sha256.as_deref(),
        stored.finalized,
        revision.revision,
        &revision.report_sha256,
        revision.outcome == MissionOutcome::Pending,
    ) {
        RevisionDecision::Apply => {}
        RevisionDecision::Duplicate => {
            let owners = resolve_owners(&mut tx, revision).await?;
            tx.commit().await?;
            return Ok(answer(stored.id, revision, &owners, false));
        }
        RevisionDecision::Stale => {
            return Err(telemetry_conflict(
                STALE_REVISION,
                format!(
                    "revision {} is older than the stored revision",
                    revision.revision
                ),
                json!({ "match_id": stored.id, "revision": stored.revision }),
            ));
        }
        RevisionDecision::Conflict => {
            return Err(telemetry_conflict(
                REVISION_CONFLICT,
                "the stored revision carries another report",
                json!({
                    "match_id": stored.id,
                    "revision": stored.revision,
                    "report_sha256": stored.report_sha256,
                }),
            ));
        }
        RevisionDecision::FinalizedRegression => {
            return Err(telemetry_conflict(
                MATCH_FINALIZED,
                "a finalized match cannot return to pending",
                json!({ "match_id": stored.id, "revision": stored.revision }),
            ));
        }
    }

    let obligated = lock_obligated_registrants(
        &mut tx,
        (stored.event_id, stored.mission_id),
        revision.event_id,
        revision.mission_id,
    )
    .await?;
    let mut identities: Vec<String> =
        sqlx::query_scalar("SELECT DISTINCT arma_id FROM match_player_stats WHERE match_id = $1")
            .bind(stored.id)
            .fetch_all(&mut *tx)
            .await?;
    identities.extend(
        revision
            .players
            .iter()
            .map(|line| line.arma_id.trim().to_owned()),
    );
    identities.extend(
        revision
            .removed_lines
            .iter()
            .map(|line| line.arma_id.trim().to_owned()),
    );
    identities.sort_unstable();
    identities.dedup();
    let identity_refs: Vec<&str> = identities.iter().map(String::as_str).collect();
    lock_identities(&mut tx, &identity_refs).await?;

    let mut affected: Vec<String> = sqlx::query_scalar(
        "SELECT discord_id FROM users WHERE arma_id = ANY($1) AND deleted_at IS NULL",
    )
    .bind(&identity_refs)
    .fetch_all(&mut *tx)
    .await?;
    let previous: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT discord_id FROM match_player_stats
         WHERE arma_id = ANY($1) AND discord_id IS NOT NULL",
    )
    .bind(&identity_refs)
    .fetch_all(&mut *tx)
    .await?;
    affected.extend(previous);
    affected.extend(prior_match_accounts(&mut tx, stored.id).await?);
    affected.extend(obligated);
    affected.sort_unstable();
    affected.dedup();
    lock_accounts(&mut tx, &affected).await?;

    write_match_row(&mut tx, stored.id, revision).await?;
    delete_removed_lines(&mut tx, stored.id, revision).await?;
    let owners = resolve_owners(&mut tx, revision).await?;
    for (line, owner) in revision.players.iter().zip(&owners) {
        write_player_line(&mut tx, stored.id, line, owner.as_deref()).await?;
    }
    reconcile_match(&mut tx, stored.id, &affected).await?;

    let result = answer(stored.id, revision, &owners, true);
    if !result.unlinked_arma_ids.is_empty() {
        let shown = result.unlinked_arma_ids.len().min(AUDIT_UNLINKED_ID_SAMPLE);
        let mut ids = result.unlinked_arma_ids[..shown].join(", ");
        if result.unlinked_arma_ids.len() > shown {
            ids.push_str(&format!(
                ", +{} more",
                result.unlinked_arma_ids.len() - shown
            ));
        }
        append_system_audit(
            &mut tx,
            "match.unlinked_players",
            "match",
            &stored.id.to_string(),
            &format!(
                "{} of {} player line(s) had no linked account, so their stats are stored but \
                 excluded from the leaderboard and deployment counts until the identity is \
                 linked. Unlinked arma_id(s): {ids}",
                result.unlinked, result.players
            ),
        )
        .await?;
    }
    for account in &affected {
        recompute_user_stats_on_connection(&mut tx, account).await?;
    }
    refresh_leaderboard_on_connection(&mut tx).await?;
    tx.commit().await?;
    Ok(result)
}

#[cfg(test)]
#[path = "tests/match_results_ingest.rs"]
mod tests;
