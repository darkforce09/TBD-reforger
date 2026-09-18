//! The ORBAT read for one event mission, and the member directory a leader seats it from.
//!
//! Both surfaces are read-only. The ORBAT groups the materialized `orbat_slots` rows by
//! `(faction, squad)` and resolves display names for occupants and reservers; the directory is
//! the offset-paginated, ban-filtered user list the assignment UI picks an assignee out of.

use std::collections::{HashMap, HashSet};

use axum::extract::{Path, Query, State};
use axum::response::Json;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Postgres, QueryBuilder};

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::http::pagination::PageParams;
use crate::core::middleware::{AuthUser, LeaderUser};
use crate::operations::models::event::{OrbatReservation, OrbatSlot};
use crate::operations::services::event_lookup::load_em;

#[derive(Debug, Serialize)]
struct OrbatSlotDto {
    id: String,
    number: i64,
    role: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    loadout: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    tag: String,
    slot_index: i64,
    assigned_to: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    assigned_name: String,
}

#[derive(Debug, Serialize)]
struct OrbatSquadDto {
    faction: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    callsign: String,
    squad: String,
    filled: i64,
    total: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    reserved_by: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    reserved_by_name: String,
    slots: Vec<OrbatSlotDto>,
}

/// `GET /api/v1/event-missions/:emid/orbat` — ORBAT grouped by squad.
///
/// @route GET /api/v1/event-missions/:emid/orbat
pub async fn get_orbat(
    State(state): State<AppState>,
    _u: AuthUser,
    Path(emid): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let em = load_em(&state.pool, &emid).await?;
    let slots: Vec<OrbatSlot> = sqlx::query_as(
        "SELECT id, event_mission_id, faction, squad, COALESCE(callsign, '') AS callsign, role, COALESCE(loadout, '') AS loadout, COALESCE(tag, '') AS tag, slot_index, assigned_to, assigned_at FROM orbat_slots WHERE event_mission_id = $1 \
         ORDER BY faction ASC, squad ASC, slot_index ASC",
    )
    .bind(em.id)
    .fetch_all(&state.pool)
    .await?;

    let reservations: Vec<OrbatReservation> =
        sqlx::query_as("SELECT * FROM orbat_reservations WHERE event_mission_id = $1")
            .bind(em.id)
            .fetch_all(&state.pool)
            .await?;
    let reserved_by: HashMap<String, String> = reservations
        .into_iter()
        .map(|r| (r.squad, r.reserved_by))
        .collect();

    // Resolve display names for assignees + reservers.
    let mut ids: HashSet<String> = HashSet::new();
    for s in &slots {
        if let Some(a) = &s.assigned_to {
            ids.insert(a.clone());
        }
    }
    for who in reserved_by.values() {
        ids.insert(who.clone());
    }
    let id_vec: Vec<String> = ids.into_iter().collect();
    let names: HashMap<String, String> =
        sqlx::query_as("SELECT discord_id, COALESCE(username, '') AS username FROM users WHERE discord_id = ANY($1)")
            .bind(&id_vec)
            .fetch_all(&state.pool)
            .await?
            .into_iter()
            .collect();

    // ══ GROUPED BY (FACTION, SQUAD), NOT SQUAD ═════════════════════════════════════════════
    // Keying on the squad name alone merges two factions that field a squad by the same name
    // into ONE card, labelled with whichever faction was seen first, holding both factions'
    // seats and running `number` 1, 2, 1, 2 — with the second faction absent from the response
    // entirely, so its seats can be neither seen nor picked. `order` dedupes on the same key
    // (the closure runs once per key), so there is no second card to render into either.
    //
    // That collision is unreachable while `idx_orbat_slot` is unique on
    // `(event_mission_id, squad, slot_index)`: attaching two same-named squads fails on
    // duplicate key first. The faction belongs in the key regardless, so that widening the
    // index to cover `faction` cannot turn a visible 500 into an invisible wrong-army bug.
    //
    // The reservation key stays the squad NAME — `orbat_reservations` has no faction column, so
    // both factions' cards correctly show the same holder. See `squad_reserved_by`.
    let mut order: Vec<(String, String)> = Vec::new();
    let mut groups: HashMap<(String, String), OrbatSquadDto> = HashMap::new();
    for s in &slots {
        let key = (s.faction.clone(), s.squad.clone());
        let g = groups.entry(key).or_insert_with(|| {
            order.push((s.faction.clone(), s.squad.clone()));
            let (rb, rbn) = match reserved_by.get(&s.squad) {
                Some(who) => (who.clone(), names.get(who).cloned().unwrap_or_default()),
                None => (String::new(), String::new()),
            };
            OrbatSquadDto {
                faction: s.faction.clone(),
                callsign: s.callsign.clone(),
                squad: s.squad.clone(),
                filled: 0,
                total: 0,
                reserved_by: rb,
                reserved_by_name: rbn,
                slots: Vec::new(),
            }
        });
        let assigned_name = s
            .assigned_to
            .as_ref()
            .and_then(|a| names.get(a).cloned())
            .unwrap_or_default();
        if s.assigned_to.is_some() {
            g.filled += 1;
        }
        g.total += 1;
        g.slots.push(OrbatSlotDto {
            id: s.id.to_string(),
            number: s.slot_index + 1,
            role: s.role.clone(),
            loadout: s.loadout.clone(),
            tag: s.tag.clone(),
            slot_index: s.slot_index,
            assigned_to: s.assigned_to.clone(),
            assigned_name,
        });
    }
    let out: Vec<OrbatSquadDto> = order
        .into_iter()
        .filter_map(|key| groups.remove(&key))
        .collect();
    Ok(Json(json!({ "data": out })))
}

