//! Route specs of the `operations_reservations` part: seat registration and withdrawal, leader
//! seat assignment, squad holds, waitlist promotion, fire missions, and the game-runtime roster
//! and player deployments.
//!
//! Each spec supplies only what the framework cannot derive: the success status and contract,
//! the ownership probes, the domain's malformed and boundary probes, and a reason for every
//! dimension that does not apply. Fixture keys other than a spec key are minted by
//! `world/operations_reservations.rs`.

use serde_json::json;
use website_api::operations::models::generated::game_runtime_deployment::{
    DeploymentDecision, EndedLife,
};
use website_api::operations::models::generated::game_runtime_roster::EventRoster;
use website_api::operations::models::generated::reservation_response::ReservationResponse;
use website_api::operations::models::generated::waitlist_promotion_response::WaitlistPromotionResponse;

use super::super::contracts::round_trip;
use super::super::spec::{Actor, Change, Contract, Executor, Expect, Probe, Role, RouteSpec};

const SELF_SCOPED: &str = "self-scoped: the caller registers or withdraws its own account only";
const MEMBERS_ONLY: &str = "an event without its own access policy admits members only: the \
     event policy, not the route's access class, refuses a guest's registration";
const ANY_LEADER: &str = "the waitlist belongs to the attachment: any leader or administrator \
     may run the promotion";
const SHARED_READ: &str = "an event's fire missions are shared with every member: no row is \
     owned by the reader";
const STATELESS: &str = "a firing solution is computed from the body alone and addresses no \
     stored row";
const OWN_NEW_ROW: &str = "the saved fire mission is the caller's own new row; the event it \
     names is shared by every member";
const RESERVATIONS: &str = "reservation-actions.schema.json";
const FIRE: &str = "fire-mission.schema.json";
const DEPLOYMENT: &str = "game-runtime-deployment.schema.json";
const ENLISTED: Actor = Actor::User(Role::Enlisted);
const LEADER: Actor = Actor::User(Role::Leader);
const PEER_LEADER: Actor = Actor::Peer(Role::Leader);
const ADMIN: Actor = Actor::User(Role::Admin);
const RUNTIME: Actor = Actor::Machine(Executor::ModRuntime);
const OTHER_RUNTIME: Actor = Actor::MachineOtherServer(Executor::ModRuntime);

fn reservation_actions(definition: &'static str) -> Contract {
    Contract::schema(RESERVATIONS, definition)
}

/// The three squad-management ownership probes over a squad the primary leader holds: another
/// leader is refused with `refusal`, the holder and an administrator succeed.
fn squad_management(spec: RouteSpec, fixture: &'static str, refusal: &'static str) -> RouteSpec {
    spec.ownership(
        Probe::new("another leader cannot manage the held squad", PEER_LEADER)
            .fixture(fixture)
            .expect_with(Expect::status(403).error_contains(refusal)),
    )
    .ownership(Probe::new("the holding leader manages its squad", LEADER).fixture(fixture))
    .ownership(Probe::new("an administrator overrides the hold", ADMIN).fixture(fixture))
}

