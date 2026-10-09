//! The missions reviews part's world: missions under review and live, a deployment in flight on
//! the mod runtime's server, a registered fleet scenario, and fresh missions, servers and
//! deployments per probe for the routes that consume them.
//!
//! **Role:** implements [`PartWorld`] for mission submission, the review thread and its
//! artifacts, the approval queue, the website and in-game deployment routes, and the fleet
//! scenario registry.
//!
//! **Position:** mounted by `tests/route_acceptance_missions_reviews.rs` with `#[path]`; its
//! specs are `specs/missions_reviews.rs`. Every row is made through the real routes (mission
//! creation, version save, submission, approval, deployment request, cancellation) except the
//! servers, a demoted author and a banned account, which are plain rows.
//!
//! **Signals & state:** a counter that names the fresh missions, servers and terrain keys; the
//! ids of the rows `build` seeds.
//!
//! **Invariants:** the primary mission maker authors every mission; the guest-owned mission is
//! one whose author was demoted to guest after submitting it. The mod runtime's server always
//! runs a deployment of the live mission's approved artifact when a probe reads it, and has none
//! in flight when a probe relays a new request; consuming routes (submission, decisions,
//! deployment requests and cancellations, scenario removal) act on rows minted for that probe.

use std::sync::atomic::{AtomicU32, Ordering};

use api::router::router;
use api_state::AppState;
use axum::Router;
use axum::http::StatusCode;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::COMPILABLE_EDITOR_PAYLOAD;
use crate::route_acceptance_support::actors::Actors;
use crate::route_acceptance_support::requests::{Outgoing, send};
use crate::route_acceptance_support::spec::{Actor, Role};
use crate::route_acceptance_support::world::{Fixture, PartWorld, WorldCore};

/// The terrain every world mission is authored on; the world registers its fleet scenario.
const TERRAIN: &str = "everon";
/// A scenario header resource in the registry's `{16 uppercase hex}path.conf` form.
const SCENARIO_ID: &str = "{0123456789ABCDEF}Missions/RouteAcceptance.conf";
const COMMENT: &str = "Route acceptance review comment";
const REJECTION: &str = "The briefing needs a medic slot";

/// A mission with the artifact a review of it decides.
#[derive(Clone, Copy)]
struct Reviewed {
    mission: Uuid,
    artifact: Uuid,
}

/// The missions reviews world.
pub(crate) struct MissionsReviewsWorld {
    /// Authored by the primary mission maker, pending approval.
    under_review: Reviewed,
    /// Pending approval; its author is the primary guest (a demoted mission maker).
    guest_owned: Reviewed,
    /// Approved into the live library; `artifact` is the approved artifact.
    live: Reviewed,
    /// The deployment `build` requests on the mod runtime's server.
    deployment: Uuid,
    /// The primary administrator's linked Arma identity.
    administrator_arma_id: String,
    serial: AtomicU32,
}

/// The rows every setup call needs: a router, the pool, the author's and the administrator's
/// bearers, and the namespace that keeps names unique across the binary's worlds.
struct Setup<'a> {
    app: &'a Router,
    pool: &'a PgPool,
    maker: &'a str,
    admin: &'a str,
    namespace: u8,
}

fn uuid_at(body: &Value, field: &str) -> Uuid {
    body[field]
        .as_str()
        .and_then(|raw| raw.parse().ok())
        .unwrap_or_else(|| panic!("expected a UUID at {field} in {body}"))
}

async fn expect(app: &Router, outgoing: Outgoing, status: StatusCode, what: &str) -> Value {
    let received = send(app, &outgoing).await;
    assert_eq!(received.status, status, "{what}: {}", received.excerpt());
    received.json().unwrap_or(Value::Null)
}

async fn arma_id_of(pool: &PgPool, discord_id: &str) -> String {
    let linked: Option<String> =
        sqlx::query_scalar("SELECT arma_id FROM users WHERE discord_id = $1")
            .bind(discord_id)
            .fetch_one(pool)
            .await
            .expect("read the account's Arma identity");
    linked.unwrap_or_else(|| panic!("account {discord_id} has no linked Arma identity"))
}