#[derive(Debug, Serialize)]
struct MemberDto {
    discord_id: String,
    username: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    avatar_url: String,
}

#[derive(Debug, Deserialize)]
pub struct MemberQuery {
    q: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

/// Apply the optional `q` substring filter shared by the members COUNT + page SELECTs.
fn push_member_search_filter(qb: &mut QueryBuilder<Postgres>, search: &str) {
    let like = format!("%{search}%");
    qb.push(" AND (username ILIKE ").push_bind(like.clone());
    qb.push(" OR discord_handle ILIKE ")
        .push_bind(like)
        .push(")");
}

/// `GET /api/v1/members` — slim member directory for leaders (excludes banned).
///
/// Offset-paginated via shared [`PageParams`] bounds (default limit 20 / max 100). Response
/// shape matches other list endpoints: `{data, total, limit, offset}`.
///
/// @route GET /api/v1/members
pub async fn search_members(
    State(state): State<AppState>,
    _l: LeaderUser,
    Query(q): Query<MemberQuery>,
) -> Result<Json<Value>, ApiError> {
    let (limit, offset) = PageParams {
        limit: q.limit,
        offset: q.offset,
    }
    .bounds();
    let search = q.q.as_deref().filter(|s| !s.is_empty());

    let mut count_qb: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT count(*) FROM users WHERE is_banned = false");
    if let Some(s) = search {
        push_member_search_filter(&mut count_qb, s);
    }
    let total: i64 = count_qb
        .build_query_scalar()
        .fetch_one(&state.pool)
        .await
        .map_err(ApiError::from)?;

    // COALESCE nullable text → '' so a NULL column scans as the empty string the wire
    // contract renders for an absent value.
    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT discord_id, COALESCE(username, ''), COALESCE(avatar_url, '') \
         FROM users WHERE is_banned = false",
    );
    if let Some(s) = search {
        push_member_search_filter(&mut qb, s);
    }
    qb.push(" ORDER BY username ASC LIMIT ")
        .push_bind(limit)
        .push(" OFFSET ")
        .push_bind(offset);
    let rows: Vec<(String, String, String)> = qb
        .build_query_as()
        .fetch_all(&state.pool)
        .await
        .map_err(ApiError::from)?;
    let out: Vec<MemberDto> = rows
        .into_iter()
        .map(|(discord_id, username, avatar_url)| MemberDto {
            discord_id,
            username,
            avatar_url,
        })
        .collect();
    Ok(Json(
        json!({ "data": out, "total": total, "limit": limit, "offset": offset }),
    ))
}

#[cfg(test)]
#[path = "tests/orbat_view.rs"]
mod tests;
