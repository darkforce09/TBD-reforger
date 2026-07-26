//! API response DTOs (snake_case = the API contract, ported from types/api). The generic list
//! envelope + the endpoint bodies the client/pages need; each is proven byte-exact against a live
//! backend by the **R-api gate** (the `#[cfg(test)] mod r_api` at the bottom): every committed
//! golden under `tests/fixtures/api/` — captured from a running Axum stack —
//! deserializes into its DTO and re-serializes **canonically byte-equal** to the golden. A dropped,
//! renamed, or wrong-typed field breaks the equality, so drift can't ship silently.
//!
//! Strong vs envelope: `MeResponse`/`ModpackDto`/`DashboardResponse`/`LinkStatus`/`Deployments`/
//! `Leaderboard` are fully typed (every field asserted). List bodies whose *item* type isn't ported
//! yet ride `Paginated<Value>` / `DataEnvelope<Value>` — the envelope contract is proven exactly,
//! the item type gets typed + strengthened when its page lands (T-159.8+).
//!
//! **T-306 — a `Value`-typed golden is only honest while no DTO reads that endpoint.** `Value`
//! round-trips anything, so such a test asserts the envelope and *nothing at all* about the DTO it
//! exists to protect. That is not hypothetical: `/servers` was pinned as `DataEnvelope<Value>` and
//! passed for a month while `ServerStatusDto` could not deserialize the very fixture it was pinned
//! against (`server_fps: 58.7` into an `i64`), which silently dropped every live SSE telemetry frame.
//!
//! So the rule this file now follows: **if the SPA reads an endpoint through a DTO, that endpoint's
//! golden is typed with the same DTO.** Every such endpoint was enumerated rather than eyeballed
//! (T-329's lesson) by cross-referencing each `client::api_get::<T>` call against its `r_api` test —
//! six were mismatched, and typing them found two more latent drifts that no page had hit yet
//! (`EventListItem::percent` `f64`→`i64`, `OrbatSlot::assigned_to` wrongly skipped). The goldens
//! still on `Value` are the ones with **no** typed consumer — `/announcements`, `/wiki`,
//! `/vehicle-database`, `/modpacks` (list), `/admin/audit-logs` — where `Value` is the accurate
//! statement, not an escape hatch.
//!
//! Two known gaps left, both outside this slice's file ownership:
//!   * `DashboardResponse::server_status` is still `Option<Value>` — the *third* read site of the
//!     same telemetry payload. Typing it as `Option<ServerStatusDto>` is the obvious follow-up, but
//!     `dashboard.rs` reads it through `Value` helpers (T-232's `vf64`) and would need changing with
//!     it.
//!   * `/members`, `/registry/compat` and `POST /fire-missions/solve` have live typed DTOs and **no
//!     fixture at all**, so the gate cannot speak to them either way.
use crate::auth::User;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// List endpoints return `{data, total, limit, offset}` (CLAUDE.md; audit logs use a cursor).
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Paginated<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// The lighter list envelope — `{data}` only (servers, wiki, vehicle-database, modpacks list).
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct DataEnvelope<T> {
    pub data: Vec<T>,
}

/// `GET /me` → the authed user + Arma link flag.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct MeResponse {
    pub user: User,
    pub arma_linked: bool,
}

/// `GET /me/link/status` → the caller's Arma identity link state. The optionals are omitted by the
/// backend when empty, so they round-trip absent.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkStatus {
    pub linked: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arma_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arma_character: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_code: Option<bool>,
}

/// A mortar firing solution — mirrors `types/api` `FireSolution` (`POST /fire-missions/solve`).
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct FireSolution {
    pub weapon_system: String,
    pub distance_m: f64,
    pub azimuth_deg: f64,
    pub elevation_mils: i64,
    pub time_of_flight_s: f64,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// One events-list row — mirrors `types/api` `EventListItem` (`GET /events?scope=…`). T-159.25.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct EventListItem {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_override: Option<String>,
    pub start_time: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub briefing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banner_image_url: Option<String>,
    pub status: String,
    pub registration_locked: bool,
    pub max_slots: i64,
    pub mission_count: i64,
    pub registered: i64,
    pub filled: i64,
    pub total_slots: i64,
    /// T-306 sweep — `i64`, not `f64`: the backend computes `filled * 100 / total` on `i64`
    /// (`handlers/events.rs`), so the wire is always a whole number. As an `f64` this deserialized
    /// fine (serde widens an integer) but re-serialized as `55.0` where the wire says `55`, so it
    /// was latent golden drift rather than a live defect — invisible only because the `/events`
    /// golden is typed `Paginated<Value>` while the Event Manager page reads
    /// `Paginated<EventListItem>`. See the `r_api` §`Value`-typed goldens note.
    pub percent: i64,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// Live server telemetry frame — mirrors `types/models/telemetry` `ServerStatus` (SSE `data:`
/// payload + the `status` field of a server row). T-159.25.
///
/// **T-306 — `server_fps` is `f64`, and every field here was swept against
/// `api/src/models/telemetry.rs::ServerStatus` and the `server_statuses` column types. It is the
/// only field that was wrong; the other nine agree.** It had been `i64` since T-159.25 while the
/// model is `f64` over a `numeric(5,1)` column that the query casts (`server_fps::float8`), so a
/// healthy operator frame carries `58.7`. Because this struct is the *whole* SSE payload, an `i64`
/// there did not degrade one readout — it failed the entire frame's deserialization, and both read
/// sites dropped the result, so a complete healthy frame rendered as a dead server. T-232 traced
/// the same root cause to a confident `FPS: 0` on the dashboard card and fixed that one locally.
///
/// **Do not round the wire value to an integer.** `numeric(5,1)` carries a tenth on purpose and the
/// operator's frame really is `58.7`; `{}`-formatting an `f64` prints `58.7` for a fractional value
/// and `30` for a whole one, which is exactly what the React original produced from a JS number.
///
/// No `#[serde(flatten)] extra` catch-all: nothing in the SPA ever read `.extra` (it existed only
/// for forward-compat), and while it was there the newly-typed `/servers` golden would still have
/// round-tripped cleanly the day the backend grew a status field — silently re-emitting a field the
/// DTO does not model is the same "gate asserts nothing" defect as typing the fixture as `Value`.
/// Dropping it does not make the live read stricter (serde ignores unknown fields either way); it
/// makes the *gate* strict, because an unmodelled field now goes missing on re-serialize and fails
/// the byte-equality.
// `Debug` (unlike its siblings) so a rejected `SseFrame` can print what it decoded, and so a test
// failure names the frame instead of the variant.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServerStatusDto {
    pub server_id: String,
    pub is_online: bool,
    pub player_count: i64,
    pub max_players: i64,
    /// `numeric(5,1)` → `f64` (backend `ServerStatus::server_fps`). See the struct note.
    pub server_fps: f64,
    pub uptime_seconds: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_match_id: Option<String>,
    /// Backend-side this is a `String` with `skip_serializing_if = "String::is_empty"`, so `""`
    /// never reaches the wire and absent is the only "no value" encoding — `Option` round-trips it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ingame_time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ingame_weather: Option<String>,
    pub updated_at: String,
}

