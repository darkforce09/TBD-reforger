//! The personnel roster read model: one searchable page of `users` with each member's warning
//! count and deployment tally.
//!
//! **Role:** serves `GET /api/v1/admin/users?q&page&per_page` to administrators.
//! **Position:** reads `users` and `warnings`; answers
//! [`crate::models::personnel_page::PersonnelPage`] to the personnel page of the
//! single-page app.
//! **Signals & state:** none; one count and one page query per request.
//! **Invariants:** rows order by `lower(username)`, then `discord_id`, a total order, so every
//! member appears on exactly one page; `page` defaults to 1 and `per_page` to 20, a `per_page`
//! above 100 is served as 100, and a `page` or `per_page` below 1 or not a number answers 400 in
//! the `{error, details?}` envelope; a page past the end answers no items and the real total;
//! the count and the page apply the same trimmed search.

use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::response::Json;
use serde::Deserialize;
use sqlx::{Postgres, QueryBuilder};

use crate::models::personnel_page::{PersonnelPage, PersonnelRow};
use api_foundation::error_handling::api_error::ApiError;
use api_http_layer::middleware::AdminUser;
use api_state::AppState;

/// The page served when `page` is absent.
pub const DEFAULT_PAGE: i64 = 1;
/// The page size served when `per_page` is absent.
pub const DEFAULT_PER_PAGE: i64 = 20;
/// The largest page size served; a larger `per_page` is clamped to it.
pub const MAX_PER_PAGE: i64 = 100;

/// The roster's query string. A value that is not an integer rejects the whole query.
#[derive(Debug, Deserialize)]
pub struct PersonnelRosterQuery {
    /// Case-insensitive search over username, Discord handle, Arma character and Arma id.
    q: Option<String>,
    page: Option<i64>,
    per_page: Option<i64>,
}

/// The decoded query string, or the 400 a value that does not decode answers, in the
/// `{error, details?}` envelope rather than axum's plain-text rejection, through
/// [`ApiError::from_query_rejection`].
pub fn roster_query(
    query: Result<Query<PersonnelRosterQuery>, QueryRejection>,
) -> Result<PersonnelRosterQuery, ApiError> {
    query
        .map(|Query(query)| query)
        .map_err(|rejection| ApiError::from_query_rejection(rejection, "personnel roster query"))
}

/// The served `(page, per_page)` for the requested values, or the 400 a value below 1 answers.
pub fn page_window(page: Option<i64>, per_page: Option<i64>) -> Result<(i64, i64), ApiError> {
    let page = page.unwrap_or(DEFAULT_PAGE);
    let per_page = per_page.unwrap_or(DEFAULT_PER_PAGE);
    if page < 1 {
        return Err(ApiError::bad_request("page must be 1 or greater"));
    }
    if per_page < 1 {
        return Err(ApiError::bad_request("per_page must be 1 or greater"));
    }
    Ok((page, per_page.min(MAX_PER_PAGE)))
}

/// The row offset of `page`; a page too far out to address saturates, which reads past the end.
pub fn page_offset(page: i64, per_page: i64) -> i64 {
    (page - 1).saturating_mul(per_page)
}

/// `GET /api/v1/admin/users` — one page of the personnel roster with per-member warning counts.
///
/// @route GET /api/v1/admin/users
pub async fn list_users(
    State(state): State<AppState>,
    _admin: AdminUser,
    query: Result<Query<PersonnelRosterQuery>, QueryRejection>,
) -> Result<Json<PersonnelPage>, ApiError> {
    let query = roster_query(query)?;
    let (page, per_page) = page_window(query.page, query.per_page)?;
    // The trim and the emptiness test are on the same expression, and the trimmed value is what
    // `push_search` binds, so the guard and the bind cannot disagree. A whitespace-only `?q=`
    // applies no filter.
    let search = query.q.as_deref().map(str::trim).filter(|s| !s.is_empty());

    let mut count: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT count(*) FROM users WHERE true");
    if let Some(search) = search {
        push_search(&mut count, search);
    }
    let total: i64 = count.build_query_scalar().fetch_one(&state.pool).await?;

    let mut rows: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT discord_id, COALESCE(username, '') AS username, COALESCE(discord_handle, '') AS discord_handle, \
         arma_id, COALESCE(arma_character, '') AS arma_character, role, is_banned, \
         (SELECT count(*) FROM warnings w WHERE w.discord_id = users.discord_id) AS warnings, \
         total_deployments \
         FROM users WHERE true",
    );
    if let Some(search) = search {
        push_search(&mut rows, search);
    }
    rows.push(" ORDER BY lower(username) ASC, discord_id ASC LIMIT ")
        .push_bind(per_page)
        .push(" OFFSET ")
        .push_bind(page_offset(page, per_page));
    let items: Vec<PersonnelRow> = rows.build_query_as().fetch_all(&state.pool).await?;

    Ok(Json(PersonnelPage {
        items,
        page,
        per_page,
        total,
    }))
}

/// `search` as an `ILIKE` pattern matching any value that contains it literally: the pattern
/// characters `%` and `_` and the escape character `\` are escaped.
pub fn contains_pattern(search: &str) -> String {
    let mut pattern = String::with_capacity(search.len() + 2);
    pattern.push('%');
    for c in search.chars() {
        if matches!(c, '%' | '_' | '\\') {
            pattern.push('\\');
        }
        pattern.push(c);
    }
    pattern.push('%');
    pattern
}

/// Narrows `qb` to members whose username, Discord handle, Arma character or Arma id contains
/// `search`, ignoring case.
fn push_search(qb: &mut QueryBuilder<Postgres>, search: &str) {
    let like = contains_pattern(search);
    qb.push(" AND (username ILIKE ").push_bind(like.clone());
    qb.push(" OR discord_handle ILIKE ").push_bind(like.clone());
    qb.push(" OR arma_character ILIKE ").push_bind(like.clone());
    qb.push(" OR arma_id ILIKE ").push_bind(like).push(")");
}

#[cfg(test)]
#[path = "tests/personnel_roster.rs"]
mod tests;
