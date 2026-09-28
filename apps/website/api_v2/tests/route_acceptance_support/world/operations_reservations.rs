//! The operations reservations part's world: a fresh operation per reservation probe and one
//! event running on the world's server under an open runtime session.
//!
//! **Role:** implements [`PartWorld`] for the seat registration, seat assignment, squad hold,
//! waitlist promotion and game-runtime roster and deployment routes.
//!
//! **Position:** mounted by `tests/route_acceptance_operations_reservations.rs` with `#[path]`;
//! its specs are `specs/operations_reservations.rs`. The running event comes from
//! `event_eligibility_support` and `fleet_support` (a bound event, a recorded deployment of its
//! attachment and a runtime session reporting the deployed artifact); every other row is seeded
//! here or driven through the real routes.
//!
//! **Signals & state:** the running event (built once per world) and a counter that names each
//! player life the world deploys.
//!
//! **Invariants:** every reservation probe gets its own operation (event, attachment, seats), so
//! no probe sees another probe's registration, squad hold or waitlist entry; the primary enlisted
//! account holds seat 0 of the running attachment, so its lives are reservation lives; the peer
//! enlisted account deploys into the open seat 1; before a deployment probe the peer's earlier
//! life is ended, so the probe's decision is `allowed`.

use std::sync::atomic::{AtomicU32, Ordering};

use axum::Router;
use axum::http::StatusCode;
use serde_json::{Value, json};
use uuid::Uuid;
use website_api::core::application_state::AppState;
use website_api::core::http_router;

use crate::event_eligibility_support::{
    Actor as EligibilityActor, EventShape, Fixture as EligibilityFixture,
};
use crate::fleet_support::{bind_event, linked_arma, running_session, seed_deployment};
use crate::route_acceptance_support::actors::{Actors, UserActor};
use crate::route_acceptance_support::requests::{Outgoing, Received, send};
use crate::route_acceptance_support::spec::{Actor, Executor, Role};
use crate::route_acceptance_support::world::{Fixture, PartWorld, WorldCore};

/// The label the eligibility fixture prefixes to its own accounts.
const FIXTURE_SUITE: &str = "route_acceptance_operations_reservations";
/// The squad of every seat the world seeds.
const SQUAD: &str = "Alpha";
/// The runtime credential of the world's server.
const RUNTIME: Actor = Actor::Machine(Executor::ModRuntime);

/// One operation: an event, its single attachment and that attachment's seats.
struct Operation {
    event: Uuid,
    attachment: Uuid,
    seats: Vec<Uuid>,
}

/// The event running on the world's server.
struct RunningEvent {
    /// Keeps the eligibility fixture (and its pool) alive for the world's lifetime.
    _fixture: EligibilityFixture,
    event: Uuid,
    attachment: Uuid,
    seats: Vec<Uuid>,
    session: Uuid,
}

/// The operations reservations world.
pub struct OperationsReservationsWorld {
    running: RunningEvent,
    /// An event bound to the world's server with no deployment in effect.
    idle_event: Uuid,
    /// An event bound to no server.
    unbound_event: Uuid,
    lives: AtomicU32,
}

/// Send one request with `bearer`, and a JSON body when `body` is given.
async fn call(
    app: &Router,
    bearer: &str,
    method: &str,
    uri: &str,
    body: Option<&Value>,
) -> Received {
    let mut outgoing = Outgoing::new(method, uri).bearer(bearer);
    if let Some(body) = body {
        outgoing = outgoing.json(body);
    }
    send(app, &outgoing).await
}

/// Send one setup request and require a success status; answers the JSON body.
async fn require(
    app: &Router,
    bearer: &str,
    method: &str,
    uri: &str,
    body: Option<&Value>,
) -> Value {
    let received = call(app, bearer, method, uri, body).await;
    assert!(
        received.status.is_success(),
        "setup {method} {uri}: {} {}",
        received.status,
        received.excerpt()
    );
    received.json().unwrap_or(Value::Null)
}