fn reservation_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::authenticated("POST /api/v1/event-missions/{emid}/register")
            .ok(
                200,
                Contract::schema_root("reservation-response.schema.json"),
            )
            .decodes(round_trip::<ReservationResponse>)
            .request_contract(reservation_actions("RegistrationRequest"))
            .authorized_as(ENLISTED)
            .override_derived(
                "guest",
                Expect::status(403).details_code("ACCESS_POLICY"),
                MEMBERS_ONLY,
            )
            .no_ownership(SELF_SCOPED)
            .malformed(
                Probe::new("missing slot id", ENLISTED)
                    .change(Change::RemoveField("slot_id"))
                    .expect(400),
            )
            .malformed(
                Probe::new("non-uuid slot id", ENLISTED)
                    .body(json!({"slot_id": "not-a-uuid"}))
                    .expect(400),
            )
            .boundary(Probe::new("register without a seat", ENLISTED).body(json!({"slot_id": ""})))
            .boundary(
                Probe::new("unknown seat", ENLISTED)
                    .body(json!({"slot_id": "00000000-0000-4000-8000-000000000000"}))
                    .expect_with(Expect::status(404).error_contains("slot not found")),
            )
            .boundary(
                Probe::new("seat held by another participant", ENLISTED)
                    .fixture("register-seat-taken")
                    .expect(409),
            )
            .boundary(
                Probe::new("registration closed", ENLISTED)
                    .fixture("operation-locked")
                    .expect_with(Expect::status(409).error_contains("registration is closed")),
            ),
        RouteSpec::authenticated("DELETE /api/v1/event-missions/{emid}/register")
            .ok(200, reservation_actions("RegistrationWithdrawal"))
            .authorized_as(ENLISTED)
            .no_ownership(SELF_SCOPED)
            .boundary(
                Probe::new("not registered", ENLISTED)
                    .fixture("withdraw-unregistered")
                    .expect_with(Expect::status(404).error_contains("not registered")),
            )
            .boundary(
                Probe::new("withdrawing twice is idempotent", ENLISTED)
                    .fixture("withdraw-withdrawn"),
            ),
        squad_management(
            RouteSpec::role(
                "PUT /api/v1/event-missions/{emid}/slots/{slotId}/assign",
                Role::Leader,
            )
            .ok(200, reservation_actions("SlotAssignmentOutcome"))
            .request_contract(reservation_actions("SlotAssignmentRequest")),
            "assign-held-by-leader",
            "reserve this squad",
        )
        .malformed(
            Probe::new("empty discord id", LEADER)
                .merge_body(json!({"discord_id": ""}))
                .expect(400),
        )
        .malformed(
            Probe::new("missing discord id", LEADER)
                .change(Change::RemoveField("discord_id"))
                .expect(400),
        )
        .boundary(
            Probe::new("discord id over 128 bytes", LEADER)
                .merge_body(json!({"discord_id": "9".repeat(129)}))
                .expect_with(Expect::status(400).error_contains("128 bytes")),
        )
        .boundary(
            Probe::new("repeating an assignment is idempotent", LEADER).fixture("assign-repeated"),
        ),
        squad_management(
            RouteSpec::role(
                "DELETE /api/v1/event-missions/{emid}/slots/{slotId}/assign",
                Role::Leader,
            )
            .ok(200, reservation_actions("SlotClearance")),
            "clear-held-by-leader",
            "reserve this squad",
        )
        .boundary(Probe::new("clearing an empty seat", LEADER).fixture("clear-empty")),
        RouteSpec::role(
            "POST /api/v1/event-missions/{emid}/squads/reserve",
            Role::Leader,
        )
        .ok(201, reservation_actions("SquadReservation"))
        .request_contract(reservation_actions("SquadRequest"))
        .ownership(
            Probe::new("another leader cannot take the held squad", PEER_LEADER)
                .fixture("squad-held-by-leader")
                .expect_with(Expect::status(409).error_contains("already reserved")),
        )
        .ownership(
            Probe::new("an administrator cannot take the held squad", ADMIN)
                .fixture("squad-held-by-leader")
                .expect_with(Expect::status(409).error_contains("already reserved")),
        )
        .ownership(
            Probe::new("the holder's repeat answers the existing hold", LEADER)
                .fixture("squad-held-by-leader")
                .expect_with(Expect::status(200).contract(reservation_actions("SquadReservation"))),
        )
        .malformed(
            Probe::new("empty squad", LEADER)
                .merge_body(json!({"squad": ""}))
                .expect(400),
        )
        .boundary(
            Probe::new("squad outside the ORBAT", LEADER)
                .merge_body(json!({"squad": "Zulu"}))
                .expect_with(Expect::status(404).error_contains("squad not found")),
        )
        .boundary(
            Probe::new("registration closed", LEADER)
                .fixture("operation-locked")
                .expect_with(Expect::status(409).error_contains("registration is closed")),
        ),
        squad_management(
            RouteSpec::role(
                "POST /api/v1/event-missions/{emid}/squads/release",
                Role::Leader,
            )
            .ok(200, reservation_actions("SquadRelease"))
            .request_contract(reservation_actions("SquadRequest")),
            "squad-held-by-leader",
            "only the reserver or an admin",
        )
        .malformed(
            Probe::new("empty squad", LEADER)
                .merge_body(json!({"squad": ""}))
                .expect(400),
        )
        .boundary(
            Probe::new("squad not held", LEADER)
                .fixture("squad-unheld")
                .expect_with(Expect::status(404).error_contains("not reserved")),
        ),
        RouteSpec::role(
            "POST /api/v1/event-missions/{emid}/waitlist/promote",
            Role::Leader,
        )
        .ok(
            200,
            Contract::schema_root("waitlist-promotion-response.schema.json"),
        )
        .decodes(round_trip::<WaitlistPromotionResponse>)
        .no_ownership(ANY_LEADER)
        .boundary(
            Probe::new("nobody waiting", LEADER)
                .fixture("promote-empty")
                .expect_with(Expect::status(404).error_contains("no participant is waiting")),
        )
        .boundary(
            Probe::new("no free seat", LEADER)
                .fixture("promote-full")
                .expect_with(Expect::status(409).details_code("EVENT_FULL")),
        ),
    ]
}