/// What one `\n\n`-delimited SSE frame from `/servers/:id/status/stream` turned out to be.
///
/// Lives here rather than in `sse.rs` because `sse.rs` is `#[cfg(target_arch = "wasm32")]`, so a
/// test module inside it is never compiled by `cargo test` — see that file's header note. Decoding a
/// frame *is* wire-contract work, and this is the wire-contract module, next to the captured live
/// frame the R-api gate pins it against.
// `dead_code` on the native target only: the sole non-test consumer is `sse.rs`, which `main.rs`
// gates to wasm32. Same reason every DTO in this file carries the attribute.
#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub enum SseFrame {
    /// A `data:` frame that deserialized into a telemetry status. Boxed: the variant would otherwise
    /// make the enum as large as the whole DTO.
    Status(Box<ServerStatusDto>),
    /// A `data:` frame that did **not** deserialize, carrying the serde error and the payload.
    ///
    /// T-306: the point of the variant is that it is not `None`. A bare `Option` made "the DTO
    /// cannot read this backend's frames" indistinguishable from "no frame has arrived yet", and the
    /// page rendered the second while the first was true.
    Rejected { error: String, payload: String },
    /// Not a `data:` frame — an SSE comment/keepalive (`:`), an `event:`/`id:`/`retry:` line, or the
    /// empty tail. Silence is correct here; auditing these would drown the signal that matters.
    NotData,
}

/// Decode one raw SSE frame (the text between `\n\n` boundaries) into an [`SseFrame`].
#[allow(dead_code)]
pub fn decode_server_status_frame(frame: &str) -> SseFrame {
    let Some(data) = frame.trim().strip_prefix("data:") else {
        return SseFrame::NotData;
    };
    let data = data.trim();
    match serde_json::from_str::<ServerStatusDto>(data) {
        Ok(dto) => SseFrame::Status(Box::new(dto)),
        Err(e) => SseFrame::Rejected {
            error: e.to_string(),
            payload: data.chars().take(PAYLOAD_AUDIT_CHARS).collect(),
        },
    }
}

/// How much of a rejected payload to quote. Enough to identify the offending field, short enough
/// that a warn stays readable.
pub const PAYLOAD_AUDIT_CHARS: usize = 400;

/// Report a telemetry payload the DTO refused, and return the message so a caller can also surface
/// it in the UI. Shared by the SSE loop and `server_intel`'s cached-row read.
///
/// **Deduped, because one caller is a realtime stream.** A DTO/wire mismatch does not fail one
/// frame, it fails *every* frame, so a per-frame warn would bury its own message within seconds. The
/// first occurrence of each distinct serde error warns in full; after that only the powers of ten
/// do, which still shows a broken stream is *still* broken and at what volume. Same warn-once
/// reasoning as `yrs_persist::note_orphan`, with a counter added.
///
/// Returns the message rather than logging blind so the `error` signal and the console agree —
/// `mission_hydrate::restore_snapshot`'s two-channel precedent.
pub fn audit_rejected_frame(context: &str, error: &str, payload: &str) -> String {
    use std::cell::RefCell;
    use std::collections::HashMap;
    thread_local! {
        static SEEN: RefCell<HashMap<String, u64>> = RefCell::new(HashMap::new());
    }
    let n = SEEN.with(|s| {
        let mut s = s.borrow_mut();
        let c = s.entry(error.to_string()).or_insert(0);
        *c += 1;
        *c
    });
    let msg = format!(
        "[t306] {context}: telemetry frame REJECTED and dropped — {error}. The stream is connected \
         and the payload arrived intact, so this is a DTO/wire contract mismatch, not a network \
         fault: dto.rs ServerStatusDto disagrees with api/src/models/telemetry.rs ServerStatus. \
         {n} dropped so far with this error. Payload: {payload}"
    );
    if n == 1 || is_power_of_ten(n) {
        #[cfg(target_arch = "wasm32")]
        web_sys::console::warn_1(&wasm_bindgen::JsValue::from_str(&msg));
        #[cfg(not(target_arch = "wasm32"))]
        eprintln!("{msg}");
    }
    msg
}

/// Is `n` one of 10, 100, 1000, …? The warn ladder above — **not** `u64::is_power_of_two`. `1` is
/// excluded on purpose: the first drop is already covered by the `n == 1` arm, and counting it here
/// too would warn twice on frame one.
fn is_power_of_ten(n: u64) -> bool {
    let mut p = 10u64;
    loop {
        if p == n {
            return true;
        }
        if p > n {
            return false;
        }
        match p.checked_mul(10) {
            Some(next) => p = next,
            None => return false,
        }
    }
}

