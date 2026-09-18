//! The Personnel Roster read model: a paginated, searchable projection of `users` with each
//! member's warning count and deployment tally.

use axum::extract::{Query, State};
use axum::response::Json;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Postgres, QueryBuilder};

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::http::pagination::PageParams;
use crate::core::middleware::AdminUser;
use crate::identity_and_access::models::user_account::UserRole;

#[derive(Debug, Serialize, sqlx::FromRow)]
struct RosterRow {
    discord_id: String,
    username: String,
    discord_handle: String,
    arma_id: Option<String>,
    arma_character: String,
    role: UserRole,
    is_banned: bool,
    warnings: i64,
    /// Denormalized column maintained by telemetry/me — projected for the personnel dossier.
    total_deployments: i64,
}

#[derive(Debug, Deserialize)]
pub struct RosterQuery {
    q: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

/// `GET /api/v1/admin/users` — Personnel Roster + per-user warning counts.
///
/// @route GET /api/v1/admin/users
pub async fn list_users(
    State(state): State<AppState>,
    _a: AdminUser,
    Query(f): Query<RosterQuery>,
) -> Result<Json<Value>, ApiError> {
    let (limit, offset) = PageParams {
        limit: f.limit,
        offset: f.offset,
    }
    .bounds();
    // The trim and the emptiness test are on the same expression, and the trimmed value is what
    // `push_search` binds, so the guard and the bind cannot disagree. Worst case for a
    // whitespace-only `?q=` is a no-op filter, not a write.
    let search = f.q.as_deref().map(str::trim).filter(|s| !s.is_empty());

    let mut cq: QueryBuilder<Postgres> = QueryBuilder::new("SELECT count(*) FROM users WHERE true");
    if let Some(s) = search {
        push_search(&mut cq, s);
    }
    let total: i64 = cq
        .build_query_scalar()
        .fetch_one(&state.pool)
        .await
        .map_err(ApiError::from)?;

    let mut sq: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT discord_id, COALESCE(username, '') AS username, COALESCE(discord_handle, '') AS discord_handle, \
         arma_id, COALESCE(arma_character, '') AS arma_character, role, is_banned, \
         (SELECT count(*) FROM warnings w WHERE w.discord_id = users.discord_id) AS warnings, \
         total_deployments \
         FROM users WHERE true",
    );
    if let Some(s) = search {
        push_search(&mut sq, s);
    }
    sq.push(" ORDER BY username ASC LIMIT ")
        .push_bind(limit)
        .push(" OFFSET ")
        .push_bind(offset);
    let rows: Vec<RosterRow> = sq
        .build_query_as()
        .fetch_all(&state.pool)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(
        json!({ "data": rows, "total": total, "limit": limit, "offset": offset }),
    ))
}

fn push_search(qb: &mut QueryBuilder<Postgres>, s: &str) {
    let like = format!("%{s}%");
    qb.push(" AND (username ILIKE ").push_bind(like.clone());
    qb.push(" OR discord_handle ILIKE ").push_bind(like.clone());
    qb.push(" OR arma_character ILIKE ").push_bind(like.clone());
    qb.push(" OR arma_id ILIKE ").push_bind(like).push(")");
}

#[cfg(test)]
#[path = "tests/personnel_roster.rs"]
mod tests;
