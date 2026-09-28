//! The success contract of every route that answers JSON, one row per route.
//!
//! **Role:** the table [`super::route_contracts::contract_for`] resolves a golden's method and
//! path against, so every golden in the frontend corpus names the `contracts_v2/definitions`
//! shape its route promises.
//!
//! **Position:** one row per `/// @route` handler whose success answer is JSON or an event
//! stream; redirects, `204`s and binary downloads have no row. Read by the
//! `contract_parity_goldens` binary only.
//!
//! **Signals & state:** none; a pure table.
//!
//! **Invariants:** paths use the `@route` spelling (`:param` or `{param}`); a row with a query
//! predicate outranks the same route without one; an envelope whose `data` rows hold a 2020-12
//! document names that document's schema separately, because the draft-07 envelope files cannot
//! reference it (the faction rows' `doc`); every other envelope validates its rows as a whole.

use super::route_contracts::{RouteContract, SchemaRef, array_of, definition, event_stream, root};

/// Routes whose whole JSON body answers to one schema: `METHOD path file#` for a file's root
/// schema, `METHOD path file#/definitions/Name` for a draft-07 definition.
const WHOLE_BODY_ROUTES: &str = "\
POST /api/v1/auth/refresh session-token.schema.json#/definitions/SessionTokenPair
GET /api/v1/me current-profile.schema.json#
PATCH /api/v1/me profile-update.schema.json#/definitions/UpdatedProfile
POST /api/v1/me/link arma-link.schema.json#/definitions/LinkCode
GET /api/v1/me/link/status arma-link.schema.json#/definitions/LinkStatus
DELETE /api/v1/me/link arma-link.schema.json#/definitions/LinkRemoval
POST /api/v1/ingest/link-confirm arma-link.schema.json#/definitions/LinkConfirmation
GET /api/v1/admin/audit-logs audit-log.schema.json#/definitions/AuditLogPage
GET /api/v1/admin/users personnel-roster.schema.json#/definitions/PersonnelPage
POST /api/v1/admin/users/:discordId/ban personnel-actions.schema.json#/definitions/BanState
DELETE /api/v1/admin/users/:discordId/ban personnel-actions.schema.json#/definitions/BanState
POST /api/v1/admin/users/:discordId/warnings personnel-actions.schema.json#/definitions/Warning
POST /api/v1/admin/users/:discordId/membership-grace personnel-actions.schema.json#/definitions/MembershipGraceExtension
POST /api/v1/admin/roles/sync personnel-actions.schema.json#/definitions/RoleResyncOutcome
GET /api/v1/dashboard command-center.schema.json#/definitions/Dashboard
GET /api/v1/leaderboards command-center.schema.json#/definitions/LeaderboardPage
GET /api/v1/users/:discordId/stats command-center.schema.json#/definitions/PlayerStatsCard
GET /api/v1/announcements announcement.schema.json#/definitions/AnnouncementPage
GET /api/v1/announcements/:id announcement.schema.json#/definitions/Announcement
GET /api/v1/cms/announcements announcement.schema.json#/definitions/AnnouncementPage
POST /api/v1/cms/announcements announcement.schema.json#/definitions/Announcement
PATCH /api/v1/cms/announcements/:id announcement.schema.json#/definitions/Announcement
POST /api/v1/cms/announcements/:id/push-discord announcement.schema.json#/definitions/DiscordPushOutcome
POST /api/v1/cms/uploads content-upload.schema.json#/definitions/UploadResponse
GET /api/v1/modpacks modpack.schema.json#/definitions/ModpackList
GET /api/v1/modpacks/current modpack.schema.json#/definitions/Modpack
POST /api/v1/modpacks modpack.schema.json#/definitions/Modpack
PUT /api/v1/modpacks/:id modpack.schema.json#/definitions/Modpack
POST /api/v1/modpacks/:id/set-current modpack.schema.json#/definitions/Modpack
GET /api/v1/vehicle-database vehicle-database.schema.json#/definitions/VehicleList
GET /api/v1/vehicle-database/{id} vehicle-database.schema.json#/definitions/Vehicle
POST /api/v1/vehicle-database vehicle-database.schema.json#/definitions/Vehicle
PUT /api/v1/vehicle-database/{id} vehicle-database.schema.json#/definitions/Vehicle
PATCH /api/v1/vehicle-database/{id} vehicle-database.schema.json#/definitions/Vehicle
DELETE /api/v1/vehicle-database/{id} vehicle-database.schema.json#/definitions/Vehicle
GET /api/v1/wiki wiki-page.schema.json#/definitions/WikiPageList
GET /api/v1/wiki/{slug} wiki-page.schema.json#/definitions/WikiArticle
PUT /api/v1/wiki/{slug} wiki-page.schema.json#/definitions/WikiArticle
GET /api/v1/wiki/{slug}/revisions wiki-page.schema.json#/definitions/WikiRevisionPage
GET /api/v1/wiki/{slug}/revisions/{revision} wiki-page.schema.json#/definitions/WikiRevision
GET /api/v1/debug/equipment-data/status equipment-data-viewer/dataset.schema.json#
GET /api/v1/debug/equipment-data/overview equipment-data-viewer/dataset.schema.json#
GET /api/v1/debug/equipment-data/resources equipment-data-viewer/resources.schema.json#
GET /api/v1/debug/equipment-data/relationships equipment-data-viewer/relationships.schema.json#
GET /api/v1/debug/equipment-data/fields equipment-data-viewer/field-inventory.schema.json#
GET /api/v1/debug/equipment-data/resource-cards equipment-data-viewer/resource-cards.schema.json#
GET /api/v1/debug/equipment-data/selection equipment-data-viewer/source-inspection.schema.json#
GET /api/v1/debug/equipment-data/containers equipment-data-viewer/source-inspection.schema.json#
GET /api/v1/debug/equipment-data/properties equipment-data-viewer/source-inspection.schema.json#
GET /api/v1/debug/equipment-data/values equipment-data-viewer/source-inspection.schema.json#
GET /api/v1/debug/equipment-data/documents equipment-data-viewer/source-inspection.schema.json#
POST /api/v1/ingest/matches match-telemetry.schema.json#/definitions/MatchRegistrationAnswer
POST /api/v1/ingest/match-results match-telemetry.schema.json#/definitions/MatchResultsAnswer
POST /api/v1/ingest/match-events match-telemetry.schema.json#/definitions/MatchEventBatchAnswer
GET /api/v1/matches/:matchId/events match-telemetry.schema.json#
POST /api/v1/game-runtime/sessions/:sessionId/heartbeats runtime-heartbeat-receipt.schema.json#/definitions/HeartbeatReceipt
GET /api/v1/missions mission-library.schema.json#/definitions/MissionLibraryPage
POST /api/v1/missions mission-review.schema.json#/definitions/MissionRow
GET /api/v1/missions/:id mission-library.schema.json#/definitions/MissionDetail
PATCH /api/v1/missions/:id mission-review.schema.json#/definitions/MissionRow
POST /api/v1/missions/:id/bookmark mission-library.schema.json#/definitions/BookmarkState
DELETE /api/v1/missions/:id/bookmark mission-library.schema.json#/definitions/BookmarkState
GET /api/v1/missions/:id/armory mission-library.schema.json#/definitions/MissionArmoryList
PUT /api/v1/missions/:id/armory mission-library.schema.json#/definitions/MissionArmoryList
POST /api/v1/missions/:id/versions mission-review.schema.json#/definitions/MissionVersion
GET /api/v1/missions/:id/versions/:vid mission-review.schema.json#/definitions/MissionVersion
POST /api/v1/missions/:id/versions/:vid/set-current mission-review.schema.json#/definitions/MissionRow
POST /api/v1/missions/:id/submit mission-review.schema.json#/definitions/MissionRow
GET /api/v1/missions/:id/reviews mission-review.schema.json#
POST /api/v1/missions/:id/review-comments mission-review.schema.json#/definitions/ReviewComment
GET /api/v1/missions/:id/artifacts/:artifact_id mission-review.schema.json#/definitions/MissionArtifact
GET /api/v1/missions/:id/artifacts/:artifact_id/workspace mission-review.schema.json#/definitions/ReviewWorkspace
GET /api/v1/approvals mission-review.schema.json#/definitions/ApprovalQueuePage
POST /api/v1/approvals/:id/approve mission-review.schema.json#/definitions/MissionRow
POST /api/v1/approvals/:id/reject mission-review.schema.json#/definitions/MissionRow
GET /api/v1/admin/mission-default-overrides mission-default-overrides.schema.json#/definitions/MissionDefaultOverrideReport
POST /api/v1/factions arsenal-envelopes.schema.json#/definitions/UserFaction
GET /api/v1/factions/:id arsenal-envelopes.schema.json#/definitions/UserFaction
PUT /api/v1/factions/:id arsenal-envelopes.schema.json#/definitions/UserFaction
GET /api/v1/game-runtime/deployment mission-deployment.schema.json#/definitions/RuntimeDeployment
GET /api/v1/game-runtime/missions mission-deployment.schema.json#/definitions/DeployableMissionList
POST /api/v1/game-runtime/deployments mission-deployment.schema.json#/definitions/MissionDeployment
POST /api/v1/servers/:id/deployments mission-deployment.schema.json#/definitions/MissionDeployment
GET /api/v1/servers/:id/deployments mission-deployment.schema.json#/definitions/MissionDeploymentPage
GET /api/v1/servers/:id/deployments/:deploymentId mission-deployment.schema.json#/definitions/MissionDeployment
POST /api/v1/servers/:id/deployments/:deploymentId/cancel mission-deployment.schema.json#/definitions/MissionDeployment
GET /api/v1/events event-schedule.schema.json#/definitions/EventListPage
POST /api/v1/events event-schedule.schema.json#/definitions/Event
GET /api/v1/events/:id event-hub.schema.json#
PATCH /api/v1/events/:id event-schedule.schema.json#/definitions/Event
POST /api/v1/events/:id/missions event-schedule.schema.json#/definitions/EventMission
GET /api/v1/events/:id/fire-missions fire-mission.schema.json#/definitions/FireMissionList
POST /api/v1/fire-missions/solve fire-mission.schema.json#/definitions/FireSolution
POST /api/v1/fire-missions fire-mission.schema.json#/definitions/SavedFireMission
GET /api/v1/events/:id/access event-access-administration.schema.json#/definitions/EventAccessAdministration
PUT /api/v1/events/:id/access-policy event-access-administration.schema.json#/definitions/AccessChangeOutcome
PUT /api/v1/events/:id/reservation-quotas event-access-administration.schema.json#/definitions/AccessChangeOutcome
POST /api/v1/events/:id/groups event-access-administration.schema.json#/definitions/AccessChangeOutcome
PATCH /api/v1/events/:id/groups/:groupId event-access-administration.schema.json#/definitions/AccessChangeOutcome
DELETE /api/v1/events/:id/groups/:groupId event-access-administration.schema.json#/definitions/AccessChangeOutcome
PUT /api/v1/events/:id/groups/:groupId/members/:discordId event-access-administration.schema.json#/definitions/AccessChangeOutcome
DELETE /api/v1/events/:id/groups/:groupId/members/:discordId event-access-administration.schema.json#/definitions/AccessChangeOutcome
PUT /api/v1/event-missions/:emid/squads/:faction/:squad/access-policy event-access-administration.schema.json#/definitions/AccessChangeOutcome
DELETE /api/v1/event-missions/:emid/squads/:faction/:squad/access-policy event-access-administration.schema.json#/definitions/AccessChangeOutcome
PUT /api/v1/event-missions/:emid/slots/:slotId/access-policy event-access-administration.schema.json#/definitions/AccessChangeOutcome
DELETE /api/v1/event-missions/:emid/slots/:slotId/access-policy event-access-administration.schema.json#/definitions/AccessChangeOutcome
GET /api/v1/event-missions/:emid/orbat event-orbat.schema.json#
POST /api/v1/event-missions/:emid/register reservation-response.schema.json#
DELETE /api/v1/event-missions/:emid/register reservation-actions.schema.json#/definitions/RegistrationWithdrawal
PUT /api/v1/event-missions/:emid/slots/:slotId/assign reservation-actions.schema.json#/definitions/SlotAssignmentOutcome
DELETE /api/v1/event-missions/:emid/slots/:slotId/assign reservation-actions.schema.json#/definitions/SlotClearance
POST /api/v1/event-missions/:emid/squads/reserve reservation-actions.schema.json#/definitions/SquadReservation
POST /api/v1/event-missions/:emid/squads/release reservation-actions.schema.json#/definitions/SquadRelease
POST /api/v1/event-missions/:emid/waitlist/promote waitlist-promotion-response.schema.json#
GET /api/v1/members member-directory.schema.json#/definitions/MemberSearchPage
GET /api/v1/me/deployments service-record.schema.json#/definitions/ServiceRecord
GET /api/v1/me/leave-requests leave-request.schema.json#/definitions/LeaveRequestList
POST /api/v1/me/leave-requests leave-request.schema.json#/definitions/LeaveRequest
GET /api/v1/admin/leave-requests leave-request.schema.json#/definitions/LeaveRequestPage
PATCH /api/v1/admin/leave-requests/:id leave-request.schema.json#/definitions/LeaveReviewOutcome
GET /api/v1/game-runtime/events/:id/roster game-runtime-roster.schema.json#
POST /api/v1/game-runtime/sessions/:sessionId/deployments game-runtime-deployment.schema.json#
POST /api/v1/game-runtime/sessions/:sessionId/deployments/:occupancyId/end game-runtime-deployment.schema.json#/definitions/EndedLife
GET /api/v1/servers server-intel.schema.json#/definitions/ServerIntelList
POST /api/v1/servers server-intel.schema.json#/definitions/ServerIntel
GET /api/v1/servers/:id/status server-intel.schema.json#/definitions/ServerIntel
PATCH /api/v1/servers/:id server-intel.schema.json#/definitions/ServerIntel
POST /api/v1/servers/:id/commands fleet-command.schema.json#/definitions/FleetCommandReceipt
GET /api/v1/servers/:id/commands fleet-command.schema.json#/definitions/FleetCommandList
GET /api/v1/servers/:id/commands/:commandId fleet-command.schema.json#/definitions/FleetCommandReceipt
POST /api/v1/servers/:id/commands/:commandId/cancel fleet-command.schema.json#/definitions/FleetCommandReceipt
POST /api/v1/fleet-executor/commands/:commandId/executing fleet-command.schema.json#/definitions/FleetCommandReceipt
POST /api/v1/fleet-executor/commands/:commandId/result fleet-command.schema.json#/definitions/FleetCommandReceipt
GET /api/v1/servers/:id/credentials machine-credential.schema.json#/definitions/MachineCredentialList
POST /api/v1/servers/:id/credentials machine-credential.schema.json#
DELETE /api/v1/servers/:id/credentials/:credentialId machine-credential.schema.json#/definitions/MachineCredential
GET /api/v1/fleet/scenarios mission-deployment.schema.json#/definitions/FleetScenarioList
PUT /api/v1/fleet/scenarios/:terrainKey mission-deployment.schema.json#/definitions/FleetScenario
POST /api/v1/game-runtime/sessions game-runtime-session.schema.json#
POST /api/v1/game-runtime/sessions/:sessionId/end game-runtime-session.schema.json#/definitions/RuntimeSessionEnd
GET /healthz service-health.schema.json#/definitions/PublicHealth
POST /api/v1/fleet-executor/commands/claim fleet-command.schema.json#/definitions/ClaimedFleetCommand
";

