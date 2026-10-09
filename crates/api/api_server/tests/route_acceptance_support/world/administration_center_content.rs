//! The administration, command center and community content part's world: seeded content rows,
//! a stand-in Discord webhook, an imported equipment dataset, and fresh accounts and rows per
//! probe for the routes that consume them.
//!
//! **Role:** implements [`PartWorld`] for `specs/administration_center_content.rs`: the
//! personnel, discipline, membership grace, role resync and audit log routes, the command
//! center reads, the announcement, upload, modpack, vehicle database and wiki routes, and the
//! development-only equipment data viewer reads.
//!
//! **Position:** mounted by `tests/route_acceptance_administration_center_content.rs` with
//! `#[path]`; builds request bodies with the `content_support` (multipart forms, vehicle bodies)
//! and `wiki_support` (save bodies, unique slugs) helpers that binary declares, and imports the
//! committed equipment exports under `tests/fixtures/equipment_data_viewer/` with the
//! production importer.
//!
//! **Signals & state:** a local HTTP server answering the Discord webhook post (aborted when the
//! world drops), and a private equipment data directory (removed when the world drops).
//!
//! **Invariants:** every row a route consumes (a ban target, an announcement or modpack to
//! delete, a vehicle to retire, a page to revise) is minted per probe, so no probe sees
//! another's effect; the shared rows (a published and a draft announcement, a modpack, a
//! vehicle, a wiki page at revision 1) are only ever read or rewritten in place; the world never
//! bans, demotes or deletes an actor account.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use api_configuration::configuration::Config;
use api_discord::discord_webhook::WebhookService;
use api_equipment_datasets::EquipmentDataService;
use api_equipment_datasets::importing::generation_import;
use api_server::router::router;
use api_state::AppState;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::content_support::{MultipartPart, multipart_body, png_bytes, vehicle_body};
use crate::route_acceptance_support::actors::Actors;
use crate::route_acceptance_support::requests::{Outgoing, send};
use crate::route_acceptance_support::spec::{Actor, Role};
use crate::route_acceptance_support::specs::administration_center_content::equipment_query;
use crate::route_acceptance_support::world::{Fixture, PartWorld, WorldCore};
use crate::wiki_support::{save_body, unique_slug};

/// The committed importer inputs of the equipment data viewer.
fn equipment_fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/equipment_data_viewer")
}

/// The administration, command center and community content world.
pub(crate) struct AdministrationCenterContentWorld {
    webhook: tokio::task::JoinHandle<()>,
    equipment_dir: PathBuf,
    published_announcement: Uuid,
    draft_announcement: Uuid,
    modpack: Uuid,
    vehicle: Uuid,
    wiki_slug: String,
}

impl Drop for AdministrationCenterContentWorld {
    fn drop(&mut self) {
        self.webhook.abort();
        let _ = std::fs::remove_dir_all(&self.equipment_dir);
    }
}

/// The stand-in for the Discord webhook: every post creates message `route-acceptance`.
fn stand_in_webhook() -> Router {
    Router::new().route(
        "/webhook",
        post(|| async { Json(json!({"id": "route-acceptance"})) }),
    )
}

/// `method uri` as `token` with `body`; panics unless the answer is a success, and answers its
/// JSON body (`null` for an empty one).
async fn succeed(app: &Router, token: &str, method: &str, uri: &str, body: Option<Value>) -> Value {
    let mut request = Outgoing::new(method, uri).bearer(token);
    if let Some(body) = &body {
        request = request.json(body);
    }
    let received = send(app, &request).await;
    assert!(
        received.status.is_success(),
        "{method} {uri}: {} {}",
        received.status,
        received.excerpt()
    );
    received.json().unwrap_or(Value::Null)
}

/// The `id` of a created row.
fn id_of(body: &Value) -> Uuid {
    body["id"]
        .as_str()
        .and_then(|id| Uuid::parse_str(id).ok())
        .unwrap_or_else(|| panic!("a created row names its id: {body}"))
}

fn announcement_body(status: &str) -> Value {
    json!({"title": "Route acceptance notice", "body": "Route acceptance body text.",
        "tag": "update", "status": status})
}

fn modpack_body() -> Value {
    json!({"name": format!("Route acceptance {}", Uuid::new_v4().simple()), "version": "1.0.0",
        "total_size_bytes": 1024, "workshop_url": "https://example.com/route-acceptance",
        "mods": [{"name": "Route Acceptance Core", "workshop_id": "5965550F24A0C152"}]})
}

fn wiki_body(base_revision: Option<i64>) -> Value {
    save_body(
        "Route acceptance page",
        "Route acceptance page body.",
        base_revision,
    )
}

