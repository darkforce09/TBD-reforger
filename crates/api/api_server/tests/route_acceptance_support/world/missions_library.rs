//! The missions library part's world: a live, a pending and a draft mission of one author, two
//! of that author's factions, and a small modpack registry; consuming probes get fresh rows.
//!
//! **Role:** implements [`PartWorld`] for the mission library, lifecycle, version, armory,
//! export, review workspace, default-override, faction and registry routes.
//!
//! **Position:** mounted by `tests/route_acceptance_missions_library.rs` with `#[path]`; its
//! specs are `specs/missions_library.rs`. Rows are made through the real routes (create, save,
//! submit, approve, faction create) except the registry catalogue, which is inserted directly.
//!
//! **Signals & state:** the seeded row ids and a counter for unique faction names and slot
//! roles.
//!
//! **Invariants:** every mission and faction belongs to the part's primary mission maker, so the
//! peer mission maker is always the non-owner and an administrator the override; each saved
//! payload carries a unique slot role, so no two artifacts compile to the same digest; the
//! registry modpack is never the current modpack, so its fixtures name it explicitly.

use std::sync::atomic::{AtomicU32, Ordering};

use api_configuration::configuration::Config;
use api_server::router::router;
use api_state::AppState;
use axum::Router;
use axum::http::StatusCode;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::common::COMPILABLE_EDITOR_PAYLOAD;
use crate::route_acceptance_support::actors::Actors;
use crate::route_acceptance_support::requests::{Outgoing, send};
use crate::route_acceptance_support::spec::{Actor, Role};
use crate::route_acceptance_support::world::{Fixture, PartWorld, WorldCore};

/// The version body limit the world configures: small, so the over-limit probe stays cheap.
const VERSION_BODY_LIMIT: i64 = 2 << 20;
/// The committed faction-library sample every faction body starts from.
fn faction_sample() -> std::path::PathBuf {
    repository_root::find_repository_root_from(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
        .expect("the repository root above the API package")
        .join("contracts/fixtures/registry/faction-library.sample.json")
}
const RIFLE: &str = "{00000000000000A1}Prefabs/Weapons/Rifles/Rifle_RouteAcceptance.et";
const MAGAZINE: &str = "{00000000000000A2}Prefabs/Weapons/Magazines/Magazine_RouteAcceptance.et";
const CHARACTER: &str = "{00000000000000A3}Prefabs/Characters/Character_RouteAcceptance.et";

/// The missions library world.
pub(crate) struct MissionsLibraryWorld {
    live: Uuid,
    live_version: Uuid,
    live_artifact: Uuid,
    pending: Uuid,
    pending_artifact: Uuid,
    draft: Uuid,
    draft_version: Uuid,
    faction: Uuid,
    sibling_faction_name: String,
    modpack: Uuid,
    counter: AtomicU32,
    namespace: u8,
}

fn uuid_at(body: &Value, pointer: &str) -> Uuid {
    body.pointer(pointer)
        .and_then(Value::as_str)
        .and_then(|raw| raw.parse().ok())
        .unwrap_or_else(|| panic!("expected a UUID at {pointer} in {body}"))
}

/// Setup requests through the real routes, as the part's primary mission maker (the author)
/// or its administrator.
struct Setup<'a> {
    app: &'a Router,
    author: String,
    admin: String,
}

