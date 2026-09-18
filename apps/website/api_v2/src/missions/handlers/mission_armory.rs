//! The mission armory: the read, and the wholesale replacement write.
//!
//! The replacement is destructive by design — its transaction opens with an unconditional DELETE —
//! so the input types here require every field whose absence would turn a malformed request into
//! an accepted "clear the armory".

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::response::Json;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::{AuthUser, MissionMakerUser};
use crate::missions::models::mission::MissionArmory;
use crate::missions::services::mission_lookup::load_mission_or_404;
use crate::missions::validation::access::{can_edit, can_view};

/// `GET /api/v1/missions/:id/armory`.
///
/// @route GET /api/v1/missions/:id/armory
pub async fn get_armory(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let m = load_mission_or_404(&state.pool, &id).await?;
    if !can_view(&user, &m) {
        return Err(ApiError::not_found("mission not found"));
    }
    let items: Vec<MissionArmory> = sqlx::query_as(
        "SELECT id, mission_id, faction, category, item_name, quantity, COALESCE(icon, '') AS icon, sort_order FROM mission_armories WHERE mission_id = $1 ORDER BY sort_order ASC",
    )
    .bind(m.id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({ "data": items })))
}

/// One row of the replacement armory.
///
/// **`item_name` and `faction` are deliberately required — do not add `#[serde(default)]` to
/// either.** `category`/`icon`/`sort_order` keep their defaults on purpose: those are presentation
/// hints, matched by nothing, and an absent one degrades a row without making it a lie. A row with
/// `item_name: ""` renders in the faction dossier as a blank line the author cannot identify,
/// cannot select and cannot delete except by replacing the whole armory again — and a defaulted
/// `item_name` makes `{"items":[{}]}` a 200 that deletes every real row and inserts one nameless
/// one. That is the same mistake as `{}` one level up, so guarding only the outer field would
/// leave a trivial bypass of that guard.
///
/// `faction` is **not** a presentation hint either: it is the **join key** of the Event Hub
/// dossier. `get_event` groups the armory by it and the SPA matches those groups against the
/// mission's faction list by *exact string equality*. That list is built from a **different
/// table** — `orbat_slots.faction` — so a `faction` here that does not match one of those
/// byte-for-byte renders a dossier card with **no items at all**: against a mission whose ORBAT
/// declares `USA`, a defaulted `faction: ""` gives the author a 200 with their own value echoed
/// back and the players an empty armory. That is reachable with **no whitespace anywhere in the
/// request**, which is why requiring the field — not trimming it — is the guard.
#[derive(Debug, Deserialize)]
pub struct ArmoryItemInput {
    faction: String,
    #[serde(default)]
    category: String,
    item_name: String,
    quantity: Option<i64>,
    #[serde(default)]
    icon: String,
    #[serde(default)]
    sort_order: i64,
}

/// The armory replacement body.
///
/// **`items` is deliberately required — do not add `#[serde(default)]` to it.** A defaulted field
/// does not decode as "no data", it decodes as an affirmative *empty* value, and here that value
/// is handed to a destructive write: `DELETE FROM mission_armories WHERE mission_id = $1`, run
/// unconditionally before the inserts. Defaulted, `{}` would delete every armory row for the
/// mission, insert nothing and answer **200** — silent, total and unrecoverable, since the armory
/// is not versioned with the mission.
///
/// An empty armory is still a legitimate request; it just has to be *stated*. `{"items":[]}`
/// means "clear the armory" and still succeeds. `{}` means the caller never mentioned the
/// armory at all, and now fails to decode, which the handler maps to 400.
#[derive(Debug, Deserialize)]
pub struct SetArmoryInput {
    items: Vec<ArmoryItemInput>,
}