/// A new event of `status` (no participant limit) with one live mission attached and `seats`
/// seats in squad [`SQUAD`].
async fn operation(state: &AppState, author: &str, status: &str, seats: usize) -> Operation {
    let event: Uuid = sqlx::query_scalar(
        "INSERT INTO events (name_override, start_time, status, max_slots, created_by, created_at)
         VALUES ('Route acceptance operation', clock_timestamp() + interval '3 days', $1::event_status,
             0, $2, clock_timestamp() - interval '1 hour')
         RETURNING id",
    )
    .bind(status)
    .bind(author)
    .fetch_one(&state.pool)
    .await
    .expect("seed an operation");
    let mission: Uuid = sqlx::query_scalar(
        "INSERT INTO missions (title, author_id, terrain, game_mode, max_players, status)
         VALUES ('Route acceptance mission', $1, 'everon', 'pve_coop', 32, 'live') RETURNING id",
    )
    .bind(author)
    .fetch_one(&state.pool)
    .await
    .expect("seed the operation's mission");
    let attachment: Uuid = sqlx::query_scalar(
        "INSERT INTO event_missions (event_id, mission_id, start_time)
         VALUES ($1, $2, clock_timestamp() + interval '3 days') RETURNING id",
    )
    .bind(event)
    .bind(mission)
    .fetch_one(&state.pool)
    .await
    .expect("attach the operation's mission");
    let mut seat_ids = Vec::with_capacity(seats);
    for index in 0..seats {
        seat_ids.push(
            sqlx::query_scalar(
                "INSERT INTO orbat_slots (event_mission_id, faction, squad, role, slot_index)
                 VALUES ($1, 'BLUFOR', $2, 'Rifleman', $3) RETURNING id",
            )
            .bind(attachment)
            .bind(SQUAD)
            .bind(index as i32)
            .fetch_one(&state.pool)
            .await
            .expect("seed the operation's seat"),
        );
    }
    Operation {
        event,
        attachment,
        seats: seat_ids,
    }
}

/// Queue `account` on the waitlist of `operation`'s attachment, an hour ago.
async fn queue(core: &WorldCore, operation: &Operation, account: &UserActor) {
    sqlx::query(
        "INSERT INTO event_registrations (event_mission_id, discord_id, reservation_state, queue_entered_at)
         VALUES ($1, $2, 'waitlisted', clock_timestamp() - interval '1 hour')",
    )
    .bind(operation.attachment)
    .bind(&account.discord_id)
    .execute(&core.state.pool)
    .await
    .expect("queue the waiting participant");
}

/// The Arma identity `common::access_token` links for `account`.
fn arma_of(account: &UserActor) -> String {
    linked_arma(&EligibilityActor {
        id: account.discord_id.clone(),
        token: String::new(),
    })
}

fn register_uri(attachment: Uuid) -> String {
    format!("/api/v1/event-missions/{attachment}/register")
}

fn squad_uri(attachment: Uuid, action: &str) -> String {
    format!("/api/v1/event-missions/{attachment}/squads/{action}")
}

fn seat_uri(operation: &Operation) -> String {
    format!(
        "/api/v1/event-missions/{}/slots/{}/assign",
        operation.attachment, operation.seats[0]
    )
}

impl OperationsReservationsWorld {
    fn runtime_bearer(core: &WorldCore) -> String {
        core.actors
            .bearer(RUNTIME)
            .expect("the world's runtime credential")
    }

    async fn fresh_operation(core: &WorldCore, status: &str, seats: usize) -> Operation {
        let author = &core.actors.user(Role::Admin).discord_id;
        operation(&core.state, author, status, seats).await
    }

    /// The leader who holds the squad in a probe's operation: the probe's own leader, or the
    /// primary leader for every other caller (an administrator then acts over another's hold).
    fn holder(core: &WorldCore, actor: Actor) -> &UserActor {
        match actor {
            Actor::User(Role::Leader) | Actor::Peer(Role::Leader) => core
                .actors
                .account(actor)
                .expect("a leader actor has an account"),
            _ => core.actors.user(Role::Leader),
        }
    }

    /// A fresh operation whose squad `holder` holds; `assigned` seats the primary enlisted
    /// account in seat 0.
    async fn held_operation(core: &WorldCore, holder: &UserActor, assigned: bool) -> Operation {
        let operation = Self::fresh_operation(core, "open", 2).await;
        let squad = json!({"squad": SQUAD});
        let hold = squad_uri(operation.attachment, "reserve");
        require(&core.app, &holder.token, "POST", &hold, Some(&squad)).await;
        if assigned {
            let seat = json!({"discord_id": core.actors.user(Role::Enlisted).discord_id});
            require(
                &core.app,
                &holder.token,
                "PUT",
                &seat_uri(&operation),
                Some(&seat),
            )
            .await;
        }
        operation
    }

