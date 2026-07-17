//! API response DTOs (snake_case = the API contract, ported from types/api). The generic list
//! envelope + the endpoint bodies the client/pages need; each is proven byte-exact against a live
//! backend by the **R-api gate** (the `#[cfg(test)] mod r_api` at the bottom): every committed
//! golden under `.ai/artifacts/t159_gates/fixtures/api/` — captured from a running Axum stack —
//! deserializes into its DTO and re-serializes **canonically byte-equal** to the golden. A dropped,
//! renamed, or wrong-typed field breaks the equality, so drift can't ship silently.
//!
//! Strong vs envelope: `MeResponse`/`ModpackDto`/`DashboardResponse`/`LinkStatus`/`Deployments`/
//! `Leaderboard` are fully typed (every field asserted). List bodies whose *item* type isn't ported
//! yet ride `Paginated<Value>` / `DataEnvelope<Value>` — the envelope contract is proven exactly,
//! the item type gets typed + strengthened when its page lands (T-159.8+).
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
    pub percent: f64,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// Live server telemetry frame — mirrors `types/models/telemetry` `ServerStatus` (SSE `data:`
/// payload + the `status` field of a server row). T-159.25.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerStatusDto {
    pub server_id: String,
    pub is_online: bool,
    pub player_count: i64,
    pub max_players: i64,
    pub server_fps: i64,
    pub uptime_seconds: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_match_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ingame_time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ingame_weather: Option<String>,
    pub updated_at: String,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
    pub created_at: String,
    pub updated_at: String,
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
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MissionEnv {
    pub terrain: String,
    pub time: String,
    pub weather: String,
    pub view_distance: i64,
    pub thermals: bool,
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
#[cfg(test)]
mod r_api {
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

    // Goldens are compile-time-embedded from the tracked fixture corpus (repo root is three dirs up
    // from apps/website-leptos/src/).
    const FX: &str = "../../../.ai/artifacts/t159_gates/fixtures/api/";
    macro_rules! golden {
        ($f:literal) => {
            include_str!(concat!(
                "../../../.ai/artifacts/t159_gates/fixtures/api/",
                $f
            ))
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
    #[test]
    fn event_hub() {
        assert_golden::<EventHub>(golden!(
            "GET__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7.json"
        ));
    }
    #[test]
    fn orbat_envelope() {
        assert_golden::<DataEnvelope<Value>>(golden!(
            "GET__event-missions__89b1b731-37a8-4926-901a-3c7ff7de5eb3__orbat.json"
        ));
    }

    // ── paginated `{data,total,limit,offset}` envelopes (item type ported per page) ──
    #[test]
    fn events_envelope() {
        assert_golden::<Paginated<Value>>(golden!("GET__events.json"));
    }
    #[test]
    fn missions_envelope() {
        assert_golden::<Paginated<Value>>(golden!("GET__missions.json"));
    }
    #[test]
    fn announcements_envelope() {
        assert_golden::<Paginated<Value>>(golden!("GET__announcements.json"));
    }
    #[test]
    fn approvals_envelope() {
        assert_golden::<Paginated<Value>>(golden!("GET__approvals.json"));
    }
    #[test]
    fn factions_envelope() {
        assert_golden::<Paginated<Value>>(golden!("GET__factions.json"));
    }
    #[test]
    fn admin_users_envelope() {
        assert_golden::<Paginated<AdminUserRow>>(golden!("GET__admin__users.json"));
    }
    #[test]
    fn audit_logs_envelope() {
        // audit logs use a cursor envelope, not offset/total.
        assert_golden::<CursorList<Value>>(golden!("GET__admin__audit-logs.json"));
    }

    // ── `{data}` envelopes ──
    #[test]
    fn servers_envelope() {
        assert_golden::<DataEnvelope<Value>>(golden!("GET__servers.json"));
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
}