/// One `GET /servers` row — the backend `handlers::servers::ServerIntelDto`: a flattened `Server`
/// plus its live `status` and required modpack.
///
/// **`status` is deliberately NOT `skip_serializing_if`.** The backend field is a plain
/// `Option<ServerStatus>`, so a server with no telemetry row serializes as an explicit
/// `"status": null` — which the third row of the committed golden carries. Omitting it here would
/// break the canonical byte-equality. `required_modpack` *is* skipped, matching the backend.
///
/// No `#[serde(flatten)] extra` catch-all, on purpose: a catch-all re-emits fields the struct does
/// not know about, so the round-trip would still pass the day the backend grows a field — the exact
/// "asserts nothing" failure T-306 was filed for. Without one, an added field is dropped on
/// deserialize, goes missing on re-serialize, and the golden gate fails loudly.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerRowDto {
    pub id: String,
    pub name: String,
    /// Postgres `inet`, served as text (`host(ip)`).
    pub ip: String,
    pub port: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_modpack_id: Option<String>,
    pub is_active: bool,
    pub status: Option<ServerStatusDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_modpack: Option<ModpackDto>,
}

/// One approvals-queue row — mirrors `types/api` `ApprovalRow` (`GET /approvals`).
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovalRow {
    pub mission_id: String,
    pub title: String,
    pub terrain: String,
    pub author_id: String,
    pub author_name: String,
    pub submitted_at: String,
}

/// One mission library card — mirrors `types/api` `MissionCard` (`GET /missions?scope=…`).
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct MissionCard {
    pub id: String,
    pub title: String,
    pub author_id: String,
    pub terrain: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_terrain_name: Option<String>,
    pub game_mode: String,
    pub weather: String,
    pub time_of_day: String,
    pub max_players: i64,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub briefing: Option<String>,
    pub author_name: String,
    pub author_avatar: String,
    /// **T-389 — the review stamp, promoted out of `extra` because the page now reads it.**
    /// `GET /missions` has always carried these three (`handlers/missions.rs` `MISSION_COLS`
    /// selects all of them), so they were silently absorbed by the `extra` catch-all below and
    /// round-tripped without any DTO naming them. The library card renders the rejection reason on
    /// the author's own returned mission, so they are named fields now — `extra` proving the *wire*
    /// is not the same as a DTO the *page* can read (T-306).
    ///
    /// **Why the golden had to grow rather than just the struct (T-359).** All three are
    /// `skip_serializing_if` on the backend (`models/mission.rs:108-113`), so on a never-reviewed
    /// mission they are absent from the wire entirely. An `Option` + `skip_serializing_if` here
    /// round-trips absent → `None` → absent, which means the R-api gate stays **green over a field
    /// the backend never sent** — the exact hazard T-359 found. The pre-existing goldens are a
    /// `draft` detail and a list whose four rows carry no `rejection_reason` at all, so they cannot
    /// speak to the present case. `GET__missions__scope-mine-rejected.json` +
    /// `GET__missions__82b937fc-c88e-4bb9-abb3-0bef67379398.json` were captured off a genuinely
    /// rejected mission specifically so every field below is non-absent in at least one golden.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_at: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// One ORBAT slot row — mirrors `types/api` `OrbatSlot` (backend `orbatSquadDTO` slots).
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct OrbatSlot {
    pub id: String,
    pub number: i64,
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loadout: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    pub slot_index: i64,
    /// **T-306 — `default` but deliberately NOT `skip_serializing_if`.** The backend
    /// `handlers::events::OrbatSlotDto::assigned_to` is a bare `Option<String>` with no
    /// `skip_serializing_if`, so an unclaimed slot serializes as an explicit `"assigned_to": null` —
    /// which the committed orbat golden carries on every unclaimed slot. Skipping it emitted a
    /// payload with the key missing, and the round-trip drifted.
    ///
    /// Found by typing the orbat golden: it was `DataEnvelope<Value>`, so nothing checked, even
    /// though the ORBAT selector reads `DataEnvelope<OrbatSquad>` live. Same shape as the `/servers`
    /// hole this ticket was filed for; contrast `assigned_name`, which the backend *does* skip
    /// (`skip_serializing_if = "String::is_empty"`) and which is therefore correct as-is.
    #[serde(default)]
    pub assigned_to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned_name: Option<String>,
}

/// A squad grouping of ORBAT slots — mirrors `types/api` `OrbatSquad`. `GET
/// /event-missions/:emid/orbat` returns `{data: OrbatSquad[]}` (T-159.25 selector).
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct OrbatSquad {
    pub faction: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub callsign: Option<String>,
    pub squad: String,
    pub filled: i64,
    pub total: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reserved_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reserved_by_name: Option<String>,
    pub slots: Vec<OrbatSlot>,
}

/// A slim member row for the leader's assignee picker — `GET /members?q=` `{data: Member[]}`.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Member {
    pub discord_id: String,
    pub username: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
}

/// `POST /me/link` → a freshly minted one-time Arma link code (T-159.25 Settings mutations).
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkCodeResponse {
    pub code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// A modpack row — backend `models::content::Modpack`. `workshop_url` is omitted when empty.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Modpack {
    pub id: String,
    pub name: String,
    pub version: String,
    pub total_size_bytes: i64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub workshop_url: String,
    pub is_current: bool,
    pub created_at: String,
}

/// A modpack with its mod list embedded (backend `ModpackDto`: `#[serde(flatten)]` modpack + mods).
/// `mods` items are typed when the Modpacks page lands (T-159.12); the flatten + envelope is proven.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ModpackDto {
    #[serde(flatten)]
    pub modpack: Modpack,
    pub mods: Vec<Value>,
}