impl Setup<'_> {
    async fn call(&self, token: &str, outgoing: Outgoing, expected: StatusCode) -> Value {
        let described = format!("{} {}", outgoing.method, outgoing.uri);
        let received = send(self.app, &outgoing.bearer(token)).await;
        assert_eq!(
            received.status,
            expected,
            "setup {described}: {}",
            received.excerpt()
        );
        received.json().unwrap_or(Value::Null)
    }

    /// A draft mission of the author titled `title`; answers `(mission, its initial version)`.
    async fn draft_titled(&self, title: &str) -> (Uuid, Uuid) {
        let body = json!({"title": title, "terrain": "everon",
            "game_mode": "pve_coop", "max_players": 16});
        let created = self
            .call(
                &self.author,
                Outgoing::new("POST", "/api/v1/missions").json(&body),
                StatusCode::CREATED,
            )
            .await;
        (
            uuid_at(&created, "/id"),
            uuid_at(&created, "/current_version_id"),
        )
    }

    /// A draft mission of the author; answers `(mission, its initial version)`.
    async fn draft(&self) -> (Uuid, Uuid) {
        self.draft_titled("Route acceptance mission").await
    }

    async fn save_version(&self, mission: Uuid, body: &Value) -> Uuid {
        let uri = format!("/api/v1/missions/{mission}/versions");
        let saved = self
            .call(
                &self.author,
                Outgoing::new("POST", uri).json(body),
                StatusCode::CREATED,
            )
            .await;
        uuid_at(&saved, "/id")
    }

    /// Submit `mission` for review; answers the artifact the review decides.
    async fn submit(&self, state: &AppState, mission: Uuid) -> Uuid {
        let uri = format!("/api/v1/missions/{mission}/submit");
        self.call(&self.author, Outgoing::new("POST", uri), StatusCode::OK)
            .await;
        sqlx::query_scalar("SELECT id FROM mission_artifacts WHERE mission_id = $1")
            .bind(mission)
            .fetch_one(&state.pool)
            .await
            .expect("the submission's artifact")
    }

    async fn approve(&self, mission: Uuid, artifact: Uuid) {
        let uri = format!("/api/v1/approvals/{mission}/approve");
        let body = json!({"artifact_id": artifact});
        self.call(
            &self.admin,
            Outgoing::new("POST", uri).json(&body),
            StatusCode::OK,
        )
        .await;
    }

    async fn delete(&self, uri: String) {
        self.call(
            &self.author,
            Outgoing::new("DELETE", uri),
            StatusCode::NO_CONTENT,
        )
        .await;
    }

    async fn faction(&self, document: &Value) -> Uuid {
        let created = self
            .call(
                &self.author,
                Outgoing::new("POST", "/api/v1/factions").json(document),
                StatusCode::CREATED,
            )
            .await;
        uuid_at(&created, "/id")
    }
}

/// A faction-library document named `name`.
fn faction_document(name: &str) -> Value {
    let raw = std::fs::read(faction_sample()).expect("read the faction-library sample");
    let mut document: Value = serde_json::from_slice(&raw).expect("the sample is JSON");
    document["name"] = json!(name);
    document
}

async fn seed_registry(state: &AppState, namespace: u8) -> Uuid {
    let modpack: Uuid = sqlx::query_scalar(
        "INSERT INTO modpacks (name, version, total_size_bytes, is_current, created_at) \
         VALUES ($1, '1.0.0', 0, false, now()) RETURNING id",
    )
    .bind(format!("Route acceptance modpack {namespace}"))
    .fetch_one(&state.pool)
    .await
    .expect("seed the registry modpack");
    sqlx::query(
        "INSERT INTO registry_items (modpack_id, resource_name, display_name, category, kind, \
         sort_order, created_at, updated_at) VALUES \
         ($1, $2, 'Route acceptance rifle', 'Rifles', 'gear_primary', 1, now(), now()), \
         ($1, $3, 'Route acceptance magazine', 'Magazines', 'magazine', 2, now(), now()), \
         ($1, $4, 'Route acceptance rifleman', 'Characters', 'character', 3, now(), now())",
    )
    .bind(modpack)
    .bind(RIFLE)
    .bind(MAGAZINE)
    .bind(CHARACTER)
    .execute(&state.pool)
    .await
    .expect("seed the registry items");
    sqlx::query(
        "INSERT INTO registry_compat (modpack_id, from_node, to_node, edge_type, evidence) VALUES \
         ($1, $2, $3, 'mag_in_weapon', NULL), \
         ($1, $2, $4, 'character_default_cargo', 'TargetStorage=Vest/Pouch')",
    )
    .bind(modpack)
    .bind(MAGAZINE)
    .bind(RIFLE)
    .bind(CHARACTER)
    .execute(&state.pool)
    .await
    .expect("seed the registry compatibility edges");
    modpack
}

impl MissionsLibraryWorld {
    fn next(&self) -> u32 {
        self.counter.fetch_add(1, Ordering::Relaxed)
    }

