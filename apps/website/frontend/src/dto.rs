//! API response DTOs (snake_case = the API contract, ported from types/api). The generic list
//! envelope + the endpoint bodies the client/pages need; each is proven byte-exact against a live
//! backend by the **R-api gate** (the `#[cfg(test)] mod r_api` at the bottom): every committed
//! golden under `tests/fixtures/api/` — captured from a running Axum stack —
//! deserializes into its DTO and re-serializes **canonically byte-equal** to the golden.
//!
//! **T-394 — byte-equality alone does not prove that.** It compares two strings, and a
//! `#[serde(flatten)]` sibling can produce the right string on behalf of a named field that no
//! longer exists: drop `MissionCard::rejection_reason` and its key is simply collected by the
//! `extra: Map<String, Value>` catch-all and re-emitted verbatim — same bytes, green gate, field
//! gone from the type. So the gate has a **second, structural half** (`assert_golden` runs both):
//! every key in the golden must be *claimed* by a named, typed field, proven by poisoning the
//! value and requiring serde to reject it. Each test declares the golden's inventory of keys the
//! DTO does not read, and the set must match exactly, so the inventory cannot rot in either
//! direction. `byte_equality_alone_cannot_see_a_dropped_field_under_flatten` is the frozen proof
//! that the two halves really do disagree.
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
//! **T-360** typed `DashboardResponse::server_status` as `Option<ServerStatusDto>` — the third
//! read site of the same telemetry payload (alongside SSE + `/servers`). That removed the last
//! `Value` sink that let a `server_fps` type drift hide from the dashboard golden.
//!
//! **T-519** closed the remaining typed-DTO gap: `/members`, `/registry/compat`, and
//! `POST /fire-missions/solve` now have committed goldens under `tests/fixtures/api/` and
//! `r_api` round-trip pins (see `members_envelope`, `registry_compat_envelope`, `fire_solution`).
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

/// A mortar firing solution — mirrors backend `services::FireSolution`
/// (`POST /fire-missions/solve`).
///
/// **T-519 — field types match the Axum wire, not a guessed TS port.** The backend emits
/// `distance_m` / `azimuth_mils` / `charge` / `elevation_mils` as JSON integers (`i64` on the
/// Rust side). Typing `distance_m` as `f64` deserializes `1000` fine but re-serializes
/// `1000.0`, which fails the R-api canonical round-trip — the same class of latent drift
/// T-306 caught on `EventListItem::percent`. `azimuth_mils` and `charge` are named fields so
/// they cannot hide under `extra`.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct FireSolution {
    pub weapon_system: String,
    pub distance_m: i64,
    pub azimuth_deg: f64,
    pub azimuth_mils: i64,
    pub elevation_mils: i64,
    pub charge: i64,
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
/// plus its live `status`, required modpack, and join-sourced theater (`matches.terrain`).
///
/// **`status` is deliberately NOT `skip_serializing_if`.** The backend field is a plain
/// `Option<ServerStatus>`, so a server with no telemetry row serializes as an explicit
/// `"status": null` — which the third row of the committed golden carries. Omitting it here would
/// break the canonical byte-equality. `required_modpack` *is* skipped, matching the backend.
///
/// **`terrain` is the same contract as `status` (T-385).** Explicit `null` when the server has no
/// current match — never `Option` + `skip_serializing_if`. That encoding would round-trip
/// absent→None→absent and keep this gate green over a field the route never sends (the T-306 /
/// T-359 hazard). The golden carries `"terrain":"everon"` on the primary (joined) row.
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
    /// Theater from `matches.terrain` via `current_match_id` — JSON `null` when unmatched.
    pub terrain: Option<String>,
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
/// omitted), so none is `skip_serializing_if`. `server_status` is the same telemetry shape as the
/// SSE frame and `ServerRowDto::status` (**T-360** — was `Option<Value>`, which forced
/// `dashboard.rs` through T-232's `vf64` hand-parse and hid the next `server_fps` drift from this
/// golden). `next_event` / `my_assignment` / `recent_announcements` stay `Value` until those
/// nested bodies get their own DTOs; `current_modpack` is fully typed.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct DashboardResponse {
    pub next_event: Option<Value>,
    pub my_assignment: Option<Value>,
    pub server_status: Option<ServerStatusDto>,
    pub current_modpack: Option<ModpackDto>,
    pub recent_announcements: Vec<Value>,
}

/// `GET /me/deployments` — the caller's service record.
///
/// **T-233 — the combat block is `Option` on purpose, and must not be unwrapped to a default.**
/// `kd_ratio` is `None` for a player with no ingested matches and `command_win_rate` is `None`
/// whenever the player has never held a command slot (the common case). The backend distinguishes
/// those from a genuinely measured `0.0` and sends `null`, because the defect this replaced was a
/// constant `2.45` presented as telemetry — and `0.00` in an unmeasured slot is the same false
/// claim in a quieter voice. Render nothing, not a zero.
///
/// `command_win_rate` is **not** a general win rate and must not be labelled "Win Rate": its
/// denominator is `command_games`, i.e. only matches where the player held a command slot. A
/// general win rate is not derivable at all — see the note on `handlers/deployments.rs`.
///
/// Every field is required (the backend always emits all six keys, `null` where unmeasured), so
/// none is `skip_serializing_if`: `Option<f64>` round-trips `null` → `None` → `null`, which is the
/// shape the golden pins.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Deployments {
    pub total_operations: i64,
    pub attendance_rate: f64,
    pub kills: i64,
    pub deaths: i64,
    pub kd_ratio: Option<f64>,
    pub command_games: i64,
    pub command_wins: i64,
    pub command_win_rate: Option<f64>,
    pub service_history: Vec<Value>,
    pub upcoming: Vec<Value>,
}