    fn seat_fixture(operation: &Operation, body: Option<Value>) -> Fixture {
        let fixture = Fixture::new()
            .param("emid", operation.attachment.to_string())
            .param("slotId", operation.seats[0].to_string());
        match body {
            Some(body) => fixture.body(body),
            None => fixture,
        }
    }

    fn assignment_body(core: &WorldCore) -> Value {
        json!({"discord_id": core.actors.user(Role::Enlisted).discord_id})
    }

    fn squad_fixture(operation: &Operation) -> Fixture {
        Fixture::new()
            .param("emid", operation.attachment.to_string())
            .body(json!({"squad": SQUAD}))
    }

    /// A fresh operation in which `actor`'s account (when it has one) holds seat 0; an account
    /// the event policy refuses (a guest) waits on the attachment's waitlist instead, as an
    /// entry queued before a demotion does.
    async fn registered_operation(core: &WorldCore, actor: Actor) -> Operation {
        let operation = Self::fresh_operation(core, "open", 1).await;
        if let Some(account) = core.actors.account(actor) {
            let body = json!({"slot_id": operation.seats[0]});
            let uri = register_uri(operation.attachment);
            let registered = call(&core.app, &account.token, "POST", &uri, Some(&body)).await;
            if registered.status == StatusCode::FORBIDDEN {
                queue(core, &operation, account).await;
            }
        }
        operation
    }

    /// A fresh operation with one free seat and the peer enlisted account waiting; `full`
    /// seats the primary enlisted account first.
    async fn waitlist_operation(core: &WorldCore, full: bool) -> Operation {
        let operation = Self::fresh_operation(core, "open", 1).await;
        if full {
            let holder = core.actors.user(Role::Enlisted);
            let body = json!({"slot_id": operation.seats[0]});
            let uri = register_uri(operation.attachment);
            require(&core.app, &holder.token, "POST", &uri, Some(&body)).await;
        }
        queue(core, &operation, core.actors.peer(Role::Enlisted)).await;
        operation
    }

    fn deployment_body(&self, account: &UserActor, seat: usize, life: String) -> Value {
        json!({"event_mission_id": self.running.attachment,
            "orbat_slot_id": self.running.seats[seat], "arma_id": arma_of(account),
            "player_life_id": life})
    }

    fn fresh_life(&self) -> String {
        format!(
            "route-acceptance-life-{}",
            self.lives.fetch_add(1, Ordering::Relaxed)
        )
    }

    /// `account`'s open life in the running session: `(occupancy id, player life id)`.
    async fn open_life(&self, core: &WorldCore, account: &UserActor) -> Option<(Uuid, String)> {
        sqlx::query_as(
            "SELECT id, player_life_id FROM live_slot_occupancies
             WHERE runtime_session_id = $1 AND arma_id = $2 AND ended_at IS NULL",
        )
        .bind(self.running.session)
        .bind(arma_of(account))
        .fetch_optional(&core.state.pool)
        .await
        .expect("read the open life")
    }

    fn end_uri(&self, occupancy: Uuid) -> String {
        format!(
            "/api/v1/game-runtime/sessions/{}/deployments/{occupancy}/end",
            self.running.session
        )
    }