fn fire_mission_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::authenticated("GET /api/v1/events/{id}/fire-missions")
            .ok(200, Contract::schema(FIRE, "FireMissionList"))
            .no_ownership(SHARED_READ)
            .boundary(
                Probe::new("an event without fire missions lists none", ENLISTED)
                    .fixture("fire-missions-none")
                    .expect_with(Expect::success().json_at("/data", json!([]))),
            ),
        RouteSpec::authenticated("POST /api/v1/fire-missions/solve")
            .ok(200, Contract::schema(FIRE, "FireSolution"))
            .request_contract(Contract::schema(FIRE, "FireSolveRequest"))
            .no_ownership(STATELESS)
            .malformed(
                Probe::new("missing target", ENLISTED)
                    .change(Change::RemoveField("tgt_x"))
                    .expect(400),
            )
            .malformed(
                Probe::new("unknown weapon", ENLISTED)
                    .merge_body(json!({"weapon_system": "M999 99mm"}))
                    .expect_with(Expect::status(400).error_contains("unknown weapon_system")),
            )
            .malformed(
                Probe::new("weapon with trailing whitespace", ENLISTED)
                    .merge_body(json!({"weapon_system": "M252 81mm "}))
                    .expect(400),
            )
            .boundary(
                Probe::new("target out of range", ENLISTED)
                    .merge_body(json!({"tgt_x": 90000.0, "tgt_y": 90000.0}))
                    .expect_with(Expect::status(422).error_contains("out of range")),
            )
            .boundary(
                Probe::new("zero coordinates are a real grid", ENLISTED)
                    .merge_body(json!({"fp_x": 0.0, "fp_y": 0.0, "tgt_x": 300.0, "tgt_y": 400.0})),
            ),
        RouteSpec::authenticated("POST /api/v1/fire-missions")
            .ok(201, Contract::schema(FIRE, "SavedFireMission"))
            .request_contract(Contract::schema(FIRE, "FireMissionSave"))
            .no_ownership(OWN_NEW_ROW)
            .malformed(
                Probe::new("blank event id", ENLISTED)
                    .merge_body(json!({"event_id": " "}))
                    .expect(400),
            )
            .malformed(
                Probe::new("non-uuid event id", ENLISTED)
                    .merge_body(json!({"event_id": "not-a-uuid"}))
                    .expect(400),
            )
            .malformed(
                Probe::new("blank grid", ENLISTED)
                    .merge_body(json!({"target_grid": "  "}))
                    .expect(400),
            )
            .boundary(
                Probe::new("a fire mission without an event", ENLISTED)
                    .merge_body(json!({"event_id": null})),
            )
            .boundary(
                Probe::new("nonexistent event", ENLISTED)
                    .merge_body(json!({"event_id": "00000000-0000-4000-8000-000000000000"}))
                    .expect(404),
            )
            .boundary(
                Probe::new("target out of range", ENLISTED)
                    .merge_body(json!({"tgt_x": 90000.0, "tgt_y": 90000.0}))
                    .expect(422),
            ),
    ]
}