/// One leave-of-absence row — mirrors backend `models::event::LeaveRequest`
/// (`POST/GET /me/leave-requests`, `GET /admin/leave-requests`).
///
/// **Dates on the wire are Go `time.Time` midnight UTC** (`2026-08-01T00:00:00Z`), not bare
/// `YYYY-MM-DD`. The create body (`CreateLeaveInput`) is the opposite: the handler parses
/// `starts_on`/`ends_on` as `%Y-%m-%d` only. Keep those shapes separate — feeding an RFC3339
/// string into POST is a 400.
///
/// `reason` / `reviewed_by` omit when empty/absent (backend `skip_serializing_if`), matching the
/// live Axum capture used by the unit round-trip below. No committed R-api golden yet: the
/// fixture corpus has no `/me/leave-requests` capture, and writing one is outside this slice's
/// `owns` (same known-gap shape as `/members` / `POST /fire-missions/solve`).
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LeaveRequest {
    pub id: String,
    pub discord_id: String,
    pub starts_on: String,
    pub ends_on: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
    /// `pending` | `approved` | `denied` — string, not an enum: the three-value Postgres
    /// `leave_status` is stable today, but a hard enum would 400 the SPA the day a fourth lands.
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_by: Option<String>,
    pub created_at: String,
}

/// Body for `POST /me/leave-requests`. Dates are **bare** `YYYY-MM-DD` (handler
/// `CreateLeaveInput`), not the RFC3339 midnight form the response uses.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateLeaveInput {
    pub starts_on: String,
    pub ends_on: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
}

/// Body for `PATCH /admin/leave-requests/:id` — `status` must be `approved` or `denied`.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewLeaveInput {
    pub status: String,
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
///
/// **T-427:** when the client passes `?limit=`/`?offset=`, the handler also returns
/// `total`/`limit`/`offset`. Omitting them keeps the legacy unpaginated envelope (golden-compatible).
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryResponse {
    pub data: Vec<RegistryItem>,
    pub etag: String,
    pub modpack_id: String,
    pub modpack_version: String,
    /// Present only on paginated responses (`?limit=`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
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
///
/// **T-427:** optional `total`/`limit`/`offset` when `?limit=` is set. For the editor cold path,
/// prefer a filtered `edge_type=` list (Arsenal families only) over the unfiltered dump.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryCompatResponse {
    pub data: Vec<RegistryCompatEdge>,
    pub etag: String,
    pub modpack_id: String,
    pub modpack_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
}

/// One aggregated cargo seed row from `GET /registry/compat?view=cargo_defaults` (T-427).
/// Shape matches `arsenal_rules::CargoRow` so the editor can install the map without re-walking
/// the ~16k raw `character_default_cargo` edges.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryCargoDefaultRow {
    pub container: String,
    pub item: String,
    pub qty: i64,
}