    fn setup<'a>(core: &'a WorldCore) -> Setup<'a> {
        Setup {
            app: &core.app,
            author: core.actors.user(Role::MissionMaker).token.clone(),
            admin: core.actors.user(Role::Admin).token.clone(),
        }
    }

    /// A version body whose payload carries a slot role no other payload uses.
    fn version_body(&self, semver: &str) -> Value {
        version_body(semver, &format!("R{}x{}", self.namespace, self.next()))
    }

    fn faction_name(&self) -> String {
        format!("Route acceptance faction {}", self.next())
    }

    /// A registry query naming the world's modpack, followed by `extra`.
    fn registry(&self, extra: &str) -> Fixture {
        Fixture::new().query(format!("modpack={}{extra}", self.modpack))
    }

    /// The registry fixture with the `If-None-Match` of the current ETag.
    async fn not_modified(&self, core: &WorldCore, route: &str, actor: Actor) -> Fixture {
        let query = format!("modpack={}", self.modpack);
        let mut first = Outgoing::new("GET", format!("{route}?{query}"));
        first.bearer = core.actors.bearer(actor);
        let received = send(&core.app, &first).await;
        let etag = received
            .header("etag")
            .unwrap_or_else(|| panic!("{route} answers an ETag: {}", received.excerpt()))
            .to_string();
        Fixture::new().query(query).header("if-none-match", etag)
    }
}