/// `GET /dashboard` — the landing aggregate. Every field is always present (nulls stay null, not
/// omitted), so none is `skip_serializing_if`. The three still-untyped nested bodies ride `Value`
/// until their pages land (events / assignment / server-status); `current_modpack` is fully typed.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct DashboardResponse {
    pub next_event: Option<Value>,
    pub my_assignment: Option<Value>,
    pub server_status: Option<Value>,
    pub current_modpack: Option<ModpackDto>,
    pub recent_announcements: Vec<Value>,
}

/// `GET /me/deployments` — the caller's service record.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Deployments {
    pub total_operations: i64,
    pub attendance_rate: f64,
    pub service_history: Vec<Value>,
    pub upcoming: Vec<Value>,
}

/// `GET /leaderboards` — `{category, data}` (NOT the paginated envelope).
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Leaderboard {
    pub category: String,
    pub data: Vec<Value>,
}

/// One Virtual Arsenal catalog item, identified by its full Enfusion `resource_name`. Mirrors the TS
/// oracle `types/models/registry.ts` `RegistryItem` (backend `models::RegistryItem`, contract
/// `registry-items.schema.json#/$defs/item`) field-for-field.
///
/// **Every optional is `skip_serializing_if`** — the backend `omitempty`s them, so the committed
/// golden's rows carry exactly the 9 required fields. Serializing an absent optional as `null` would
/// add a key the golden lacks and break the R-api canonical byte-equality (the `LinkStatus` /
/// `MissionDetail` precedent).
///
/// `kind` is a **`String`, not an enum**: the vocabulary is versioned and growing (the TS type is on
/// its "T-068.10.2 v3" revision), and an enum would hard-fail deserialization the day the backend
/// adds a kind — where a string degrades to "not a `character`", i.e. not placeable.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryItem {
    pub id: String,
    pub modpack_id: String,
    pub resource_name: String,
    pub display_name: String,
    /// A slash path (`"NATO/US_Army/Rifleman"`) — the palette's folder tree, see `asset_catalog`.
    pub category: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    pub kind: String,
    /// Non-placeable template prefab (`*_base.et`). `abstract` is a reserved Rust word.
    #[serde(rename = "abstract", default, skip_serializing_if = "Option::is_none")]
    pub r#abstract: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arsenal_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight_kg: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume_cm3: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_weight_kg: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_volume_cm3: Option<f64>,
    /// Inventory UI grid width in cells (T-068.15.1 capacity export; absent = no readable capacity).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cargo_grid_w: Option<i64>,
    /// Inventory UI grid height in cells (T-068.15.1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cargo_grid_h: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub addon: Option<String>,
    /// Factory attachment/camo configuration of a base weapon (T-068.10.5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant_of: Option<String>,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// `GET /registry` — the asset catalog + its cache identity (weak ETag). Items typed at T-159.22, so
/// `registry_envelope()` now proves the row field-set too, not just the envelope.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryResponse {
    pub data: Vec<RegistryItem>,
    pub etag: String,
    pub modpack_id: String,
    pub modpack_version: String,
}

/// One compat edge — a generic `(from_node, to_node, edge_type)` graph row. Optic/magazine
/// compatibility is expressed as `edge_type` values (`optic_on_weapon`, `mag_in_weapon`), not typed
/// fields, so new families need no DTO change (T-167 / backend `models::registry::RegistryCompatEdge`).
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryCompatEdge {
    pub id: String,
    pub modpack_id: String,
    pub from_node: String,
    pub to_node: String,
    pub edge_type: String,
    #[serde(default)]
    pub evidence: String,
    /// Edge multiplicity (T-068.15.1): duplicate `character_default_cargo` scanner
    /// emissions aggregate here; 1 for every other family (and for pre-.15.1 payloads).
    #[serde(default = "default_edge_qty")]
    pub qty: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// Serde default for [`RegistryCompatEdge::qty`] — an edge without the field counts once.
fn default_edge_qty() -> i64 {
    1
}

/// `GET /registry/compat` — the compat edge list + cache identity (mirrors `RegistryResponse`).
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryCompatResponse {
    pub data: Vec<RegistryCompatEdge>,
    pub etag: String,
    pub modpack_id: String,
    pub modpack_version: String,
}

/// One role template inside a faction doc (character + optional loadout).
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FactionRole {
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    pub character: String,
    /// A `SlotLoadoutV2` object (opaque here — the same shape `arsenal.rs` writes).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loadout: Option<Value>,
}

/// One vehicle in a faction's pool.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FactionVehicle {
    pub vehicle: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// The full faction-library document (`faction-library.schema.json`). POST/PUT body.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FactionDoc {
    pub side: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emblem: Option<String>,
    #[serde(default)]
    pub roles: Vec<FactionRole>,
    #[serde(default)]
    pub vehicles: Vec<FactionVehicle>,
}

/// One stored faction (`side`/`name` are projections of `doc`). GET/POST/PUT response.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct UserFaction {
    pub id: String,
    pub owner_id: String,
    pub side: String,
    pub name: String,
    pub doc: FactionDoc,
    pub created_at: String,
    pub updated_at: String,
}

/// `GET /factions` — the caller's faction library.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct FactionListResponse {
    pub data: Vec<UserFaction>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// The four canonical faction sides.
pub const FACTION_SIDES: &[&str] = &["BLUFOR", "OPFOR", "INDFOR", "CIV"];

/// Cursor-paginated list — `{data, next_cursor}` (audit logs). Item type ported per page.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct CursorList<T> {
    pub data: Vec<T>,
    pub next_cursor: Option<Value>,
}

/// `GET /admin/users` row — backend `handlers::admin::RosterRow` (a reduced projection, not `User`).
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct AdminUserRow {
    pub discord_id: String,
    pub username: String,
    pub discord_handle: String,
    #[serde(default)]
    pub arma_id: Option<String>,
    pub arma_character: String,
    pub role: crate::nav::Role,
    pub is_banned: bool,
    pub warnings: i64,
}