/// `PUT /api/v1/missions/:id/armory` — replace the armory wholesale (mission_maker+ author, or admin).
///
/// Wholesale means the first statement in the transaction is an unconditional DELETE, so every
/// way this body can be wrong is a way to lose the armory. `{"items":[]}` clears it deliberately
/// and answers 200; a missing `items`, a blank `item_name`, a missing body, the wrong
/// `Content-Type` and malformed JSON all answer 400 with the rows untouched — as do a missing,
/// blank or whitespace-padded `faction`, because it is the Event Hub's join key.
///
/// **Authz:** same tier as the metadata patch — demotion revokes armory replace.
///
/// @route PUT /api/v1/missions/:id/armory
pub async fn set_armory(
    State(state): State<AppState>,
    maker: MissionMakerUser,
    Path(id): Path<String>,
    body: Result<Json<SetArmoryInput>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let user = &maker.0;
    let m = load_mission_or_404(&state.pool, &id).await?;
    if !can_edit(user, &m) {
        return Err(ApiError::forbidden("not your mission"));
    }
    // `map_err`, not `.ok().unwrap_or_default()` — the latter collapses a missing body, a wrong
    // `Content-Type` and malformed JSON into an empty armory and writes it. `items` carrying no
    // `#[serde(default)]` above is what makes `{}` reach this arm at all.
    //
    // The message names every required field because all of them now fail here: `{}` misses
    // `items`, and `{"items":[{}]}` misses both a `faction` and an `item_name`. Naming only the
    // outer one sends the author of the second body looking for a field their request plainly has.
    let Json(input) = body.map_err(|_| {
        ApiError::bad_request("items is required, and every item needs a faction and an item_name")
    })?;
    // Validate every item BEFORE opening the transaction. The DELETE is the first statement in
    // it, so validating inside the loop would mean the armory is already gone by the time the
    // bad row is found — correct only because the transaction rolls back, and needlessly
    // load-bearing on that.
    //
    // `item_name` is a label, so a blank one is the same lie as no name and `trim` decides both
    // the rejection and the stored value; the two have to agree or `" "` is rejected while
    // `" M4 "` is stored with its padding.
    //
    // `faction` is a join key, so it is validated but **never rewritten** — see the note on
    // [`ArmoryItemInput`] and on its `bind` below.
    for (i, it) in input.items.iter().enumerate() {
        if it.item_name.trim().is_empty() {
            return Err(ApiError::bad_request(format!(
                "items[{i}].item_name is required"
            )));
        }
        if it.faction.trim().is_empty() {
            return Err(ApiError::bad_request(format!(
                "items[{i}].faction is required"
            )));
        }
        // Refused, not silently canonicalised. `"  USA  "` and `"USA"` are different factions to
        // every reader of this column, and this handler is not the one that gets to decide they
        // are the same — see the `bind` below for why trimming here would move the bug.
        if it.faction != it.faction.trim() {
            return Err(ApiError::bad_request(format!(
                "items[{i}].faction must not have leading or trailing whitespace"
            )));
        }
    }

    let mut tx = state.pool.begin().await?;
    sqlx::query("DELETE FROM mission_armories WHERE mission_id = $1")
        .bind(m.id)
        .execute(&mut *tx)
        .await?;
    for it in &input.items {
        sqlx::query(
            "INSERT INTO mission_armories (mission_id, faction, category, item_name, quantity, icon, sort_order) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(m.id)
        // **Verbatim, and it must stay verbatim.** The other side of this join,
        // `orbat_slots.faction`, is written with no normalisation at all: the event attach binds
        // `OrbatSquadTemplate.faction`, which is itself `#[serde(default)]` and untrimmed, straight
        // from the request. Trimming here would therefore make the two sites *disagree* on a padded
        // value instead of agreeing — an ORBAT declaring `"  USA  "` plus an armory row
        // `"  USA  "` renders correctly, and a unilateral trim turns that card's item count to 0.
        //
        // The guard above is agreement-preserving under *either* hypothesis about the other side:
        // only a value already equal to its trimmed form is storable, and for such a value
        // verbatim and trimmed are the same bytes. Canonicalising a padded value here is not a
        // fix, it *is* the disagreement.
        .bind(&it.faction)
        .bind(&it.category)
        .bind(it.item_name.trim())
        .bind(it.quantity)
        .bind(&it.icon)
        .bind(it.sort_order)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    let items: Vec<MissionArmory> = sqlx::query_as(
        "SELECT id, mission_id, faction, category, item_name, quantity, COALESCE(icon, '') AS icon, sort_order FROM mission_armories WHERE mission_id = $1 ORDER BY sort_order ASC",
    )
    .bind(m.id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({ "data": items })))
}