fn version_body(semver: &str, role: &str) -> Value {
    let payload =
        COMPILABLE_EDITOR_PAYLOAD.replace(r#""role":"SL""#, &format!(r#""role":"{role}""#));
    let payload: Value = serde_json::from_str(&payload).expect("the compilable payload is JSON");
    json!({"semver": semver, "payload": payload, "editor_notes": "Route acceptance"})
}

impl PartWorld for MissionsLibraryWorld {
    fn configure(config: &mut Config) {
        config.mission_version_max_body_bytes = VERSION_BODY_LIMIT;
    }

    async fn build(state: &mut AppState, actors: &Actors) -> Self {
        let app = router(state.clone());
        let setup = Setup {
            app: &app,
            author: actors.user(Role::MissionMaker).token.clone(),
            admin: actors.user(Role::Admin).token.clone(),
        };
        let namespace = actors.namespace;
        let (live, _) = setup.draft().await;
        let live_body = version_body("0.2.0", &format!("L{namespace}"));
        let live_version = setup.save_version(live, &live_body).await;
        let live_artifact = setup.submit(state, live).await;
        setup.approve(live, live_artifact).await;
        let (pending, _) = setup.draft().await;
        let pending_body = version_body("0.2.0", &format!("P{namespace}"));
        setup.save_version(pending, &pending_body).await;
        let pending_artifact = setup.submit(state, pending).await;
        let (draft, draft_version) = setup.draft().await;
        let faction = setup
            .faction(&faction_document("Route acceptance faction A"))
            .await;
        let sibling_faction_name = "Route acceptance faction B".to_string();
        setup
            .faction(&faction_document(&sibling_faction_name))
            .await;
        let modpack = seed_registry(state, namespace).await;
        MissionsLibraryWorld {
            live,
            live_version,
            live_artifact,
            pending,
            pending_artifact,
            draft,
            draft_version,
            faction,
            sibling_faction_name,
            modpack,
            counter: AtomicU32::new(1),
            namespace,
        }
    }

    async fn fixture(&self, core: &WorldCore, key: &str, actor: Actor) -> Option<Fixture> {
        let setup = Self::setup(core);
        let fixture = match key {
            "GET /api/v1/missions/{id}"
            | "POST /api/v1/missions/{id}/bookmark"
            | "DELETE /api/v1/missions/{id}/bookmark"
            | "GET /api/v1/missions/{id}/armory"
            | "GET /api/v1/missions/{id}/export" => {
                Fixture::new().param("id", self.live.to_string())
            }
            "draft-mission" => Fixture::new().param("id", self.draft.to_string()),
            "deleted-mission" => {
                let (mission, _) = setup.draft().await;
                setup.delete(format!("/api/v1/missions/{mission}")).await;
                Fixture::new().param("id", mission.to_string())
            }
            "library-draft-search" => {
                let title = format!("library-probe-{}", Uuid::new_v4().simple());
                setup.draft_titled(&title).await;
                Fixture::new().query(format!("q={title}"))
            }
            "POST /api/v1/missions" => Fixture::new().body(json!({
                "title": "Route acceptance mission", "terrain": "everon", "game_mode": "pve_coop",
                "max_players": 16, "weather": "clear", "time_of_day": "06:30"
            })),
            "PATCH /api/v1/missions/{id}" => {
                let (mission, _) = setup.draft().await;
                Fixture::new()
                    .param("id", mission.to_string())
                    .body(json!({"briefing": "Route acceptance briefing", "max_players": 24}))
            }
            "live-mission-change" => Fixture::new()
                .param("id", self.live.to_string())
                .body(json!({"status": "draft"})),
            "DELETE /api/v1/missions/{id}" => {
                let (mission, _) = setup.draft().await;
                Fixture::new().param("id", mission.to_string())
            }
            "PUT /api/v1/missions/{id}/armory" => {
                let (mission, _) = setup.draft().await;
                Fixture::new()
                    .param("id", mission.to_string())
                    .body(json!({"items": [
                        {"faction": "BLUFOR", "category": "Rifles", "item_name": "M16A2",
                         "quantity": 12, "icon": "", "sort_order": 1},
                        {"faction": "BLUFOR", "category": "Launchers", "item_name": "M72 LAW",
                         "quantity": null, "sort_order": 2}
                    ]}))
            }
            "POST /api/v1/missions/{id}/versions" => {
                let (mission, _) = setup.draft().await;
                Fixture::new()
                    .param("id", mission.to_string())
                    .body(self.version_body("1.0.0"))
            }
            "version-duplicate" => {
                let (mission, _) = setup.draft().await;
                let body = self.version_body("1.0.0");
                setup.save_version(mission, &body).await;
                Fixture::new().param("id", mission.to_string()).body(body)
            }
            "GET /api/v1/missions/{id}/versions/{vid}" => Fixture::new()
                .param("id", self.live.to_string())
                .param("vid", self.live_version.to_string()),
            "draft-version" | "POST /api/v1/missions/{id}/versions/{vid}/set-current" => {
                Fixture::new()
                    .param("id", self.draft.to_string())
                    .param("vid", self.draft_version.to_string())
            }
            "foreign-version" => Fixture::new()
                .param("id", self.draft.to_string())
                .param("vid", self.live_version.to_string()),
            "GET /api/v1/missions/{id}/artifacts/{artifact_id}/workspace" => Fixture::new()
                .param("id", self.pending.to_string())
                .param("artifact_id", self.pending_artifact.to_string()),
            "live-workspace" => Fixture::new()
                .param("id", self.live.to_string())
                .param("artifact_id", self.live_artifact.to_string()),
            "foreign-artifact" => Fixture::new()
                .param("id", self.pending.to_string())
                .param("artifact_id", self.live_artifact.to_string()),
            "POST /api/v1/factions" => Fixture::new().body(faction_document(&self.faction_name())),
            "faction-name-taken" => {
                Fixture::new().body(faction_document(&self.sibling_faction_name))
            }
            "GET /api/v1/factions/{id}" => Fixture::new().param("id", self.faction.to_string()),
            "PUT /api/v1/factions/{id}" => Fixture::new()
                .param("id", self.faction.to_string())
                .body(faction_document(&self.faction_name())),
            "faction-rename-clash" => Fixture::new()
                .param("id", self.faction.to_string())
                .body(faction_document(&self.sibling_faction_name)),
            "DELETE /api/v1/factions/{id}" => {
                let faction = setup.faction(&faction_document(&self.faction_name())).await;
                Fixture::new().param("id", faction.to_string())
            }
            "faction-deleted" => {
                let faction = setup.faction(&faction_document(&self.faction_name())).await;
                setup.delete(format!("/api/v1/factions/{faction}")).await;
                Fixture::new().param("id", faction.to_string())
            }
            "GET /api/v1/registry" | "GET /api/v1/registry/compat" => self.registry(""),
            "registry-limit-text" => self.registry("&limit=x"),
            "registry-page-of-one" => self.registry("&limit=1"),
            "registry-limit-over" | "compat-limit-over" => self.registry("&limit=100000"),
            "registry-unknown-modpack" => {
                Fixture::new().query(format!("modpack={}", Uuid::new_v4()))
            }
            "registry-not-modified" => self.not_modified(core, "/api/v1/registry", actor).await,
            "compat-offset-text" => self.registry("&limit=1&offset=first"),
            "compat-mag-edges" => self.registry("&edge_type=mag_in_weapon"),
            "compat-cargo-defaults" => self.registry("&view=cargo_defaults"),
            "compat-not-modified" => {
                self.not_modified(core, "/api/v1/registry/compat", actor)
                    .await
            }
            _ => return None,
        };
        Some(fixture)
    }
}