/// The mission version embedded in `GET /missions/:id` (`current_version`). `json_payload` is the
/// editor superset — kept as an opaque `Value` (rendered pages read only `semver`).
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct MissionVersionRef {
    pub created_at: String,
    pub created_by: String,
    pub id: String,
    pub json_payload: Value,
    pub mission_id: String,
    pub semver: String,
}

/// `GET /missions/:id` → the full Mission Overview (backend `missionDetail`): the card fields + the
/// current version + armory. Optionals the backend omits when empty round-trip absent
/// (skip_serializing_if) so the R-api gate stays byte-exact.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct MissionDetail {
    pub armory: Vec<Value>,
    pub author_avatar: String,
    pub author_id: String,
    pub author_name: String,
    pub bookmarked: bool,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_version: Option<MissionVersionRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_version_id: Option<String>,
    pub game_mode: String,
    pub id: String,
    pub max_players: i64,
    pub status: String,
    pub terrain: String,
    pub time_of_day: String,
    pub title: String,
    pub updated_at: String,
    pub weather: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub briefing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_terrain_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
    /// **T-389 — the review stamp.** `rejection_reason` is the *only* thing an author is ever told
    /// about why their mission came back (`handlers/approvals.rs:217` is its sole writer), and until
    /// this slice no DTO named it, so the dossier could not render it. Unlike `MissionCard` this
    /// struct has **no `extra` catch-all** — T-306 removed it precisely so a field the backend does
    /// not send fails the gate instead of rendering a placeholder — which means adding these three
    /// is a real shape assertion, not a widening.
    ///
    /// See the `MissionCard` counterpart for why a *rejected* golden had to be captured alongside
    /// the change: absent → `None` → absent round-trips green and proves nothing (T-359).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_at: Option<String>,
}

/// One armory row inside `armory_by_faction[].items[]` (T-159.25 faction dossiers). The flattened
/// `extra` map preserves any wire fields beyond the rendered three, so the R-api canonical
/// round-trip stays byte-exact whatever the backend adds.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmoryItem {
    pub id: String,
    pub item_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<i64>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// One faction's armory group in a mission dossier (`armory_by_faction[]`).
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmoryFaction {
    pub faction: String,
    pub items: Vec<ArmoryItem>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// One mission dossier nested in `GET /events/:id` (`missions[]`). Optionals the backend omits
/// (briefing/thumbnail/my_state/my_slot_id) round-trip absent.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct EventMissionDossier {
    pub armory_by_faction: Vec<ArmoryFaction>,
    pub event_mission_id: String,
    pub factions: Vec<String>,
    pub filled: i64,
    pub game_mode: String,
    pub mission_id: String,
    pub start_time: String,
    pub terrain: String,
    pub title: String,
    pub total: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub briefing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub my_state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub my_slot_id: Option<String>,
}

/// `GET /events/:id` → the Event Hub (backend `eventHub`): the event container + nested mission
/// dossiers. `created_at`/`created_by`/`updated_at` are on the wire (not in the hand TS type) so they
/// must be modeled for the R-api round-trip; the empty optionals round-trip absent.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct EventHub {
    pub created_at: String,
    pub created_by: String,
    pub id: String,
    pub max_slots: i64,
    pub missions: Vec<EventMissionDossier>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_override: Option<String>,
    pub registration_locked: bool,
    pub start_time: String,
    pub status: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub briefing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banner_image_url: Option<String>,
}

/// The doc's terrain + environment fields, for the Mission Settings dialog. Pure data (no wasm
/// deps), so it lives here in the always-compiled DTO module: the wasm `editor_ops::read_env`
/// returns it, and the native `eden_chrome` view-shell fallback (`::default()`) needs it too.
#[derive(Clone, Debug, PartialEq)]
pub struct MissionEnv {
    pub terrain: String,
    pub time: String,
    pub weather: String,
    pub view_distance: i64,
    pub thermals: bool,
    // T-173 P6 — render prefs restored from the React Mission Settings (per-mission, in
    // `meta.environment`; the per-user basemap view + world-layer toggles live in localStorage —
    // see `world_layer_prefs`). Defaults mirror the React `useDemLayer` OPACITY=0.4 + grid on.
    pub show_hillshade: bool,
    pub hillshade_opacity: f64,
    pub show_grid: bool,
}

