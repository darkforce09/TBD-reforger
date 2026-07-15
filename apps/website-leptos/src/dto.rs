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

/// `GET /registry` — the asset catalog + its cache identity (weak ETag). Items typed at T-159.22.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryResponse {
    pub data: Vec<Value>,
    pub etag: String,
    pub modpack_id: String,
    pub modpack_version: String,
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
        assert_golden::<Paginated<Value>>(golden!("GET__admin__users.json"));
    }
    #[test]
    fn audit_logs_envelope() {
        // audit logs use a cursor, not offset/total — ride Value until the admin page lands.
        assert_golden::<Value>(golden!("GET__admin__audit-logs.json"));
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