/// After `ready`, the audit stream sends each published row unnamed and `reset` when a cursor
/// cannot be replayed.
const AUDIT_STREAM_LATER_FRAMES: &[(&str, SchemaRef)] = &[
    (
        "",
        SchemaRef::definition("audit-log.schema.json", "AuditLogEntry"),
    ),
    (
        "reset",
        SchemaRef::definition("audit-log.schema.json", "AuditStreamReset"),
    ),
];

/// After the snapshot, the server status stream relays hub broadcasts of the same shape.
const SERVER_STATUS_LATER_FRAMES: &[(&str, SchemaRef)] = &[(
    "",
    SchemaRef::definition("server-intel.schema.json", "ServerStatus"),
)];

/// Every JSON-answering route with its success contract.
pub fn route_contracts() -> Vec<RouteContract> {
    let mut contracts: Vec<RouteContract> =
        WHOLE_BODY_ROUTES.lines().map(whole_body_route).collect();
    contracts.extend([
        event_stream(
            "GET",
            "/api/v1/admin/audit-logs/stream",
            SchemaRef::definition("audit-log.schema.json", "AuditStreamReady"),
            AUDIT_STREAM_LATER_FRAMES,
        ),
        event_stream(
            "GET",
            "/api/v1/servers/:id/status/stream",
            SchemaRef::definition("server-intel.schema.json", "ServerStatus"),
            SERVER_STATUS_LATER_FRAMES,
        ),
        array_of(
            "GET",
            "/api/v1/events/:id/access/participants",
            "event-access-administration.schema.json",
            "ParticipantAccessExplanation",
        ),
        definition(
            "GET",
            "/api/v1/factions",
            "arsenal-envelopes.schema.json",
            "FactionList",
        )
        .with_rows(Some("doc"), SchemaRef::root("faction-library.schema.json")),
        definition(
            "GET",
            "/api/v1/registry",
            "arsenal-envelopes.schema.json",
            "RegistryItemPage",
        ),
        definition(
            "GET",
            "/api/v1/registry/compat",
            "arsenal-envelopes.schema.json",
            "RegistryCargoDefaults",
        )
        .when_query("view", "cargo_defaults"),
        definition(
            "GET",
            "/api/v1/registry/compat",
            "arsenal-envelopes.schema.json",
            "RegistryCompatPage",
        ),
    ]);
    contracts
}

/// One [`WHOLE_BODY_ROUTES`] line as a contract; a malformed line panics naming itself.
fn whole_body_route(line: &'static str) -> RouteContract {
    let mut fields = line.split(' ');
    let (Some(method), Some(path), Some(schema), None) =
        (fields.next(), fields.next(), fields.next(), fields.next())
    else {
        panic!("route contract row {line:?} is not `METHOD path schema`");
    };
    match schema.split_once('#') {
        Some((file, "")) => root(method, path, file),
        Some((file, pointer)) => match pointer.strip_prefix("/definitions/") {
            Some(name) => definition(method, path, file, name),
            None => panic!("route contract row {line:?} names no definition"),
        },
        None => panic!("route contract row {line:?} names no schema pointer"),
    }
}