impl Default for MissionEnv {
    fn default() -> Self {
        Self {
            terrain: String::new(),
            time: String::new(),
            weather: String::new(),
            view_distance: 0,
            thermals: false,
            show_hillshade: true,
            hillshade_opacity: 0.4,
            show_grid: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nav::Role;

    #[test]
    fn paginated_shape() {
        let p: Paginated<i64> =
            serde_json::from_str(r#"{"data":[1,2,3],"total":3,"limit":20,"offset":0}"#).unwrap();
        assert_eq!(p.data, vec![1, 2, 3]);
        assert_eq!((p.total, p.limit, p.offset), (3, 20, 0));
    }

    #[test]
    fn link_status_optionals() {
        let full: LinkStatus = serde_json::from_str(
            r#"{"linked":true,"arma_id":"a","arma_character":"Cpl","pending_code":true}"#,
        )
        .unwrap();
        assert!(
            full.linked && full.pending_code == Some(true) && full.arma_id.as_deref() == Some("a")
        );
        // The minimal shape (backend drops the empties)…
        let min: LinkStatus = serde_json::from_str(r#"{"linked":false}"#).unwrap();
        assert!(!min.linked && min.arma_id.is_none() && min.pending_code.is_none());
        // …and it re-serializes absent (skip_serializing_if), so it round-trips exactly.
        assert_eq!(serde_json::to_string(&min).unwrap(), r#"{"linked":false}"#);
    }

    #[test]
    fn me_response_round_trips() {
        let json = r#"{"user":{"discord_id":"1","username":"u","discord_handle":"u#1","avatar_url":"","arma_id":null,"arma_character":"","role":"enlisted","is_banned":false,"total_deployments":0,"attendance_rate":0.0,"created_at":"t","updated_at":"t"},"arma_linked":true}"#;
        let me: MeResponse = serde_json::from_str(json).unwrap();
        assert!(me.arma_linked && me.user.role == Role::Enlisted);
        let back: MeResponse = serde_json::from_str(&serde_json::to_string(&me).unwrap()).unwrap();
        assert!(back == me, "MeResponse re-serialize → reparse is stable");
    }
}

/* ══════════════════════════════ R-api gate ══════════════════════════════ */
// Each committed golden (captured from a running Axum `:8080` via dev-login — see the fixture dir's
// _index.tsv) must round-trip through its DTO **canonically byte-equal**. `canon` sorts object keys
// recursively (order-independent, works with or without serde_json's preserve_order feature) and
// normalizes whitespace/number-repr on BOTH sides equally, so the assertion isolates exactly one
// thing: does the DTO's serialized field-set + values match the live backend's? Any drop / rename /
// type change fails it. This is the load-bearing R-api proof (stronger than a browser round-trip:
// deterministic, no network, compile-time-pinned goldens).
// `pub(crate)` so `sse.rs`'s own tests can drive the one captured live SSE frame
// (`LIVE_SSE_FRAME`) through the real decoder instead of keeping a second, drifting copy of it.
#[cfg(test)]
pub(crate) mod r_api {
    use super::*;
    use serde::de::DeserializeOwned;

    /// Recursively key-sort + renormalize a JSON string to a canonical form.
    fn canon(s: &str) -> String {
        fn sort(v: Value) -> Value {
            match v {
                Value::Object(m) => {
                    let mut keys: Vec<String> = m.keys().cloned().collect();
                    keys.sort();
                    let mut out = serde_json::Map::new();
                    for k in keys {
                        let child = m.get(&k).cloned().unwrap();
                        out.insert(k, sort(child));
                    }
                    Value::Object(out)
                }
                Value::Array(a) => Value::Array(a.into_iter().map(sort).collect()),
                other => other,
            }
        }
        let v: Value = serde_json::from_str(s).expect("golden is valid JSON");
        serde_json::to_string(&sort(v)).unwrap()
    }

    /// The gate: `golden` must deserialize into `T` and re-serialize canonical-equal to `golden`.
    fn assert_golden<T: Serialize + DeserializeOwned>(golden: &str) {
        let dto: T = serde_json::from_str(golden)
            .unwrap_or_else(|e| panic!("R-api: golden does not deserialize into the DTO: {e}"));
        let back = serde_json::to_string(&dto).expect("DTO re-serializes");
        assert_eq!(
            canon(golden),
            canon(&back),
            "R-api: DTO must re-serialize canonically byte-equal to the live-backend golden"
        );
    }

    // Goldens are compile-time-embedded from the crate-local fixture corpus
    // (apps/website/frontend/tests/fixtures/api/ — T-171 fixture convention).
    const FX: &str = "../tests/fixtures/api/";
    macro_rules! golden {
        ($f:literal) => {
            include_str!(concat!("../tests/fixtures/api/", $f))
        };
    }

    // ── strong-typed bodies (every field asserted) ──
    #[test]
    fn me() {
        assert_golden::<MeResponse>(golden!("GET__me.json"));
    }
    #[test]
    fn modpack_current() {
        assert_golden::<ModpackDto>(golden!("GET__modpacks__current.json"));
    }
    #[test]
    fn dashboard() {
        assert_golden::<DashboardResponse>(golden!("GET__dashboard.json"));
    }
    #[test]
    fn link_status() {
        assert_golden::<LinkStatus>(golden!("GET__me__link__status.json"));
    }
    #[test]
    fn deployments() {
        assert_golden::<Deployments>(golden!("GET__me__deployments.json"));
    }
    #[test]
    fn leaderboards() {
        assert_golden::<Leaderboard>(golden!("GET__leaderboards.json"));
    }
    #[test]
    fn registry_envelope() {
        assert_golden::<RegistryResponse>(golden!("GET__registry.json"));
    }
    #[test]
    fn mission_detail() {
        assert_golden::<MissionDetail>(golden!(
            "GET__missions__512d8658-7025-4a70-94e9-a1b44a7aa155.json"
        ));
    }
    /// T-389 — the golden above is a `draft`, so all three review-stamp fields are **absent** from
    /// it. Against `Option` + `skip_serializing_if` that round-trips absent → `None` → absent and
    /// asserts nothing at all about `rejection_reason` / `reviewed_by` / `reviewed_at` — T-359's
    /// hazard exactly. This golden was captured off a mission driven through the real
    /// submit → reject path, so every one of the three is **present and non-empty** on the wire and
    /// the round-trip has something to be wrong about. The two tests are a matched pair: absent case
    /// above, present case here.
    #[test]
    fn mission_detail_rejected_carries_the_review_stamp() {
        const G: &str = golden!("GET__missions__82b937fc-c88e-4bb9-abb3-0bef67379398.json");
        assert_golden::<MissionDetail>(G);
        // Belt-and-braces on the round-trip: assert the golden really is the present case, so this
        // test cannot quietly decay into a second copy of the absent one if the fixture is
        // recaptured off a draft.
        let d: MissionDetail = serde_json::from_str(G).unwrap();
        assert_eq!(d.status, "rejected");
        assert!(
            d.rejection_reason.as_deref().is_some_and(|r| !r.is_empty()),
            "the rejected golden must carry a non-empty rejection_reason"
        );
        assert!(d.reviewed_by.is_some(), "and the reviewer");
        assert!(d.reviewed_at.is_some(), "and the review timestamp");
    }
    #[test]
    fn event_hub() {
        assert_golden::<EventHub>(golden!(
            "GET__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7.json"
        ));
    }
    /// T-306 — was `DataEnvelope<Value>` while the ORBAT selector reads `DataEnvelope<OrbatSquad>`
    /// live. Typing it immediately failed and found `OrbatSlot::assigned_to` skipping a key the
    /// backend emits as an explicit `null`.
    #[test]
    fn orbat_envelope() {
        assert_golden::<DataEnvelope<OrbatSquad>>(golden!(
            "GET__event-missions__89b1b731-37a8-4926-901a-3c7ff7de5eb3__orbat.json"
        ));
    }

    // ── paginated `{data,total,limit,offset}` envelopes (item type ported per page) ──
    /// T-306 — was `Paginated<Value>` while `event_manager` reads `Paginated<EventListItem>`. Typing
    /// it pinned `percent` as the `i64` the backend actually sends (the golden's `37` re-serialized
    /// as `37.0` from the old `f64`).
    #[test]
    fn events_envelope() {
        assert_golden::<Paginated<EventListItem>>(golden!("GET__events.json"));
    }
    /// T-306 — was `Paginated<Value>` while the Mission Library reads `Paginated<MissionCard>`.
    #[test]
    fn missions_envelope() {
        assert_golden::<Paginated<MissionCard>>(golden!("GET__missions.json"));
    }
    /// T-389 — the `/missions` golden above has three `live` rows and one `draft`, so it pins
    /// `reviewed_by`/`reviewed_at` (the live rows were approved) but **no row carries a
    /// `rejection_reason`**, which is the one field the library card now renders. Captured from
    /// `GET /missions?scope=mine` as the author after a real rejection.
    ///
    /// Naming: `fixture_for()` in `vsuite.rs` strips the query string, so `/api/v1/missions?scope=…`
    /// resolves to `GET__missions.json` and this file can never shadow it during a DOM capture — it
    /// is read by this test alone, deliberately, so recapturing it cannot move any oracle freeze.
    #[test]
    fn missions_envelope_rejected_card_carries_its_reason() {
        const G: &str = golden!("GET__missions__scope-mine-rejected.json");
        assert_golden::<Paginated<MissionCard>>(G);
        let page: Paginated<MissionCard> = serde_json::from_str(G).unwrap();
        let rejected: Vec<&MissionCard> = page
            .data
            .iter()
            .filter(|m| m.status == "rejected")
            .collect();
        assert!(
            !rejected.is_empty(),
            "this golden exists to cover the rejected card; recapture it off a rejected mission"
        );
        for m in rejected {
            assert!(
                m.rejection_reason.as_deref().is_some_and(|r| !r.is_empty()),
                "a rejected card must carry the reason the page renders"
            );
            // The field must be a NAMED field, not swept into `extra` — that is the difference
            // between proving the wire and giving the page something it can read (T-306).
            assert!(
                !m.extra.contains_key("rejection_reason"),
                "rejection_reason must be a named field, not absorbed by the `extra` catch-all"
            );
        }
    }
    /// Still `Value`, and that is the honest statement: no DTO reads `/announcements` — the page
    /// itself takes `Paginated<Value>`. Type this the day an `AnnouncementDto` lands.
    #[test]
    fn announcements_envelope() {
        assert_golden::<Paginated<Value>>(golden!("GET__announcements.json"));
    }
    /// T-306 — was `Paginated<Value>` while the approvals queue reads `Paginated<ApprovalRow>`.
    #[test]
    fn approvals_envelope() {
        assert_golden::<Paginated<ApprovalRow>>(golden!("GET__approvals.json"));
    }
    /// T-306 — was `Paginated<Value>`; the faction manager reads `FactionListResponse`, which is not
    /// even the same envelope shape the test was asserting.
    #[test]
    fn factions_envelope() {
        assert_golden::<FactionListResponse>(golden!("GET__factions.json"));
    }
    #[test]
    fn admin_users_envelope() {
        assert_golden::<Paginated<AdminUserRow>>(golden!("GET__admin__users.json"));
    }
    /// Cursor envelope, not offset/total — and `Value` is honest here: the audit page reads
    /// `CursorList<Value>` too.
    #[test]
    fn audit_logs_envelope() {
        assert_golden::<CursorList<Value>>(golden!("GET__admin__audit-logs.json"));
    }

    // ── `{data}` envelopes ──
    /// T-306 — this used to be `DataEnvelope<Value>`, and that is why a month-old wire/DTO type
    /// mismatch shipped: `Value` round-trips *any* payload, so the gate passed while the real
    /// `ServerStatusDto` could not deserialize the very golden it was pinned against. Typed, this
    /// test fails on `server_fps: 58.7` against an `i64` field with
    /// `invalid type: floating point 58.7, expected i64` — the defect, caught by the gate that
    /// exists to catch it.
    #[test]
    fn servers_envelope() {
        assert_golden::<DataEnvelope<ServerRowDto>>(golden!("GET__servers.json"));
    }

    /// One **live** `GET /servers/:id/status/stream` frame, captured byte-exact off a running Axum
    /// stack (`curl -sN .../status/stream`) whose `server_statuses` row reproduces the
    /// `GET__servers.json` golden. Includes the `data: ` prefix and the `\n\n` terminator the
    /// `sse.rs` splitter keys on, so the fixture is the wire and not a paraphrase of it.
    ///
    /// The SSE payload had **no golden of any kind** before T-306 — the fixture corpus is all `GET`
    /// bodies — so nothing pinned the one DTO that a realtime consumer deserializes on every frame.
    pub(crate) const LIVE_SSE_FRAME: &str = concat!(
        r#"data: {"server_id":"00000000-0000-4000-d000-000000000001","is_online":true,"#,
        r#""player_count":47,"max_players":64,"server_fps":58.7,"uptime_seconds":19842,"#,
        r#""current_match_id":"00000000-0000-4000-f000-000000000003","ingame_time":"06:42","#,
        r#""ingame_weather":"overcast","updated_at":"2026-07-26T05:00:00Z"}"#,
        "\n\n"
    );

    /// The captured live frame must deserialize, and must carry the tenth the `numeric(5,1)`
    /// column really holds — rounding it away would be a second, quieter version of this bug.
    #[test]
    fn live_sse_frame_deserializes_with_its_fractional_fps() {
        let payload = LIVE_SSE_FRAME
            .trim()
            .strip_prefix("data:")
            .expect("captured frame is a data: frame")
            .trim();
        let dto: ServerStatusDto = serde_json::from_str(payload)
            .unwrap_or_else(|e| panic!("R-api: live SSE frame does not deserialize: {e}"));
        assert_eq!(dto.server_fps, 58.7, "the wire tenth must survive the DTO");
        assert_eq!(dto.player_count, 47);
        assert_eq!(dto.max_players, 64);
        assert_eq!(dto.uptime_seconds, 19842);
        assert_eq!(dto.ingame_time.as_deref(), Some("06:42"));
        assert_eq!(dto.ingame_weather.as_deref(), Some("overcast"));
        // The frame is also a golden: it must re-serialize canonically byte-equal.
        assert_eq!(canon(payload), canon(&serde_json::to_string(&dto).unwrap()));
    }
    #[test]
    fn wiki_envelope() {
        assert_golden::<DataEnvelope<Value>>(golden!("GET__wiki.json"));
    }
    #[test]
    fn vehicle_db_envelope() {
        assert_golden::<DataEnvelope<Value>>(golden!("GET__vehicle-database.json"));
    }
    #[test]
    fn modpacks_list_envelope() {
        assert_golden::<DataEnvelope<Value>>(golden!("GET__modpacks.json"));
    }

    // Guard: the fixture dir constant + macro base agree (a rename would break include_str! anyway,
    // but this keeps the human-visible path honest).
    #[test]
    fn fixture_dir_constant_documented() {
        assert!(FX.ends_with("fixtures/api/"));
    }

    // ── SSE frame decode (T-306) ──
    //
    // These live beside the R-api gate because the module that *uses* them (`sse.rs`) is
    // wasm32-only and therefore untestable by `cargo test` — the reason the decode moved here.

    /// The captured live frame through the real decoder. Before T-306 this returned `Rejected` —
    /// every live telemetry frame did — and the read loop dropped it without a word.
    #[test]
    fn a_live_frame_decodes_into_a_status() {
        match decode_server_status_frame(LIVE_SSE_FRAME) {
            SseFrame::Status(dto) => {
                assert_eq!(dto.server_fps, 58.7);
                assert_eq!(dto.player_count, 47);
                assert_eq!(dto.max_players, 64);
                assert!(dto.is_online);
            }
            other => panic!("live frame must decode into a status, got {other:?}"),
        }
    }

    /// The `i64` regression, pinned: a fractional `server_fps` must never be why a frame is dropped.
    #[test]
    fn a_fractional_fps_is_not_a_reason_to_reject_a_frame() {
        for fps in ["58.7", "0.0", "29.4", "60", "100.0", "19.9"] {
            let frame = format!(
                "data: {{\"server_id\":\"s\",\"is_online\":true,\"player_count\":1,\
                 \"max_players\":2,\"server_fps\":{fps},\"uptime_seconds\":3,\
                 \"updated_at\":\"t\"}}\n\n"
            );
            assert!(
                matches!(decode_server_status_frame(&frame), SseFrame::Status(_)),
                "server_fps={fps} must decode"
            );
        }
    }

    /// A malformed payload must come back carrying its reason. A bare `None` here is exactly what
    /// made this class of defect invisible.
    #[test]
    fn a_bad_payload_is_rejected_with_its_reason_not_silently_dropped() {
        match decode_server_status_frame("data: {\"server_id\":\"s\",\"is_online\":\"yes\"}\n\n") {
            SseFrame::Rejected { error, payload } => {
                assert!(error.contains("invalid type"), "unexpected error: {error}");
                assert!(payload.contains("server_id"), "payload must be reported");
            }
            other => panic!("expected Rejected, got {other:?}"),
        }
    }

    /// Keepalives and non-`data:` lines are NOT rejections — auditing them would drown the real
    /// signal, which is the failure mode the audit exists to avoid.
    #[test]
    fn non_data_frames_are_not_audited_as_rejections() {
        for f in [": keepalive\n\n", "event: ping\n\n", "\n\n", "id: 7\n\n"] {
            assert_eq!(
                decode_server_status_frame(f),
                SseFrame::NotData,
                "frame {f:?}"
            );
        }
    }

    #[test]
    fn the_warn_ladder_is_first_then_powers_of_ten() {
        for n in [10u64, 100, 1000, 10_000] {
            assert!(is_power_of_ten(n), "{n} should be on the ladder");
        }
        for n in [0u64, 1, 2, 9, 11, 99, 101, 1001] {
            assert!(!is_power_of_ten(n), "{n} should not be on the ladder");
        }
    }

    /// The audit returns a message that names the field and the two structs to reconcile — a warn
    /// that just said "parse failed" would have cost T-306 the same month.
    #[test]
    fn the_audit_message_names_the_offending_field_and_both_structs() {
        let SseFrame::Rejected { error, payload } = decode_server_status_frame(
            "data: {\"server_id\":\"s\",\"is_online\":true,\"player_count\":1,\
             \"max_players\":2,\"server_fps\":\"nope\",\"uptime_seconds\":3,\"updated_at\":\"t\"}\n\n",
        ) else {
            panic!("expected Rejected");
        };
        let msg = audit_rejected_frame("test", &error, &payload);
        assert!(msg.contains("server_fps"), "must name the field: {msg}");
        assert!(msg.contains("ServerStatusDto") && msg.contains("ServerStatus"));
        assert!(msg.contains("REJECTED and dropped"));
    }
}