/// A multipart form holding one field named `name`, a file field when `file_name` is set.
fn form(name: &str, file_name: Option<&str>, bytes: &[u8]) -> Fixture {
    let (content_type, body) = multipart_body(&[MultipartPart {
        name,
        file_name,
        bytes,
    }]);
    Fixture::new().raw_body(body, content_type)
}

/// Imports the committed diagnostic and gameplay exports into `state`'s equipment catalogs.
async fn import_equipment(state: &AppState) {
    let data_dir = PathBuf::from(&state.cfg.equipment_data_dir);
    let diagnostic = Arc::new(EquipmentDataService::new(
        &data_dir,
        Some(equipment_fixtures().join("diagnostic_export")),
    ));
    generation_import::poll(diagnostic)
        .await
        .expect("the importer publishes the committed diagnostic export");
    generation_import::initialize(&state.equipment_data.diagnostic)
        .await
        .expect("the diagnostic catalog activates the imported generation");
    generation_import::initialize(&state.equipment_data.gameplay)
        .await
        .expect("the gameplay catalog starts empty");
    generation_import::poll(state.equipment_data.gameplay.clone())
        .await
        .expect("the importer publishes the committed gameplay export");
}

impl AdministrationCenterContentWorld {
    fn admin(core: &WorldCore) -> String {
        core.actors.user(Role::Admin).token.clone()
    }

    async fn created(core: &WorldCore, uri: &str, body: Value) -> Uuid {
        id_of(&succeed(&core.app, &Self::admin(core), "POST", uri, Some(body)).await)
    }

    /// A fresh enlisted account with verified guild membership.
    async fn fresh_account(core: &WorldCore) -> String {
        core.actors
            .fresh_user(&core.state, Role::Enlisted, true)
            .await
            .discord_id
    }

    /// A fresh account that is already banned.
    async fn banned_account(core: &WorldCore) -> String {
        let account = Self::fresh_account(core).await;
        sqlx::query("UPDATE users SET is_banned = true WHERE discord_id = $1")
            .bind(&account)
            .execute(&core.state.pool)
            .await
            .expect("ban the unban fixture account");
        account
    }

    /// A fresh page at revision 1.
    async fn fresh_page(core: &WorldCore) -> String {
        let slug = unique_slug("route-acceptance");
        let uri = format!("/api/v1/wiki/{slug}");
        succeed(
            &core.app,
            &Self::admin(core),
            "PUT",
            &uri,
            Some(wiki_body(None)),
        )
        .await;
        slug
    }
}

impl PartWorld for AdministrationCenterContentWorld {
    fn configure(config: &mut Config) {
        let private = std::env::temp_dir().join(format!(
            "route-acceptance-equipment-{}",
            Uuid::new_v4().simple()
        ));
        config.equipment_data_dir = private.display().to_string();
        config.equipment_export_source_dir = Some(
            equipment_fixtures()
                .join("export_source")
                .display()
                .to_string(),
        );
    }