fn game_runtime_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::machine(
            "GET /api/v1/game-runtime/events/{id}/roster",
            Executor::ModRuntime,
        )
        .ok(
            200,
            Contract::schema_root("game-runtime-roster.schema.json"),
        )
        .decodes(round_trip::<EventRoster>)
        .ownership(
            Probe::new("another server's runtime", OTHER_RUNTIME)
                .expect_with(Expect::status(403).error_contains("credential")),
        )
        .ownership(
            Probe::new("an event bound to no server", RUNTIME)
                .fixture("roster-unbound")
                .expect_with(Expect::status(403).error_contains("not bound")),
        )
        .boundary(
            Probe::new("an event without a deployment in effect", RUNTIME)
                .fixture("roster-idle")
                .expect_with(Expect::success().json_at("/slots", json!([]))),
        ),
        RouteSpec::machine(
            "POST /api/v1/game-runtime/sessions/{sessionId}/deployments",
            Executor::ModRuntime,
        )
        .ok(200, Contract::schema_root(DEPLOYMENT))
        .decodes(round_trip::<DeploymentDecision>)
        .request_contract(Contract::schema(DEPLOYMENT, "DeploymentRequest"))
        .ownership(
            Probe::new("another server's runtime", OTHER_RUNTIME)
                .expect_with(Expect::status(403).error_contains("credential")),
        )
        .malformed(
            Probe::new("missing player life id", RUNTIME)
                .change(Change::RemoveField("player_life_id"))
                .expect(400),
        )
        .malformed(
            Probe::new("non-uuid slot id", RUNTIME)
                .merge_body(json!({"orbat_slot_id": "not-a-uuid"}))
                .expect(400),
        )
        .malformed(
            Probe::new("blank arma id", RUNTIME)
                .merge_body(json!({"arma_id": "  "}))
                .expect(400),
        )
        .boundary(
            Probe::new("arma id over 128 bytes", RUNTIME)
                .merge_body(json!({"arma_id": "a".repeat(129)}))
                .expect_with(Expect::status(400).error_contains("128 bytes")),
        )
        .boundary(
            Probe::new("an unlinked identity is denied", RUNTIME)
                .merge_body(json!({"arma_id": "route-acceptance-unlinked"}))
                .expect_with(
                    Expect::success()
                        .json_at("/decision", json!("denied"))
                        .json_at("/reason", json!("IDENTITY_NOT_LINKED")),
                ),
        )
        .boundary(
            Probe::new("a retried life answers its recorded decision", RUNTIME)
                .fixture("deployment-retried")
                .expect_with(Expect::success().json_at("/decision", json!("allowed"))),
        )
        .boundary(
            Probe::new("a life id reused for another slot", RUNTIME)
                .fixture("deployment-life-reused")
                .expect(409),
        )
        .boundary(
            Probe::new("a slot outside the event mission", RUNTIME)
                .merge_body(json!({"orbat_slot_id": "00000000-0000-4000-8000-000000000000"}))
                .expect_with(Expect::status(404).error_contains("slot not found")),
        ),
        RouteSpec::machine(
            "POST /api/v1/game-runtime/sessions/{sessionId}/deployments/{occupancyId}/end",
            Executor::ModRuntime,
        )
        .ok(200, Contract::schema(DEPLOYMENT, "EndedLife"))
        .decodes(round_trip::<EndedLife>)
        .ownership(
            Probe::new("another server's runtime", OTHER_RUNTIME)
                .expect_with(Expect::status(403).error_contains("credential")),
        )
        .boundary(
            Probe::new("ending a life twice keeps its first end", RUNTIME)
                .fixture("life-ended")
                .expect_with(Expect::success().json_at("/end_reason", json!("life_ended"))),
        ),
    ]
}

/// Every spec of the `operations_reservations` part.
pub fn specs() -> Vec<RouteSpec> {
    [
        reservation_specs(),
        fire_mission_specs(),
        game_runtime_specs(),
    ]
    .into_iter()
    .flatten()
    .collect()
}
