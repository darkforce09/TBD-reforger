//! Route specs of the `operations_reservations` part: seat registration and withdrawal, leader
//! seat assignment, squad holds, waitlist promotion, and the game-runtime roster and player
//! deployments.
//!
//! Each spec supplies only what the framework cannot derive: the success status and contract,
//! the authorized caller and fixture where the defaults do not fit, and the unauthorized probes
//! and overrides the access class does not imply.

use super::super::spec::{Actor, Contract, Executor, Role, RouteSpec};

const RESERVATIONS: &str = "reservation-actions.schema.json";
const DEPLOYMENT: &str = "game-runtime-deployment.schema.json";
const ENLISTED: Actor = Actor::User(Role::Enlisted);

fn reservation_actions(definition: &'static str) -> Contract {
    Contract::schema(RESERVATIONS, definition)
}

/// The three squad-management ownership probes over a squad the primary leader holds: another
/// leader is refused with `refusal`, the holder and an administrator succeed.
fn squad_management(spec: RouteSpec, _fixture: &'static str, _refusal: &'static str) -> RouteSpec {
    spec
}

fn reservation_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::authenticated("POST /api/v1/event-missions/{emid}/register")
            .ok(
                200,
                Contract::schema_root("reservation-response.schema.json"),
            )
            .authorized_as(ENLISTED),
        RouteSpec::authenticated("DELETE /api/v1/event-missions/{emid}/register")
            .ok(200, reservation_actions("RegistrationWithdrawal"))
            .authorized_as(ENLISTED),
        squad_management(
            RouteSpec::role(
                "PUT /api/v1/event-missions/{emid}/slots/{slotId}/assign",
                Role::Leader,
            )
            .ok(200, reservation_actions("SlotAssignmentOutcome")),
            "assign-held-by-leader",
            "reserve this squad",
        ),
        squad_management(
            RouteSpec::role(
                "DELETE /api/v1/event-missions/{emid}/slots/{slotId}/assign",
                Role::Leader,
            )
            .ok(200, reservation_actions("SlotClearance")),
            "clear-held-by-leader",
            "reserve this squad",
        ),
        RouteSpec::role(
            "POST /api/v1/event-missions/{emid}/squads/reserve",
            Role::Leader,
        )
        .ok(201, reservation_actions("SquadReservation")),
        squad_management(
            RouteSpec::role(
                "POST /api/v1/event-missions/{emid}/squads/release",
                Role::Leader,
            )
            .ok(200, reservation_actions("SquadRelease")),
            "squad-held-by-leader",
            "only the reserver or an admin",
        ),
        RouteSpec::role(
            "POST /api/v1/event-missions/{emid}/waitlist/promote",
            Role::Leader,
        )
        .ok(
            200,
            Contract::schema_root("waitlist-promotion-response.schema.json"),
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
        ),
        RouteSpec::machine(
            "POST /api/v1/game-runtime/sessions/{sessionId}/deployments",
            Executor::ModRuntime,
        )
        .ok(200, Contract::schema_root(DEPLOYMENT)),
        RouteSpec::machine(
            "POST /api/v1/game-runtime/sessions/{sessionId}/deployments/{occupancyId}/end",
            Executor::ModRuntime,
        )
        .ok(200, Contract::schema(DEPLOYMENT, "EndedLife")),
    ]
}

/// Every spec of the `operations_reservations` part.
pub(crate) fn specs() -> Vec<RouteSpec> {
    [reservation_specs(), game_runtime_specs()]
        .into_iter()
        .flatten()
        .collect()
}
