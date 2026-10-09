//! Route specs of the `operations_events` part: events and their mission attachments, event
//! access administration (policies, quotas, groups and rosters), the ORBAT read, the member
//! directory and the leave review console.
//!
//! Each spec supplies only what the framework cannot derive: the success status and contract,
//! the authorized caller and fixture where the defaults do not fit, and the unauthorized probes
//! and overrides the access class does not imply.

use super::super::spec::{Contract, Role, RouteSpec};

const ACCESS_FILE: &str = "event-access-administration.schema.json";
const SCHEDULE_FILE: &str = "event-schedule.schema.json";
const LEAVE_FILE: &str = "leave-request.schema.json";

fn access_outcome() -> Contract {
    Contract::schema(ACCESS_FILE, "AccessChangeOutcome")
}

/// A body-carrying access change: every one validates its body and fences on the revision.
fn access_change(key: &'static str, status: u16, _request: &'static str) -> RouteSpec {
    RouteSpec::role(key, Role::Admin).ok(status, access_outcome())
}

/// A policy change of the event, a squad or a seat: the policy's own limits apply.
fn policy_change(key: &'static str) -> RouteSpec {
    access_change(key, 200, "AccessPolicyChange")
}

/// A bodiless access removal: the revision travels in the query.
fn access_removal(key: &'static str) -> RouteSpec {
    RouteSpec::role(key, Role::Admin).ok(200, access_outcome())
}

fn event_specs() -> Vec<RouteSpec> {
    let list = RouteSpec::authenticated("GET /api/v1/events")
        .ok(200, Contract::schema(SCHEDULE_FILE, "EventListPage"));
    let hub = RouteSpec::authenticated("GET /api/v1/events/{id}")
        .ok(200, Contract::schema_root("event-hub.schema.json"));
    let orbat_read = RouteSpec::authenticated("GET /api/v1/event-missions/{emid}/orbat")
        .ok(200, Contract::schema_root("event-orbat.schema.json"));
    vec![
        list,
        RouteSpec::role("POST /api/v1/events", Role::Admin)
            .ok(201, Contract::schema(SCHEDULE_FILE, "Event")),
        hub,
        RouteSpec::role("PATCH /api/v1/events/{id}", Role::Admin)
            .ok(200, Contract::schema(SCHEDULE_FILE, "Event")),
        RouteSpec::role("DELETE /api/v1/events/{id}", Role::Admin).ok(204, Contract::NoBody),
        RouteSpec::role("POST /api/v1/events/{id}/missions", Role::Admin)
            .ok(201, Contract::schema(SCHEDULE_FILE, "EventMission")),
        RouteSpec::role("DELETE /api/v1/events/{id}/missions/{emid}", Role::Admin)
            .ok(204, Contract::NoBody),
        orbat_read,
    ]
}

fn access_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::role("GET /api/v1/events/{id}/access", Role::Admin).ok(
            200,
            Contract::schema(ACCESS_FILE, "EventAccessAdministration"),
        ),
        RouteSpec::role("GET /api/v1/events/{id}/access/participants", Role::Admin).ok(
            200,
            Contract::schema_items(ACCESS_FILE, "ParticipantAccessExplanation"),
        ),
        policy_change("PUT /api/v1/events/{id}/access-policy"),
        access_change(
            "PUT /api/v1/events/{id}/reservation-quotas",
            200,
            "ReservationQuotaChange",
        ),
        access_change("POST /api/v1/events/{id}/groups", 201, "EventGroupCreation"),
        access_change(
            "PATCH /api/v1/events/{id}/groups/{groupId}",
            200,
            "EventGroupChange",
        ),
        access_removal("DELETE /api/v1/events/{id}/groups/{groupId}"),
        access_change(
            "PUT /api/v1/events/{id}/groups/{groupId}/members/{discordId}",
            200,
            "RosterMemberChange",
        ),
        access_removal("DELETE /api/v1/events/{id}/groups/{groupId}/members/{discordId}"),
        policy_change("PUT /api/v1/event-missions/{emid}/squads/{faction}/{squad}/access-policy"),
        access_removal(
            "DELETE /api/v1/event-missions/{emid}/squads/{faction}/{squad}/access-policy",
        ),
        policy_change("PUT /api/v1/event-missions/{emid}/slots/{slotId}/access-policy"),
        access_removal("DELETE /api/v1/event-missions/{emid}/slots/{slotId}/access-policy"),
    ]
}

fn directory_and_leave_specs() -> Vec<RouteSpec> {
    let members = RouteSpec::role("GET /api/v1/members", Role::Leader).ok(
        200,
        Contract::schema("member-directory.schema.json", "MemberSearchPage"),
    );
    let leave_queue = RouteSpec::role("GET /api/v1/admin/leave-requests", Role::Admin)
        .ok(200, Contract::schema(LEAVE_FILE, "LeaveRequestPage"));
    vec![
        members,
        leave_queue,
        RouteSpec::role("PATCH /api/v1/admin/leave-requests/{id}", Role::Admin)
            .ok(200, Contract::schema(LEAVE_FILE, "LeaveReviewOutcome")),
    ]
}

/// Every spec of the `operations_events` part.
pub(crate) fn specs() -> Vec<RouteSpec> {
    [event_specs(), access_specs(), directory_and_leave_specs()]
        .into_iter()
        .flatten()
        .collect()
}