impl Setup<'_> {
    fn of<'a>(app: &'a Router, pool: &'a PgPool, actors: &'a Actors) -> Setup<'a> {
        Setup {
            app,
            pool,
            maker: &actors.user(Role::MissionMaker).token,
            admin: &actors.user(Role::Admin).token,
            namespace: actors.namespace,
        }
    }

    /// A draft mission as created: its initial version places no slots.
    async fn draft(&self, label: &str) -> Uuid {
        let title = format!("Route acceptance {} {label}", self.namespace);
        let body = json!({"title": title, "terrain": TERRAIN, "game_mode": "pve_coop",
            "max_players": 16});
        let request = Outgoing::new("POST", "/api/v1/missions")
            .bearer(self.maker)
            .json(&body);
        uuid_at(
            &expect(self.app, request, StatusCode::CREATED, "create a mission").await,
            "id",
        )
    }

    /// A draft mission whose current version compiles.
    async fn saved_draft(&self, label: &str) -> Uuid {
        let mission = self.draft(label).await;
        let payload: Value =
            serde_json::from_str(COMPILABLE_EDITOR_PAYLOAD).expect("the payload is JSON");
        let request = Outgoing::new("POST", format!("/api/v1/missions/{mission}/versions"))
            .bearer(self.maker)
            .json(&json!({"semver": "0.2.0", "payload": payload}));
        expect(self.app, request, StatusCode::CREATED, "save a version").await;
        mission
    }

    /// A mission submitted by its author, with the artifact its pending review decides.
    async fn submitted(&self, label: &str) -> Reviewed {
        let mission = self.saved_draft(label).await;
        let request =
            Outgoing::new("POST", format!("/api/v1/missions/{mission}/submit")).bearer(self.maker);
        expect(self.app, request, StatusCode::OK, "submit a mission").await;
        let artifact: Uuid = sqlx::query_scalar(
            "SELECT artifact_id FROM mission_reviews WHERE mission_id = $1 AND state = 'pending'",
        )
        .bind(mission)
        .fetch_one(self.pool)
        .await
        .expect("a submission opens a review");
        Reviewed { mission, artifact }
    }

    /// A submitted mission an administrator approved into the live library.
    async fn approved(&self, label: &str) -> Reviewed {
        let reviewed = self.submitted(label).await;
        let request = Outgoing::new(
            "POST",
            format!("/api/v1/approvals/{}/approve", reviewed.mission),
        )
        .bearer(self.admin)
        .json(&json!({"artifact_id": reviewed.artifact}));
        expect(self.app, request, StatusCode::OK, "approve a mission").await;
        reviewed
    }

    /// Register the scenario the fleet runs for `terrain`.
    async fn register_scenario(&self, terrain: &str) {
        let request = Outgoing::new("PUT", format!("/api/v1/fleet/scenarios/{terrain}"))
            .bearer(self.admin)
            .json(&json!({"scenario_id": SCENARIO_ID, "display_name": "Route Acceptance"}));
        expect(
            self.app,
            request,
            StatusCode::OK,
            "register a fleet scenario",
        )
        .await;
    }

    /// A registered server with no required modpack.
    async fn server(&self, name: &str, active: bool) -> Uuid {
        sqlx::query_scalar(
            "INSERT INTO servers (name, ip, port, is_active) VALUES ($1, '127.0.0.1'::inet, 2302, $2) \
             RETURNING id",
        )
        .bind(format!("route-acceptance-{}-{name}", self.namespace))
        .bind(active)
        .fetch_one(self.pool)
        .await
        .expect("register a server")
    }

    /// An administrator's deployment of `live` on `server`, in flight.
    async fn deploy(&self, server: Uuid, live: Reviewed) -> Uuid {
        let request = Outgoing::new("POST", format!("/api/v1/servers/{server}/deployments"))
            .bearer(self.admin)
            .json(&json!({"mission_id": live.mission, "artifact_id": live.artifact}));
        uuid_at(
            &expect(
                self.app,
                request,
                StatusCode::ACCEPTED,
                "request a deployment",
            )
            .await,
            "id",
        )
    }

    async fn cancel(&self, server: Uuid, deployment: Uuid) {
        let uri = format!("/api/v1/servers/{server}/deployments/{deployment}/cancel");
        let request = Outgoing::new("POST", uri).bearer(self.admin);
        expect(self.app, request, StatusCode::OK, "cancel a deployment").await;
    }

    async fn in_flight(&self, server: Uuid) -> Option<Uuid> {
        sqlx::query_scalar(
            "SELECT id FROM mission_deployments WHERE server_id = $1 AND state = 'requested'",
        )
        .bind(server)
        .fetch_optional(self.pool)
        .await
        .expect("read the deployment in flight")
    }
}

fn mission(id: Uuid) -> Fixture {
    Fixture::new().param("id", id.to_string())
}

fn artifact(reviewed: Reviewed) -> Fixture {
    mission(reviewed.mission).param("artifact_id", reviewed.artifact.to_string())
}

fn on_server(server: Uuid) -> Fixture {
    Fixture::new().param("id", server.to_string())
}