    async fn build(state: &mut AppState, actors: &Actors) -> Self {
        import_equipment(state).await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind the stand-in webhook");
        let base = format!(
            "http://{}",
            listener.local_addr().expect("stand-in address")
        );
        let webhook = tokio::spawn(async move {
            axum::serve(listener, stand_in_webhook())
                .await
                .expect("serve the stand-in webhook");
        });
        state.webhook = Arc::new(WebhookService::new(format!("{base}/webhook")));

        let app = router(state.clone());
        let admin = actors.user(Role::Admin).token.clone();
        let create = |uri: &'static str, body: Value| {
            let app = app.clone();
            let admin = admin.clone();
            async move { id_of(&succeed(&app, &admin, "POST", uri, Some(body)).await) }
        };
        let published_announcement =
            create("/api/v1/cms/announcements", announcement_body("published")).await;
        let draft_announcement =
            create("/api/v1/cms/announcements", announcement_body("draft")).await;
        let modpack = create("/api/v1/modpacks", modpack_body()).await;
        let current = create("/api/v1/modpacks", modpack_body()).await;
        let set_current = format!("/api/v1/modpacks/{current}/set-current");
        succeed(&app, &admin, "POST", &set_current, None).await;
        let vehicle = create("/api/v1/vehicle-database", vehicle_body("Route acceptance")).await;
        let wiki_slug = unique_slug("route-acceptance");
        let page = format!("/api/v1/wiki/{wiki_slug}");
        succeed(&app, &admin, "PUT", &page, Some(wiki_body(None))).await;
        AdministrationCenterContentWorld {
            webhook,
            equipment_dir: PathBuf::from(&state.cfg.equipment_data_dir),
            published_announcement,
            draft_announcement,
            modpack,
            vehicle,
            wiki_slug,
        }
    }

    async fn fixture(&self, core: &WorldCore, key: &str, _actor: Actor) -> Option<Fixture> {
        if let Some(query) = equipment_query(key) {
            return Some(match query {
                "" => Fixture::new(),
                query => Fixture::new().query(query),
            });
        }
        let peer = core.actors.peer(Role::Enlisted).discord_id.clone();
        let fixture = match key {
            "PATCH /api/v1/admin/users/{discordId}" => Fixture::new()
                .param("discordId", peer)
                .body(json!({"role": "leader"})),
            "POST /api/v1/admin/users/{discordId}/ban" => Fixture::new()
                .param("discordId", Self::fresh_account(core).await)
                .body(json!({"reason": "Route acceptance ban"})),
            "DELETE /api/v1/admin/users/{discordId}/ban" => {
                Fixture::new().param("discordId", Self::banned_account(core).await)
            }
            "POST /api/v1/admin/users/{discordId}/warnings" => Fixture::new()
                .param("discordId", peer)
                .body(json!({"reason": "Route acceptance warning"})),
            "POST /api/v1/admin/users/{discordId}/membership-grace" => Fixture::new()
                .param("discordId", Self::fresh_account(core).await)
                .body(json!({"duration_hours": 4, "reason": "Route acceptance outage"})),
            "GET /api/v1/users/{discordId}/stats" => Fixture::new().param(
                "discordId",
                core.actors.user(Role::Enlisted).discord_id.clone(),
            ),
            "GET /api/v1/announcements/{id}"
            | "POST /api/v1/cms/announcements/{id}/push-discord" => {
                Fixture::new().param("id", self.published_announcement.to_string())
            }
            "draft-announcement" => Fixture::new().param("id", self.draft_announcement.to_string()),
            "POST /api/v1/cms/announcements" => Fixture::new().body(announcement_body("draft")),
            "PATCH /api/v1/cms/announcements/{id}" => Fixture::new()
                .param("id", self.draft_announcement.to_string())
                .body(json!({"title": "Route acceptance notice, revised"})),
            "DELETE /api/v1/cms/announcements/{id}" => {
                let id = Self::created(
                    core,
                    "/api/v1/cms/announcements",
                    announcement_body("draft"),
                )
                .await;
                Fixture::new().param("id", id.to_string())
            }
            "POST /api/v1/cms/uploads" => {
                form("file", Some("route-acceptance.png"), &png_bytes(64))
            }
            "upload-without-file" => form("caption", None, b"route acceptance"),
            "upload-text-file" => form("file", Some("route-acceptance.txt"), b"plain text"),
            "POST /api/v1/modpacks" => Fixture::new().body(modpack_body()),
            "PUT /api/v1/modpacks/{id}" => Fixture::new()
                .param("id", self.modpack.to_string())
                .body(modpack_body()),
            "POST /api/v1/modpacks/{id}/set-current" | "DELETE /api/v1/modpacks/{id}" => {
                let id = Self::created(core, "/api/v1/modpacks", modpack_body()).await;
                Fixture::new().param("id", id.to_string())
            }
            "GET /api/v1/vehicle-database/{id}" => {
                Fixture::new().param("id", self.vehicle.to_string())
            }
            "POST /api/v1/vehicle-database" => {
                Fixture::new().body(vehicle_body("Route acceptance"))
            }
            "PUT /api/v1/vehicle-database/{id}" => Fixture::new()
                .param("id", self.vehicle.to_string())
                .body(vehicle_body("Route acceptance replaced")),
            "PATCH /api/v1/vehicle-database/{id}" => Fixture::new()
                .param("id", self.vehicle.to_string())
                .body(json!({"primary_threat": "RPG"})),
            "DELETE /api/v1/vehicle-database/{id}" => {
                let body = vehicle_body("Route acceptance retired");
                let id = Self::created(core, "/api/v1/vehicle-database", body).await;
                Fixture::new().param("id", id.to_string())
            }
            "GET /api/v1/wiki/{slug}" | "GET /api/v1/wiki/{slug}/revisions" => {
                Fixture::new().param("slug", self.wiki_slug.clone())
            }
            "GET /api/v1/wiki/{slug}/revisions/{revision}" => Fixture::new()
                .param("slug", self.wiki_slug.clone())
                .param("revision", "1"),
            "PUT /api/v1/wiki/{slug}" => Fixture::new()
                .param("slug", unique_slug("route-acceptance"))
                .body(wiki_body(None)),
            "wiki-revise" => Fixture::new()
                .param("slug", Self::fresh_page(core).await)
                .body(wiki_body(Some(1))),
            "wiki-existing" => Fixture::new()
                .param("slug", self.wiki_slug.clone())
                .body(wiki_body(None)),
            _ => return None,
        };
        Some(fixture)
    }
}