    /// `account`'s open life in seat `seat`, deploying a new one when it has none.
    async fn live_life(
        &self,
        core: &WorldCore,
        account: &UserActor,
        seat: usize,
    ) -> (Uuid, String) {
        if let Some(open) = self.open_life(core, account).await {
            return open;
        }
        let body = self.deployment_body(account, seat, self.fresh_life());
        let uri = format!(
            "/api/v1/game-runtime/sessions/{}/deployments",
            self.running.session
        );
        let decision = require(
            &core.app,
            &Self::runtime_bearer(core),
            "POST",
            &uri,
            Some(&body),
        )
        .await;
        assert_eq!(
            decision["decision"], "allowed",
            "setup deployment: {decision}"
        );
        let occupancy = decision["occupancy_id"]
            .as_str()
            .and_then(|id| id.parse().ok())
            .unwrap_or_else(|| panic!("setup deployment names its occupancy: {decision}"));
        (
            occupancy,
            body["player_life_id"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
        )
    }

    /// End `account`'s open life, if any, so its next deployment is decided afresh.
    async fn vacate(&self, core: &WorldCore, account: &UserActor) {
        if let Some((occupancy, _)) = self.open_life(core, account).await {
            let uri = self.end_uri(occupancy);
            require(&core.app, &Self::runtime_bearer(core), "POST", &uri, None).await;
        }
    }

    fn session_fixture(&self) -> Fixture {
        Fixture::new().param("sessionId", self.running.session.to_string())
    }

    async fn reservation_fixture(
        &self,
        core: &WorldCore,
        key: &str,
        actor: Actor,
    ) -> Option<Fixture> {
        let fixture = match key {
            "POST /api/v1/event-missions/{emid}/register" => {
                let operation = Self::fresh_operation(core, "open", 1).await;
                Fixture::new()
                    .param("emid", operation.attachment.to_string())
                    .body(json!({"slot_id": operation.seats[0]}))
            }
            "register-seat-taken" => {
                let operation = Self::fresh_operation(core, "open", 1).await;
                let body = json!({"slot_id": operation.seats[0]});
                let peer = core.actors.peer(Role::Enlisted);
                let uri = register_uri(operation.attachment);
                require(&core.app, &peer.token, "POST", &uri, Some(&body)).await;
                Fixture::new()
                    .param("emid", operation.attachment.to_string())
                    .body(body)
            }
            "operation-locked" => {
                let operation = Self::fresh_operation(core, "locked", 1).await;
                Fixture::new()
                    .param("emid", operation.attachment.to_string())
                    .body(json!({"slot_id": operation.seats[0], "squad": SQUAD}))
            }
            "DELETE /api/v1/event-missions/{emid}/register" => {
                let operation = Self::registered_operation(core, actor).await;
                Fixture::new().param("emid", operation.attachment.to_string())
            }
            "withdraw-unregistered" => {
                let operation = Self::fresh_operation(core, "open", 1).await;
                Fixture::new().param("emid", operation.attachment.to_string())
            }
            "withdraw-withdrawn" => {
                let operation = Self::registered_operation(core, actor).await;
                let account = core.actors.account(actor).expect("a withdrawing account");
                let uri = register_uri(operation.attachment);
                require(&core.app, &account.token, "DELETE", &uri, None).await;
                Fixture::new().param("emid", operation.attachment.to_string())
            }
            "PUT /api/v1/event-missions/{emid}/slots/{slotId}/assign" => {
                let operation = Self::held_operation(core, Self::holder(core, actor), false).await;
                Self::seat_fixture(&operation, Some(Self::assignment_body(core)))
            }
            "assign-held-by-leader" => {
                let operation =
                    Self::held_operation(core, core.actors.user(Role::Leader), false).await;
                Self::seat_fixture(&operation, Some(Self::assignment_body(core)))
            }
            "assign-repeated" => {
                let operation = Self::held_operation(core, Self::holder(core, actor), true).await;
                Self::seat_fixture(&operation, Some(Self::assignment_body(core)))
            }
            "DELETE /api/v1/event-missions/{emid}/slots/{slotId}/assign" => {
                let operation = Self::held_operation(core, Self::holder(core, actor), true).await;
                Self::seat_fixture(&operation, None)
            }
            "clear-held-by-leader" => {
                let operation =
                    Self::held_operation(core, core.actors.user(Role::Leader), true).await;
                Self::seat_fixture(&operation, None)
            }
            "clear-empty" => {
                let operation = Self::held_operation(core, Self::holder(core, actor), false).await;
                Self::seat_fixture(&operation, None)
            }
            "POST /api/v1/event-missions/{emid}/squads/reserve" | "squad-unheld" => {
                Self::squad_fixture(&Self::fresh_operation(core, "open", 1).await)
            }
            "squad-held-by-leader" => {
                let operation =
                    Self::held_operation(core, core.actors.user(Role::Leader), false).await;
                Self::squad_fixture(&operation)
            }
            "POST /api/v1/event-missions/{emid}/squads/release" => {
                let operation = Self::held_operation(core, Self::holder(core, actor), false).await;
                Self::squad_fixture(&operation)
            }
            "POST /api/v1/event-missions/{emid}/waitlist/promote" | "promote-full" => {
                let full = key == "promote-full";
                let operation = Self::waitlist_operation(core, full).await;
                Fixture::new().param("emid", operation.attachment.to_string())
            }
            "promote-empty" => {
                let operation = Self::fresh_operation(core, "open", 1).await;
                Fixture::new().param("emid", operation.attachment.to_string())
            }
            _ => return None,
        };
        Some(fixture)
    }

    async fn runtime_fixture(&self, core: &WorldCore, key: &str) -> Option<Fixture> {
        let enlisted = core.actors.user(Role::Enlisted);
        let peer = core.actors.peer(Role::Enlisted);
        let fixture = match key {
            "GET /api/v1/game-runtime/events/{id}/roster" => {
                Fixture::new().param("id", self.running.event.to_string())
            }
            "roster-idle" => Fixture::new().param("id", self.idle_event.to_string()),
            "roster-unbound" => Fixture::new().param("id", self.unbound_event.to_string()),
            "POST /api/v1/game-runtime/sessions/{sessionId}/deployments" => {
                self.vacate(core, peer).await;
                self.session_fixture()
                    .body(self.deployment_body(peer, 1, self.fresh_life()))
            }
            "deployment-retried" => {
                let (_, life) = self.live_life(core, peer, 1).await;
                self.session_fixture()
                    .body(self.deployment_body(peer, 1, life))
            }
            "deployment-life-reused" => {
                let (_, life) = self.live_life(core, peer, 1).await;
                self.session_fixture()
                    .body(self.deployment_body(peer, 2, life))
            }
            "POST /api/v1/game-runtime/sessions/{sessionId}/deployments/{occupancyId}/end" => {
                let (occupancy, _) = self.live_life(core, enlisted, 0).await;
                self.session_fixture()
                    .param("occupancyId", occupancy.to_string())
            }
            "life-ended" => {
                let (occupancy, _) = self.live_life(core, enlisted, 0).await;
                let uri = self.end_uri(occupancy);
                require(&core.app, &Self::runtime_bearer(core), "POST", &uri, None).await;
                self.session_fixture()
                    .param("occupancyId", occupancy.to_string())
            }
            _ => return None,
        };
        Some(fixture)
    }
}

impl PartWorld for OperationsReservationsWorld {
    async fn build(state: &mut AppState, actors: &Actors) -> Self {
        let app = http_router::router(state.clone());
        let fixture = EligibilityFixture::new(
            FIXTURE_SUITE,
            EventShape {
                max_slots: 0,
                missions: &[&[SQUAD, SQUAD, SQUAD]],
            },
        )
        .await;
        let server = actors.machines.server;
        bind_event(&fixture, server).await;
        let deployed = seed_deployment(&fixture, server, 0).await;
        let runtime = actors
            .bearer(RUNTIME)
            .expect("the world's runtime credential");
        let (session, _) = running_session(&fixture, &runtime, &deployed).await;
        let (event, attachment, seats) =
            (fixture.event, fixture.missions[0], fixture.slots[0].clone());
        let enlisted = actors.user(Role::Enlisted);
        let seat = json!({"slot_id": seats[0]});
        require(
            &app,
            &enlisted.token,
            "POST",
            &register_uri(attachment),
            Some(&seat),
        )
        .await;
        let author = &actors.user(Role::Admin).discord_id;
        let idle_event = operation(state, author, "open", 1).await.event;
        sqlx::query("UPDATE events SET server_id = $2 WHERE id = $1")
            .bind(idle_event)
            .bind(server)
            .execute(&state.pool)
            .await
            .expect("bind the idle event to the world's server");
        let unbound_event = operation(state, author, "open", 1).await.event;
        OperationsReservationsWorld {
            running: RunningEvent {
                _fixture: fixture,
                event,
                attachment,
                seats,
                session,
            },
            idle_event,
            unbound_event,
            lives: AtomicU32::new(1),
        }
    }

    async fn fixture(&self, core: &WorldCore, key: &str, actor: Actor) -> Option<Fixture> {
        match self.reservation_fixture(core, key, actor).await {
            Some(fixture) => Some(fixture),
            None => self.runtime_fixture(core, key).await,
        }
    }
}