impl MissionsReviewsWorld {
    fn setup<'a>(&self, core: &'a WorldCore) -> Setup<'a> {
        Setup::of(&core.app, &core.state.pool, &core.actors)
    }

    fn label(&self, kind: &str) -> String {
        format!("{kind} {}", self.serial.fetch_add(1, Ordering::Relaxed))
    }

    /// The mission a review read addresses: the caller's own when the caller is the guest.
    fn readable(&self, actor: Actor) -> Reviewed {
        if matches!(actor, Actor::User(Role::Guest)) {
            self.guest_owned
        } else {
            self.under_review
        }
    }

    fn selection(&self, reviewed: Reviewed) -> Value {
        json!({"mission_id": reviewed.mission, "artifact_id": reviewed.artifact})
    }

    /// A decision on a freshly submitted mission; `extra` completes the body.
    async fn decision(&self, core: &WorldCore, extra: Value) -> Fixture {
        let reviewed = self.setup(core).submitted(&self.label("decision")).await;
        let mut body = json!({"artifact_id": reviewed.artifact});
        if let (Some(body), Some(extra)) = (body.as_object_mut(), extra.as_object()) {
            body.extend(extra.clone());
        }
        mission(reviewed.mission).body(body)
    }

    /// A decision on a saved draft that was never submitted.
    async fn decision_on_draft(&self, core: &WorldCore, extra: Value) -> Fixture {
        let draft = self.setup(core).saved_draft(&self.label("draft")).await;
        let mut body = json!({"artifact_id": Uuid::new_v4()});
        if let (Some(body), Some(extra)) = (body.as_object_mut(), extra.as_object()) {
            body.extend(extra.clone());
        }
        mission(draft).body(body)
    }

    /// Make sure the mod runtime's server runs a deployment of the live artifact.
    async fn ensure_in_flight(&self, core: &WorldCore) {
        let setup = self.setup(core);
        let server = core.actors.machines.server;
        if setup.in_flight(server).await.is_none() {
            setup.deploy(server, self.live).await;
        }
    }

    /// Make sure the mod runtime's server has no deployment in flight.
    async fn clear_in_flight(&self, core: &WorldCore) {
        let setup = self.setup(core);
        let server = core.actors.machines.server;
        if let Some(deployment) = setup.in_flight(server).await {
            setup.cancel(server, deployment).await;
        }
    }

    /// An in-game request relayed for `arma_id`, with nothing in flight on the server.
    async fn relayed(&self, core: &WorldCore, reviewed: Reviewed, arma_id: &str) -> Fixture {
        self.clear_in_flight(core).await;
        let mut body = self.selection(reviewed);
        body["requested_by_arma_id"] = json!(arma_id);
        Fixture::new().body(body)
    }

    /// A fresh administrator, linked in game, banned after linking.
    async fn banned_administrator_arma_id(&self, core: &WorldCore) -> String {
        let account = core.actors.fresh_user(&core.state, Role::Admin, true).await;
        sqlx::query("UPDATE users SET is_banned = true WHERE discord_id = $1")
            .bind(&account.discord_id)
            .execute(&core.state.pool)
            .await
            .expect("ban the administrator");
        arma_id_of(&core.state.pool, &account.discord_id).await
    }

    async fn deployment_fixture(&self, core: &WorldCore, key: &str) -> Option<Fixture> {
        let setup = self.setup(core);
        let runtime_server = core.actors.machines.server;
        let fixture = match key {
            "POST /api/v1/servers/{id}/deployments" => {
                let server = setup.server(&self.label("server"), true).await;
                on_server(server).body(self.selection(self.live))
            }
            "server-in-flight" => {
                self.ensure_in_flight(core).await;
                on_server(runtime_server).body(self.selection(self.live))
            }
            "inactive-server" => {
                let server = setup.server(&self.label("inactive server"), false).await;
                on_server(server).body(self.selection(self.live))
            }
            "unapproved-artifact" => {
                let server = setup.server(&self.label("server"), true).await;
                on_server(server).body(self.selection(self.under_review))
            }
            "GET /api/v1/servers/{id}/deployments" => on_server(runtime_server),
            "GET /api/v1/servers/{id}/deployments/{deploymentId}" => {
                on_server(runtime_server).param("deploymentId", self.deployment.to_string())
            }
            "deployment-elsewhere" => on_server(core.actors.machines.other_server)
                .param("deploymentId", self.deployment.to_string()),
            "POST /api/v1/servers/{id}/deployments/{deploymentId}/cancel"
            | "cancelled-deployment" => {
                let server = setup.server(&self.label("server"), true).await;
                let deployment = setup.deploy(server, self.live).await;
                if key == "cancelled-deployment" {
                    setup.cancel(server, deployment).await;
                }
                on_server(server).param("deploymentId", deployment.to_string())
            }
            "GET /api/v1/game-runtime/deployment" => {
                self.ensure_in_flight(core).await;
                Fixture::new()
            }
            "GET /api/v1/game-runtime/artifacts/{artifactId}" => {
                self.ensure_in_flight(core).await;
                Fixture::new().param("artifactId", self.live.artifact.to_string())
            }
            "undeployed-artifact" => {
                Fixture::new().param("artifactId", self.under_review.artifact.to_string())
            }
            "POST /api/v1/game-runtime/deployments" => {
                self.relayed(core, self.live, &self.administrator_arma_id)
                    .await
            }
            "in-game-enlisted" => {
                let enlisted = &core.actors.user(Role::Enlisted).discord_id;
                let arma_id = arma_id_of(&core.state.pool, enlisted).await;
                self.relayed(core, self.live, &arma_id).await
            }
            "in-game-banned" => {
                let arma_id = self.banned_administrator_arma_id(core).await;
                self.relayed(core, self.live, &arma_id).await
            }
            "in-game-in-flight" => {
                self.ensure_in_flight(core).await;
                let mut body = self.selection(self.live);
                body["requested_by_arma_id"] = json!(self.administrator_arma_id);
                Fixture::new().body(body)
            }
            "in-game-unapproved" => {
                self.relayed(core, self.under_review, &self.administrator_arma_id)
                    .await
            }
            _ => return None,
        };
        Some(fixture)
    }
}