/// `GET /registry/compat?view=cargo_defaults` — slim per-character cargo seed map (T-427).
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryCargoDefaultsResponse {
    /// Echo of the requested view (`"cargo_defaults"`).
    pub view: String,
    /// `character resource_name` → aggregated cargo rows.
    pub data: std::collections::HashMap<String, Vec<RegistryCargoDefaultRow>>,
    pub etag: String,
    pub modpack_id: String,
    pub modpack_version: String,
    /// Raw `character_default_cargo` edge count before aggregation (server collapsed the walk).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_edge_count: Option<i64>,
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
    /// `users.total_deployments` — same denormalized counter as `/me` (T-448).
    pub total_deployments: i64,
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

impl MissionDetail {
    /// **T-243 — this row as the `MissionMeta` the shared mission compiler reads.**
    ///
    /// The one input the editor's server-truth Export cannot derive from the document. `/compiled`
    /// builds `MissionMeta` from the mission ROW
    /// (`services::mission_compile::flatten_to_mod_document`, the only reader of
    /// `missions.author_id` / `max_players` on that path), so a preview that guessed them would be
    /// a different document. This is the row half of the twin; the payload-first `time`/`weather`
    /// precedence is applied downstream by
    /// `map_engine_core::mission::flatten::apply_authored_environment`, which is why this copies
    /// the row's values straight across and does not try to reconcile them here.
    ///
    /// **`author` is `author_id`, not `author_name`.** The server sends the Discord id (`m.author_id`)
    /// and `author_name` is the display name beside it — the one field here a reasonable reading
    /// gets wrong, and it would produce a document that looks right and is not the served one.
    /// Pinned by `compiled_meta_is_the_row_the_server_compiles_from`.
    ///
    /// Every other field is a straight copy, checked by the compiler: this returns the struct rather
    /// than a hand-built camelCase JSON object precisely so there is no key to mistype (see
    /// `MissionMeta`'s note on why it is `Serialize`).
    pub fn compiled_meta(&self) -> map_engine_core::mission::flatten::MissionMeta {
        map_engine_core::mission::flatten::MissionMeta {
            id: self.id.clone(),
            title: self.title.clone(),
            author: self.author_id.clone(),
            terrain: self.terrain.clone(),
            custom_terrain_name: self.custom_terrain_name.clone().unwrap_or_default(),
            max_players: self.max_players,
            time_of_day: self.time_of_day.clone(),
            weather_preset: self.weather.clone(),
        }
    }
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
///
/// `server_id` / `modpack_id` (T-260 / migration 0011) are omitted when unset — Hub chip prefers
/// `modpack_id` over global `/modpacks/current` (T-442).
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
    /// Game server this operation is scheduled on (T-260). Absent when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
    /// Modpack this operation requires (T-260). Absent when unset — Hub falls back to current.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modpack_id: Option<String>,
}

/// The doc's terrain + environment fields, for the Mission Settings dialog. Pure data (no wasm
/// deps), so it lives here in the always-compiled DTO module: the wasm `editor_ops::read_env`
/// returns it, and the native `eden_chrome` view-shell fallback (`::default()`) needs it too.
#[derive(Clone, Debug, PartialEq)]
pub struct MissionEnv {
    pub terrain: String,
    pub time: String,
    pub weather: String,
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

    /// Live Axum shape from `POST /me/leave-requests` (T-265 probe, 2026-07-26): midnight-UTC
    /// dates, `reason` present, `reviewed_by` absent while pending. Re-serialize must keep
    /// `reviewed_by` omitted (skip_serializing_if), not emit `"reviewed_by":null`.
    #[test]
    fn leave_request_pending_round_trips_without_reviewed_by() {
        let json = r#"{"id":"44fa4c17-5bd5-4c6b-b02d-4ccd52af6910","discord_id":"000000000000000001","starts_on":"2026-09-01T00:00:00Z","ends_on":"2026-09-03T00:00:00Z","reason":"t265-probe","status":"pending","created_at":"2026-07-26T23:27:01.118063Z"}"#;
        let loa: LeaveRequest = serde_json::from_str(json).unwrap();
        assert_eq!(loa.status, "pending");
        assert!(loa.reviewed_by.is_none());
        assert_eq!(loa.starts_on, "2026-09-01T00:00:00Z");
        let back = serde_json::to_string(&loa).unwrap();
        assert!(
            !back.contains("reviewed_by"),
            "pending LOA must omit reviewed_by, got {back}"
        );
        let again: LeaveRequest = serde_json::from_str(&back).unwrap();
        assert_eq!(again, loa);
    }

    /// `{data:[LeaveRequest]}` — `GET /me/leave-requests` envelope the deployments page reads.
    #[test]
    fn leave_request_my_list_envelope() {
        let json = r#"{"data":[{"id":"44fa4c17-5bd5-4c6b-b02d-4ccd52af6910","discord_id":"000000000000000001","starts_on":"2026-09-01T00:00:00Z","ends_on":"2026-09-03T00:00:00Z","reason":"t265-probe","status":"pending","created_at":"2026-07-26T23:27:01.118063Z"}]}"#;
        let env: DataEnvelope<LeaveRequest> = serde_json::from_str(json).unwrap();
        assert_eq!(env.data.len(), 1);
        assert_eq!(env.data[0].reason, "t265-probe");
    }

    /// Create body is bare dates — the opposite of the response wire form.
    #[test]
    fn create_leave_input_serializes_bare_ymd() {
        let body = CreateLeaveInput {
            starts_on: "2026-08-01".into(),
            ends_on: "2026-08-05".into(),
            reason: "holiday".into(),
        };
        assert_eq!(
            serde_json::to_string(&body).unwrap(),
            r#"{"starts_on":"2026-08-01","ends_on":"2026-08-05","reason":"holiday"}"#
        );
    }
}

/* ══════════════════════════════ R-api gate ══════════════════════════════ */
// Each committed golden (captured from a running Axum `:8080` via dev-login — see the fixture dir's
// _index.tsv) must round-trip through its DTO **canonically byte-equal**. `canon` sorts object keys
// recursively (order-independent, works with or without serde_json's preserve_order feature) and
// normalizes whitespace/number-repr on BOTH sides equally, so the assertion isolates exactly one
// thing: does the DTO's serialized field-set + values match the live backend's?
//
// T-394: that comparison is textual, so on a struct carrying `#[serde(flatten)]` it is satisfied
// by the flattened sibling re-emitting a key whose named field was deleted. `assert_golden` is
// therefore two assertions — the byte-equality above, plus `assert_every_wire_key_is_claimed`,
// which asks the *type* whether anything actually reads each key. Together they are the
// load-bearing R-api proof (stronger than a browser round-trip: deterministic, no network,
// compile-time-pinned goldens).
// `pub(crate)` so `sse.rs`'s own tests can drive the one captured live SSE frame
// (`LIVE_SSE_FRAME`) through the real decoder instead of keeping a second, drifting copy of it.
#[cfg(test)]
pub(crate) mod r_api {
    use super::*;
    use serde::de::DeserializeOwned;
    use serde_json::json;

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

    /// Half one — the textual gate: `golden` must deserialize into `T` and re-serialize
    /// canonical-equal to `golden`. This is the original `assert_golden`, unchanged.
    ///
    /// **It cannot see a dropped field on any struct carrying `#[serde(flatten)]`** — see
    /// [`assert_every_wire_key_is_claimed`] for the half that can, and
    /// [`byte_equality_alone_cannot_see_a_dropped_field_under_flatten`] for the proof that it
    /// can't.
    fn assert_canonical_round_trip<T: Serialize + DeserializeOwned>(golden: &str) {
        let dto: T = serde_json::from_str(golden)
            .unwrap_or_else(|e| panic!("R-api: golden does not deserialize into the DTO: {e}"));
        let back = serde_json::to_string(&dto).expect("DTO re-serializes");
        assert_eq!(
            canon(golden),
            canon(&back),
            "R-api: DTO must re-serialize canonically byte-equal to the live-backend golden"
        );
    }

    /* ═══════════════════════ T-394 — the structural half of the gate ═══════════════════════
       `assert_canonical_round_trip` compares two *strings*. A `#[serde(flatten)]` sibling can
       satisfy that comparison on behalf of a named field that no longer exists: delete
       `MissionCard::rejection_reason` and the key it used to own is simply collected by the
       `extra: Map<String, Value>` catch-all instead, then re-emitted verbatim. Same bytes out,
       gate green, field gone from the type. T-389 proved it as a negative control — with
       `#[serde(skip)]` on `rejection_reason` the byte-equality passed and only its hand-written
       "is this a named field with a value" assertion failed.

       Hand-writing that assertion per field is not the fix: it has to be written again for every
       field of every struct, and the fields it is NOT written for stay invisible — the same
       "the gate can only see what someone remembered to look at" defect in a new costume.

       So this half asserts on the **deserialized type** instead of the text, and it needs no
       per-field code at all. For every position in the golden it replaces the value with a
       poison and asks serde whether that breaks the decode. A named field with a real type
       rejects at least one poison (a `String`/number/bool rejects both, a `Vec<_>` rejects the
       object, a struct/map rejects the array). A key that is only being swept into a flatten
       catch-all rejects neither — `Map<String, Value>` takes anything — and that is exactly the
       signal "no named field of this DTO is reading this key". A `Value`-typed field is
       indistinguishable from a catch-all here, correctly: it, too, reads nothing (T-306).

       Each test therefore declares the golden's *inventory of holes* and the set must match
       exactly, so the inventory cannot rot in either direction: a new entry means a named field
       was dropped or renamed, a missing entry means a key was promoted to a named field.
    */

    /// One step down into the golden's JSON tree. Array indices render as `*`, so the inventory
    /// is one line per **shape** rather than one per row.
    #[derive(Clone, Copy)]
    enum Step<'a> {
        Key(&'a str),
        Index(usize),
    }

    fn render(path: &[Step<'_>]) -> String {
        path.iter()
            .map(|s| match s {
                Step::Key(k) => (*k).to_string(),
                Step::Index(_) => "*".to_string(),
            })
            .collect::<Vec<_>>()
            .join("/")
    }

    fn node_at<'a>(root: &'a Value, path: &[Step<'_>]) -> &'a Value {
        let mut cur = root;
        for step in path {
            cur = match step {
                Step::Key(k) => &cur[*k],
                Step::Index(i) => &cur[*i],
            };
        }
        cur
    }

    /// `root` with the node at `path` replaced by `poison`.
    fn poisoned(root: &Value, path: &[Step<'_>], poison: &Value) -> Value {
        let mut out = root.clone();
        let mut cur = &mut out;
        for step in path {
            cur = match step {
                Step::Key(k) => cur.get_mut(*k).expect("path was walked out of this tree"),
                Step::Index(i) => cur.get_mut(*i).expect("path was walked out of this tree"),
            };
        }
        *cur = poison.clone();
        out
    }

    /// Is the node at `path` **claimed** — i.e. does some named field of `T` actually read it?
    ///
    /// Two poisons, because one is not enough: an object is rejected by every scalar and by
    /// `Vec<_>`, an array is rejected by every scalar and by every struct/map. Claimed means at
    /// least one of them breaks the decode.
    fn claimed<T: DeserializeOwned>(root: &Value, path: &[Step<'_>]) -> bool {
        [
            json!({ "__t394_poison__": true }),
            json!(["__t394_poison__"]),
        ]
        .iter()
        .any(|p| serde_json::from_value::<T>(poisoned(root, path, p)).is_err())
    }

    /// Depth-first over the golden. An unclaimed position is recorded and **not** descended into:
    /// everything under a `Value` sink is unclaimed by construction, and listing it would bury the
    /// one line that matters.
    fn walk<'a, T: DeserializeOwned>(
        root: &'a Value,
        path: &mut Vec<Step<'a>>,
        out: &mut Vec<String>,
    ) {
        let visit = |path: &mut Vec<Step<'a>>, out: &mut Vec<String>| {
            if claimed::<T>(root, path) {
                walk::<T>(root, path, out);
            } else {
                out.push(render(path));
            }
        };
        match node_at(root, path) {
            Value::Object(m) => {
                for k in m.keys() {
                    path.push(Step::Key(k));
                    visit(path, out);
                    path.pop();
                }
            }
            Value::Array(a) => {
                for i in 0..a.len() {
                    path.push(Step::Index(i));
                    visit(path, out);
                    path.pop();
                }
            }
            _ => {}
        }
    }

    /// Every key on the wire that no named field of `T` reads.
    fn unclaimed_keys<T: DeserializeOwned>(golden: &str) -> Vec<String> {
        let root: Value = serde_json::from_str(golden).expect("golden is valid JSON");
        let mut out = Vec::new();
        walk::<T>(&root, &mut Vec::new(), &mut out);
        out.sort();
        out.dedup();
        out
    }

    /// Half two — the structural gate: the golden's unclaimed-key set must be exactly `expected`.
    fn assert_every_wire_key_is_claimed<T: DeserializeOwned>(golden: &str, expected: &[&str]) {
        let got = unclaimed_keys::<T>(golden);
        let mut want: Vec<String> = expected.iter().map(|s| (*s).to_string()).collect();
        want.sort();
        want.dedup();
        if got == want {
            return;
        }
        let appeared: Vec<&String> = got.iter().filter(|k| !want.contains(k)).collect();
        let vanished: Vec<&String> = want.iter().filter(|k| !got.contains(k)).collect();
        panic!(
            "R-api (T-394): the set of wire keys no named DTO field claims has changed.\n\
             NEWLY UNCLAIMED {appeared:?}\n  \
             — a named field was dropped or renamed, and a `#[serde(flatten)]` catch-all is now\n  \
             swallowing its key. Byte-equality stays green through this; that is the whole point\n  \
             of this assertion.\n\
             NO LONGER UNCLAIMED {vanished:?}\n  \
             — the key is a named field now. Drop it from this test's list.\n\
             full unclaimed set: {got:?}"
        );
    }

    /// The gate: both halves. `unclaimed` is this golden's inventory of keys the DTO does not
    /// read — `&[]` means the DTO claims every byte on the wire.
    fn assert_golden<T: Serialize + DeserializeOwned>(golden: &str, unclaimed: &[&str]) {
        assert_canonical_round_trip::<T>(golden);
        assert_every_wire_key_is_claimed::<T>(golden, unclaimed);
    }

    /// **The proof that the structural half is not vacuous, frozen as a test.**
    ///
    /// Two shapes of the same wire row. `Claimed` names `rejection_reason`; `Absorbed` is the
    /// T-389 negative control — the field is `#[serde(skip)]`ped, so the flatten catch-all takes
    /// the key instead. The old gate cannot tell them apart. The new one can, and this runs on
    /// every `cargo test` rather than living in a commit message.
    #[derive(Serialize, Deserialize)]
    struct Claimed {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rejection_reason: Option<String>,
        #[serde(flatten)]
        extra: serde_json::Map<String, Value>,
    }

    #[derive(Serialize, Deserialize)]
    struct Absorbed {
        id: String,
        #[serde(skip)]
        #[allow(dead_code)]
        rejection_reason: Option<String>,
        #[serde(flatten)]
        extra: serde_json::Map<String, Value>,
    }

    #[test]
    fn byte_equality_alone_cannot_see_a_dropped_field_under_flatten() {
        const ROW: &str = r#"{"id":"m1","rejection_reason":"too many AI"}"#;
        // The defect: the textual gate is green either way. Deleting the named field changed
        // nothing it can measure, because `extra` re-emits the key byte-for-byte.
        assert_canonical_round_trip::<Claimed>(ROW);
        assert_canonical_round_trip::<Absorbed>(ROW);
        // The structural gate separates them by asking the *type*, not the text.
        assert_eq!(unclaimed_keys::<Claimed>(ROW), Vec::<String>::new());
        assert_eq!(
            unclaimed_keys::<Absorbed>(ROW),
            vec!["rejection_reason".to_string()],
            "with no named field reading it, the key is only being swept into `extra`"
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
        assert_golden::<MeResponse>(golden!("GET__me.json"), &[]);
    }
    #[test]
    fn modpack_current() {
        assert_golden::<ModpackDto>(golden!("GET__modpacks__current.json"), &[]);
    }
    #[test]
    fn dashboard() {
        // Three still-untyped nested bodies. `server_status` was the fourth (T-306 escape) —
        // T-360 typed it as `ServerStatusDto`, so it MUST NOT appear here: if it returns to this
        // list the structural half has gone blind to the third telemetry read site again.
        assert_golden::<DashboardResponse>(
            golden!("GET__dashboard.json"),
            &["my_assignment", "next_event", "recent_announcements/*"],
        );
    }
    #[test]
    fn link_status() {
        assert_golden::<LinkStatus>(golden!("GET__me__link__status.json"), &[]);
    }
    #[test]
    fn deployments() {
        // Both lists are `Vec<Value>` — the service-record rows and the upcoming ops are not
        // ported types yet, so nothing below them is asserted.
        assert_golden::<Deployments>(
            golden!("GET__me__deployments.json"),
            &["service_history/*", "upcoming/*"],
        );
    }
    #[test]
    fn leaderboards() {
        // `data` is `Vec<Value>` — the envelope is proven, the row is not.
        assert_golden::<Leaderboard>(golden!("GET__leaderboards.json"), &["data/*"]);
    }
    #[test]
    fn registry_envelope() {
        assert_golden::<RegistryResponse>(golden!("GET__registry.json"), &[]);
    }
    /// T-519 — `GET /registry/compat` was a typed DTO (`RegistryCompatResponse`) with **no**
    /// fixture, so the gate could not speak to edge shape / etag / modpack identity. Captured
    /// off a running Axum stack (`?edge_type=mag_in_vehicle_weapon`); `data` truncated to two
    /// live rows so the corpus stays small while every named edge field is still populated
    /// (incl. non-empty `evidence` + `qty`).
    #[test]
    fn registry_compat_envelope() {
        const G: &str = golden!("GET__registry__compat.json");
        assert_golden::<RegistryCompatResponse>(G, &[]);
        let body: RegistryCompatResponse = serde_json::from_str(G).unwrap();
        assert!(
            body.data.len() >= 2,
            "compat golden must carry ≥2 edges so qty/evidence/timestamps are exercised"
        );
        assert!(
            body.data.iter().all(|e| !e.id.is_empty()
                && !e.from_node.is_empty()
                && !e.to_node.is_empty()
                && !e.edge_type.is_empty()
                && e.qty >= 1),
            "each edge must round-trip populated named fields"
        );
        assert!(
            !body.etag.is_empty()
                && !body.modpack_id.is_empty()
                && !body.modpack_version.is_empty(),
            "cache-identity fields must be present"
        );
    }
    /// T-519 — `POST /fire-missions/solve` body. Live capture at FP (0,0) → TGT (0,1000) on
    /// `M252 81mm`. Pins integer `distance_m` / `azimuth_mils` / `charge` (T-306 class: an
    /// `f64` `distance_m` greened deserialize and failed only on re-serialize).
    #[test]
    fn fire_solution() {
        const G: &str = golden!("POST__fire-missions__solve.json");
        assert_golden::<FireSolution>(G, &[]);
        let sol: FireSolution = serde_json::from_str(G).unwrap();
        assert_eq!(sol.weapon_system, "M252 81mm");
        assert_eq!(sol.distance_m, 1000);
        assert_eq!(sol.azimuth_mils, 0);
        assert_eq!(sol.charge, 1);
        assert!(sol.elevation_mils > 800, "high-angle solution");
        assert!(
            sol.extra.is_empty(),
            "every wire key must be a named field, not absorbed by extra"
        );
    }
    #[test]
    fn mission_detail() {
        // `json_payload` is the editor superset, deliberately opaque (`Value`).
        assert_golden::<MissionDetail>(
            golden!("GET__missions__512d8658-7025-4a70-94e9-a1b44a7aa155.json"),
            &["current_version/json_payload"],
        );
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
        assert_golden::<MissionDetail>(G, &["current_version/json_payload"]);
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
    /// **T-243 — the row half of the editor's server-truth Export, against a REAL captured row.**
    ///
    /// `MissionDetail::compiled_meta` feeds `flatten_mod_document_json`, whose output is pinned
    /// byte-identical to `GET /missions/:id/compiled` by
    /// `website-api`'s `client_twin_is_byte_identical_to_the_compiled_route`. That test supplies its
    /// own meta, so it proves the *compiler* agrees; this one proves the *editor* hands it the same
    /// row the server would have read. Both halves or the preview is only half-checked.
    ///
    /// The golden is used rather than a hand-built struct on purpose: a literal fixture would be
    /// written from the same misreading as the code it checks.
    #[test]
    fn compiled_meta_is_the_row_the_server_compiles_from() {
        const G: &str = golden!("GET__missions__512d8658-7025-4a70-94e9-a1b44a7aa155.json");
        let d: MissionDetail = serde_json::from_str(G).unwrap();
        let meta = d.compiled_meta();

        // `services::mission_compile::flatten_to_mod_document` reads `m.author_id`. The golden's
        // two author fields differ, so picking `author_name` here fails rather than coinciding.
        assert_ne!(
            d.author_id, d.author_name,
            "this golden can no longer tell author_id from author_name — recapture one that can",
        );
        assert_eq!(
            meta.author, d.author_id,
            "author is the Discord id, not the display name"
        );

        assert_eq!(meta.id, d.id);
        assert_eq!(meta.title, d.title);
        assert_eq!(meta.terrain, d.terrain);
        assert_eq!(meta.max_players, d.max_players);
        assert_eq!(meta.time_of_day, d.time_of_day);
        assert_eq!(meta.weather_preset, d.weather);
        assert_eq!(
            meta.custom_terrain_name,
            d.custom_terrain_name.clone().unwrap_or_default(),
            "an absent custom terrain is the empty string the row column holds, not a literal null",
        );

        // Nothing load-bearing may be silently empty: an all-`Default` meta would satisfy several
        // of the equalities above if the golden itself went blank.
        assert!(!meta.id.is_empty() && !meta.title.is_empty() && !meta.terrain.is_empty());
        assert!(
            meta.max_players > 0,
            "playerRange upper bound comes from here"
        );

        // And the wire round trip the wasm caller actually performs: serialize → the camelCase
        // bytes `flatten_mod_document_json` parses → back. A rename on either side breaks this.
        let json = serde_json::to_string(&meta).unwrap();
        let back: map_engine_core::mission::flatten::MissionMeta =
            serde_json::from_str(&json).unwrap();
        assert_eq!(
            back.max_players, meta.max_players,
            "maxPlayers survives the round trip"
        );
        assert_eq!(
            back.time_of_day, meta.time_of_day,
            "timeOfDay survives the round trip"
        );
        assert_eq!(back.weather_preset, meta.weather_preset);
        assert_eq!(back.custom_terrain_name, meta.custom_terrain_name);
        assert_eq!(back.author, meta.author);
    }
    /// T-403 — `armory_by_faction` used to be `[]` in the only Event Hub golden, so
    /// `ArmoryFaction` / `ArmoryItem` had zero structural coverage: `#[serde(skip)]` on every
    /// named field still greened this test. The golden now carries real-shaped MissionArmory
    /// rows (same shape `GET /events/:id` emits from `ArmoryFactionDto` + `MissionArmory`).
    ///
    /// Named fields claim `faction`/`items`/`id`/`item_name`/`quantity`. The rest of each
    /// armory row rides `ArmoryItem::extra` — that inventory is what stands between the
    /// catch-all and a silent field drop on the named three.
    const EVENT_HUB_ARMORY_EXTRA: &[&str] = &[
        "missions/*/armory_by_faction/*/items/*/category",
        "missions/*/armory_by_faction/*/items/*/faction",
        "missions/*/armory_by_faction/*/items/*/icon",
        "missions/*/armory_by_faction/*/items/*/mission_id",
        "missions/*/armory_by_faction/*/items/*/sort_order",
    ];

    #[test]
    fn event_hub() {
        const G: &str = golden!("GET__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7.json");
        assert_golden::<EventHub>(G, EVENT_HUB_ARMORY_EXTRA);
        // Anti-vacuous: the corpus must actually exercise the armory DTOs.
        let hub: EventHub = serde_json::from_str(G).unwrap();
        assert!(
            !hub.missions.is_empty() && !hub.missions[0].armory_by_faction.is_empty(),
            "T-403: event-hub golden must carry non-empty armory_by_faction"
        );
        assert!(
            hub.missions[0]
                .armory_by_faction
                .iter()
                .any(|f| !f.items.is_empty()),
            "T-403: at least one faction must carry non-empty items"
        );
        let item = &hub.missions[0].armory_by_faction[0].items[0];
        assert!(
            !item.id.is_empty() && !item.item_name.is_empty(),
            "T-403: ArmoryItem named fields must round-trip populated values"
        );
        assert!(
            item.quantity.is_some(),
            "T-403: quantity must be present on a golden row"
        );
    }

    /// T-403 — frozen proof that skip-all on the armory DTOs is visible once the golden is
    /// non-empty. Mirrors T-394's Claimed/Absorbed pair for `ArmoryFaction` / `ArmoryItem`.
    #[test]
    fn armory_skip_all_fields_is_visible_under_populated_golden() {
        const FACTION: &str = r#"{
            "faction":"BLUFOR",
            "items":[{
                "id":"a1000000-0000-4000-8000-000000000001",
                "mission_id":"512d8658-7025-4a70-94e9-a1b44a7aa155",
                "faction":"BLUFOR",
                "category":"rifle",
                "item_name":"M4A1",
                "quantity":24,
                "icon":"m4.png",
                "sort_order":0
            }]
        }"#;
        const ITEM: &str = r#"{
            "id":"a1000000-0000-4000-8000-000000000001",
            "mission_id":"512d8658-7025-4a70-94e9-a1b44a7aa155",
            "faction":"BLUFOR",
            "category":"rifle",
            "item_name":"M4A1",
            "quantity":24,
            "icon":"m4.png",
            "sort_order":0
        }"#;

        #[derive(Serialize, Deserialize)]
        struct AbsorbedArmoryItem {
            #[serde(skip)]
            #[allow(dead_code)]
            id: String,
            #[serde(skip)]
            #[allow(dead_code)]
            item_name: String,
            #[serde(skip)]
            #[allow(dead_code)]
            quantity: Option<i64>,
            #[serde(flatten)]
            extra: serde_json::Map<String, Value>,
        }

        #[derive(Serialize, Deserialize)]
        struct AbsorbedArmoryFaction {
            #[serde(skip)]
            #[allow(dead_code)]
            faction: String,
            #[serde(skip)]
            #[allow(dead_code)]
            items: Vec<AbsorbedArmoryItem>,
            #[serde(flatten)]
            extra: serde_json::Map<String, Value>,
        }

        // Live DTOs claim the named keys; extras ride the flatten.
        assert_eq!(
            unclaimed_keys::<ArmoryFaction>(FACTION),
            vec![
                "items/*/category".to_string(),
                "items/*/faction".to_string(),
                "items/*/icon".to_string(),
                "items/*/mission_id".to_string(),
                "items/*/sort_order".to_string(),
            ]
        );
        assert_eq!(
            unclaimed_keys::<ArmoryItem>(ITEM),
            vec![
                "category".to_string(),
                "faction".to_string(),
                "icon".to_string(),
                "mission_id".to_string(),
                "sort_order".to_string(),
            ]
        );

        // Total skip: every wire key is only swept into `extra`. Byte-equality stays green;
        // the structural half is what fails the Event Hub gate if this ever lands on the
        // real DTOs against the populated golden.
        assert_canonical_round_trip::<AbsorbedArmoryFaction>(FACTION);
        assert_canonical_round_trip::<AbsorbedArmoryItem>(ITEM);
        assert_eq!(
            unclaimed_keys::<AbsorbedArmoryFaction>(FACTION),
            vec!["faction".to_string(), "items".to_string()],
            "skip-all ArmoryFaction: faction+items must show as unclaimed"
        );
        assert_eq!(
            unclaimed_keys::<AbsorbedArmoryItem>(ITEM),
            vec![
                "category".to_string(),
                "faction".to_string(),
                "icon".to_string(),
                "id".to_string(),
                "item_name".to_string(),
                "mission_id".to_string(),
                "quantity".to_string(),
                "sort_order".to_string(),
            ],
            "skip-all ArmoryItem: every wire key must show as unclaimed"
        );
    }
    /// T-306 — was `DataEnvelope<Value>` while the ORBAT selector reads `DataEnvelope<OrbatSquad>`
    /// live. Typing it immediately failed and found `OrbatSlot::assigned_to` skipping a key the
    /// backend emits as an explicit `null`.
    #[test]
    fn orbat_envelope() {
        assert_golden::<DataEnvelope<OrbatSquad>>(
            golden!("GET__event-missions__89b1b731-37a8-4926-901a-3c7ff7de5eb3__orbat.json"),
            &[],
        );
    }

    // ── paginated `{data,total,limit,offset}` envelopes (item type ported per page) ──
    /// T-306 — was `Paginated<Value>` while `event_manager` reads `Paginated<EventListItem>`. Typing
    /// it pinned `percent` as the `i64` the backend actually sends (the golden's `37` re-serialized
    /// as `37.0` from the old `f64`).
    #[test]
    fn events_envelope() {
        // The three row keys `EventListItem` does not name; they ride the `extra` catch-all.
        assert_golden::<Paginated<EventListItem>>(
            golden!("GET__events.json"),
            &[
                "data/*/created_at",
                "data/*/created_by",
                "data/*/updated_at",
            ],
        );
    }
    /// The four `GET /missions` row keys `MissionCard` does not name — they ride the `extra`
    /// catch-all, and this list is what stands between that catch-all and the T-394 blindness: the
    /// moment a *named* field stops being named, its key joins this set and both `/missions` tests
    /// go red. Shared by the two goldens so they cannot drift apart.
    const MISSION_CARD_EXTRA: &[&str] = &[
        "data/*/bookmarked",
        "data/*/created_at",
        "data/*/current_version_id",
        "data/*/updated_at",
    ];

    /// T-306 — was `Paginated<Value>` while the Mission Library reads `Paginated<MissionCard>`.
    #[test]
    fn missions_envelope() {
        assert_golden::<Paginated<MissionCard>>(golden!("GET__missions.json"), MISSION_CARD_EXTRA);
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
        assert_golden::<Paginated<MissionCard>>(G, MISSION_CARD_EXTRA);
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
        assert_golden::<Paginated<Value>>(golden!("GET__announcements.json"), &["data/*"]);
    }
    /// T-306 — was `Paginated<Value>` while the approvals queue reads `Paginated<ApprovalRow>`.
    #[test]
    fn approvals_envelope() {
        assert_golden::<Paginated<ApprovalRow>>(golden!("GET__approvals.json"), &[]);
    }
    /// T-306 — was `Paginated<Value>`; the faction manager reads `FactionListResponse`, which is not
    /// even the same envelope shape the test was asserting.
    #[test]
    fn factions_envelope() {
        assert_golden::<FactionListResponse>(golden!("GET__factions.json"), &[]);
    }
    #[test]
    fn admin_users_envelope() {
        assert_golden::<Paginated<AdminUserRow>>(golden!("GET__admin__users.json"), &[]);
    }
    /// Cursor envelope, not offset/total — and `Value` is honest here: the audit page reads
    /// `CursorList<Value>` too.
    #[test]
    fn audit_logs_envelope() {
        assert_golden::<CursorList<Value>>(
            golden!("GET__admin__audit-logs.json"),
            &["data/*", "next_cursor"],
        );
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
        assert_golden::<DataEnvelope<ServerRowDto>>(golden!("GET__servers.json"), &[]);
    }
    /// T-519 — `GET /members` assignee-picker body. The Event Hub reads
    /// `DataEnvelope<Member>`; the golden was missing entirely so `avatar_url`'s
    /// present/absent (`skip_serializing_if`) cases had zero structural coverage. Live capture
    /// includes both shapes (Brandt omits; Okafor carries a CDN URL).
    #[test]
    fn members_envelope() {
        const G: &str = golden!("GET__members.json");
        assert_golden::<DataEnvelope<Member>>(G, &[]);
        let env: DataEnvelope<Member> = serde_json::from_str(G).unwrap();
        assert!(
            env.data.len() >= 2,
            "members golden must cover more than one row"
        );
        assert!(
            env.data.iter().any(|m| m.avatar_url.is_some()),
            "at least one row must carry avatar_url (present case)"
        );
        assert!(
            env.data.iter().any(|m| m.avatar_url.is_none()),
            "at least one row must omit avatar_url (absent case)"
        );
        assert!(
            env.data
                .iter()
                .all(|m| !m.discord_id.is_empty() && !m.username.is_empty()),
            "discord_id and username must round-trip populated"
        );
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
        assert_golden::<DataEnvelope<Value>>(golden!("GET__wiki.json"), &["data/*"]);
    }
    #[test]
    fn vehicle_db_envelope() {
        assert_golden::<DataEnvelope<Value>>(golden!("GET__vehicle-database.json"), &["data/*"]);
    }
    #[test]
    fn modpacks_list_envelope() {
        assert_golden::<DataEnvelope<Value>>(golden!("GET__modpacks.json"), &["data/*"]);
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