impl PartWorld for MissionsReviewsWorld {
    async fn build(state: &mut AppState, actors: &Actors) -> Self {
        let app = router(state.clone());
        let setup = Setup::of(&app, &state.pool, actors);
        setup.register_scenario(TERRAIN).await;
        let under_review = setup.submitted("under review").await;
        let guest_owned = setup.submitted("guest owned").await;
        sqlx::query("UPDATE missions SET author_id = $2 WHERE id = $1")
            .bind(guest_owned.mission)
            .bind(&actors.user(Role::Guest).discord_id)
            .execute(&state.pool)
            .await
            .expect("hand the mission to its demoted author");
        let live = setup.approved("live").await;
        let deployment = setup.deploy(actors.machines.server, live).await;
        let administrator_arma_id =
            arma_id_of(&state.pool, &actors.user(Role::Admin).discord_id).await;
        MissionsReviewsWorld {
            under_review,
            guest_owned,
            live,
            deployment,
            administrator_arma_id,
            serial: AtomicU32::new(1),
        }
    }

    async fn fixture(&self, core: &WorldCore, key: &str, actor: Actor) -> Option<Fixture> {
        let setup = self.setup(core);
        let fixture = match key {
            "POST /api/v1/missions/{id}/submit" => {
                mission(setup.saved_draft(&self.label("submission")).await)
            }
            "draft-without-slots" => mission(setup.draft(&self.label("empty draft")).await),
            "mission-under-review" => mission(self.under_review.mission),
            "live-mission" => mission(self.live.mission),
            "GET /api/v1/missions/{id}/reviews" => mission(self.readable(actor).mission),
            "POST /api/v1/missions/{id}/review-comments" => mission(self.under_review.mission)
                .body(json!({"body": COMMENT, "artifact_id": self.under_review.artifact})),
            "live-comment" => mission(self.live.mission).body(json!({"body": COMMENT})),
            "comment-foreign-artifact" => mission(self.under_review.mission)
                .body(json!({"body": COMMENT, "artifact_id": self.live.artifact})),
            "GET /api/v1/missions/{id}/artifacts/{artifact_id}"
            | "GET /api/v1/missions/{id}/artifacts/{artifact_id}/document" => {
                artifact(self.readable(actor))
            }
            "live-artifact" => artifact(self.live),
            "artifact-elsewhere" => artifact(Reviewed {
                mission: self.under_review.mission,
                artifact: self.live.artifact,
            }),
            "POST /api/v1/approvals/{id}/approve" => self.decision(core, json!({})).await,
            "POST /api/v1/approvals/{id}/reject" => {
                self.decision(core, json!({"reason": REJECTION})).await
            }
            "approve-draft" => self.decision_on_draft(core, json!({})).await,
            "reject-draft" => {
                self.decision_on_draft(core, json!({"reason": REJECTION}))
                    .await
            }
            "PUT /api/v1/fleet/scenarios/{terrainKey}" => Fixture::new()
                .param(
                    "terrainKey",
                    format!("route_acceptance_{}", core.actors.namespace),
                )
                .body(json!({"scenario_id": SCENARIO_ID, "display_name": "Route Acceptance"})),
            "DELETE /api/v1/fleet/scenarios/{terrainKey}" => {
                let key = format!(
                    "route_acceptance_removal_{}_{}",
                    core.actors.namespace,
                    self.serial.fetch_add(1, Ordering::Relaxed)
                );
                setup.register_scenario(&key).await;
                Fixture::new().param("terrainKey", key)
            }
            _ => return self.deployment_fixture(core, key).await,
        };
        Some(fixture)
    }
}
