//! Route specs of the `operations_events` part: events and their mission attachments, event
//! access administration (policies, quotas, groups and rosters), the ORBAT read, the member
//! directory and the leave review console.
//!
//! Each spec supplies only what the framework cannot derive: the success status and contract,
//! the visibility probes, the domain refusals (stale access revisions, capacity and name
//! limits, foreign attachments and groups) and a reason for every dimension that does not
//! apply. The fixture keys the probes name are minted by `world/operations_events.rs`.

use contract_schema_types::operations::event_access_administration::{
    AccessChangeOutcome, EventAccessAdministration, ParticipantAccessExplanation,
};
use contract_schema_types::operations::event_hub::EventHub;
use contract_schema_types::operations::event_orbat::EventMissionOrbat;
use serde_json::{Value, json};

use super::super::contracts::round_trip;
use super::super::spec::{Actor, Change, Contract, Expect, Probe, Role, RouteSpec};

const ADMINISTERED: &str = "administrators manage every event: no event has an owner";
const DIRECTORY: &str = "the member directory addresses no owned resource";
const LEAVE_CONSOLE: &str = "the leave console reviews every member's request; none is owned \
     by the reviewer";
const LISTING: &str = "a listing addresses no single event; each viewer sees only the events \
     its access policies admit, probed per event on GET /api/v1/events/{id}";
const ACCESS_FILE: &str = "event-access-administration.schema.json";
const SCHEDULE_FILE: &str = "event-schedule.schema.json";
const LEAVE_FILE: &str = "leave-request.schema.json";
const UNKNOWN_MISSION: &str = "00000000-0000-4000-8000-000000000404";
const ADMIN: Actor = Actor::User(Role::Admin);
const LEADER: Actor = Actor::User(Role::Leader);
const ENLISTED: Actor = Actor::User(Role::Enlisted);

fn access_outcome() -> Contract {
    Contract::schema(ACCESS_FILE, "AccessChangeOutcome")
}

fn stale_revision_body() -> Probe {
    Probe::new("stale access revision", ADMIN)
        .merge_body(json!({"expected_access_revision": 7}))
        .expect_with(Expect::status(409).details_code("ACCESS_REVISION_CONFLICT"))
}

fn stale_revision_query() -> Probe {
    Probe::new("stale access revision", ADMIN)
        .query("expected_access_revision=7")
        .expect_with(Expect::status(409).details_code("ACCESS_REVISION_CONFLICT"))
}

fn unknown_field() -> Probe {
    Probe::new("unknown field", ADMIN)
        .merge_body(json!({"route_acceptance": true}))
        .expect(400)
}

fn one_grant() -> Value {
    json!({"conditions": [{"kind": "authenticated"}]})
}

fn policy(grants: Value) -> Value {
    json!({"policy": {"grants": grants}})
}

fn orbat(faction: &str, slots: Value) -> Value {
    json!({"orbat": [{"faction": faction, "callsign": "A1", "squad": "Alpha", "slots": slots}]})
}

/// A body-carrying access change: every one validates its body and fences on the revision.
fn access_change(key: &'static str, status: u16, request: &'static str) -> RouteSpec {
    RouteSpec::role(key, Role::Admin)
        .ok(status, access_outcome())
        .decodes(round_trip::<AccessChangeOutcome>)
        .request_contract(Contract::schema(ACCESS_FILE, request))
        .no_ownership(ADMINISTERED)
        .malformed(unknown_field())
        .malformed(
            Probe::new("missing revision", ADMIN)
                .change(Change::RemoveField("expected_access_revision"))
                .expect(400),
        )
        .boundary(stale_revision_body())
}

/// A policy change of the event, a squad or a seat: the policy's own limits apply.
fn policy_change(key: &'static str) -> RouteSpec {
    access_change(key, 200, "AccessPolicyChange")
        .malformed(
            Probe::new("unknown condition kind", ADMIN)
                .merge_body(policy(json!([{"conditions": [{"kind": "everyone"}]}])))
                .expect(400),
        )
        .boundary(
            Probe::new("33 alternative grants", ADMIN)
                .merge_body(policy(Value::Array(vec![one_grant(); 33])))
                .expect(400),
        )
        .boundary(
            Probe::new("32 alternative grants", ADMIN)
                .merge_body(policy(Value::Array(vec![one_grant(); 32]))),
        )
        .boundary(
            Probe::new("grant without conditions", ADMIN)
                .merge_body(policy(json!([{"conditions": []}])))
                .expect(400),
        )
}

/// A bodiless access removal: the revision travels in the query.
fn access_removal(key: &'static str) -> RouteSpec {
    RouteSpec::role(key, Role::Admin)
        .ok(200, access_outcome())
        .decodes(round_trip::<AccessChangeOutcome>)
        .no_ownership(ADMINISTERED)
        .malformed(Probe::new("missing revision", ADMIN).query("").expect(400))
        .malformed(
            Probe::new("non-integer revision", ADMIN)
                .query("expected_access_revision=latest")
                .expect(400),
        )
        .malformed(
            Probe::new("unknown query key", ADMIN)
                .query("expected_access_revision=0&force=true")
                .expect(400),
        )
        .boundary(stale_revision_query())
}

fn visibility(fixture: &'static str) -> [Probe; 3] {
    [
        Probe::new(
            "hidden from an account its policy does not admit",
            Actor::Peer(Role::Enlisted),
        )
        .fixture(fixture)
        .expect(404),
        Probe::new("shown to the account its policy names", ENLISTED).fixture(fixture),
        Probe::new("shown to an administrator", ADMIN).fixture(fixture),
    ]
}

fn page_limits(actor: Actor) -> [Probe; 3] {
    [
        Probe::new("limit over 100 falls back", actor)
            .query("limit=100000")
            .expect_with(
                Expect::success()
                    .max_items("/data", 100)
                    .json_at("/limit", json!(20)),
            ),
        Probe::new("limit of 100 is honoured", actor)
            .query("limit=100")
            .expect_with(Expect::success().json_at("/limit", json!(100))),
        Probe::new("negative offset falls back", actor)
            .query("offset=-1")
            .expect_with(Expect::success().json_at("/offset", json!(0))),
    ]
}

fn with_probes(
    spec: RouteSpec,
    dimension: fn(RouteSpec, Probe) -> RouteSpec,
    probes: [Probe; 3],
) -> RouteSpec {
    probes.into_iter().fold(spec, dimension)
}

fn event_specs() -> Vec<RouteSpec> {
    let list = RouteSpec::authenticated("GET /api/v1/events")
        .ok(200, Contract::schema(SCHEDULE_FILE, "EventListPage"))
        .no_ownership(LISTING)
        .malformed(
            Probe::new("non-numeric limit", ENLISTED)
                .query("limit=x")
                .expect(400),
        )
        .malformed(
            Probe::new("unknown scope", ENLISTED)
                .query("scope=sideways")
                .expect(400),
        );
    let hub = RouteSpec::authenticated("GET /api/v1/events/{id}")
        .ok(200, Contract::schema_root("event-hub.schema.json"))
        .decodes(round_trip::<EventHub>)
        .boundary(
            Probe::new("deleted event", ENLISTED)
                .fixture("deleted-event")
                .expect(404),
        );
    let orbat_read = RouteSpec::authenticated("GET /api/v1/event-missions/{emid}/orbat")
        .ok(200, Contract::schema_root("event-orbat.schema.json"))
        .decodes(round_trip::<EventMissionOrbat>);
    vec![
        with_probes(list, RouteSpec::boundary, page_limits(ENLISTED)),
        RouteSpec::role("POST /api/v1/events", Role::Admin)
            .ok(201, Contract::schema(SCHEDULE_FILE, "Event"))
            .request_contract(Contract::schema(SCHEDULE_FILE, "EventCreation"))
            .no_ownership(ADMINISTERED)
            .malformed(
                Probe::new("missing start time", ADMIN)
                    .change(Change::RemoveField("start_time"))
                    .expect(400),
            )
            .malformed(
                Probe::new("unknown status", ADMIN)
                    .merge_body(json!({"status": "sideways"}))
                    .expect(400),
            )
            .boundary(Probe::new("capacity of 256", ADMIN).merge_body(json!({"max_slots": 256})))
            .boundary(
                Probe::new("capacity of 257", ADMIN)
                    .merge_body(json!({"max_slots": 257}))
                    .expect(400),
            )
            .boundary(
                Probe::new("created already completed", ADMIN)
                    .merge_body(json!({"status": "completed"}))
                    .expect(400),
            )
            .boundary(
                Probe::new("blank name override", ADMIN)
                    .merge_body(json!({"name_override": "   "}))
                    .expect(400),
            ),
        with_probes(hub, RouteSpec::ownership, visibility("restricted-event")),
        RouteSpec::role("PATCH /api/v1/events/{id}", Role::Admin)
            .ok(200, Contract::schema(SCHEDULE_FILE, "Event"))
            .request_contract(Contract::schema(SCHEDULE_FILE, "EventChange"))
            .no_ownership(ADMINISTERED)
            .malformed(
                Probe::new("unparseable start time", ADMIN)
                    .merge_body(json!({"start_time": "tomorrow"}))
                    .expect(400),
            )
            .malformed(
                Probe::new("unknown status", ADMIN)
                    .merge_body(json!({"status": "sideways"}))
                    .expect(400),
            )
            .boundary(
                Probe::new("capacity of 257", ADMIN)
                    .merge_body(json!({"max_slots": 257}))
                    .expect(400),
            )
            .boundary(
                Probe::new("deleted event", ADMIN)
                    .fixture("deleted-event")
                    .body(json!({"name_override": "Route acceptance renamed"}))
                    .expect(404),
            ),
        RouteSpec::role("DELETE /api/v1/events/{id}", Role::Admin)
            .ok(204, Contract::NoBody)
            .no_ownership(ADMINISTERED)
            .boundary(
                Probe::new("deleted event", ADMIN)
                    .fixture("deleted-event")
                    .expect(404),
            ),
        RouteSpec::role("POST /api/v1/events/{id}/missions", Role::Admin)
            .ok(201, Contract::schema(SCHEDULE_FILE, "EventMission"))
            .request_contract(Contract::schema(SCHEDULE_FILE, "EventMissionAttachment"))
            .no_ownership(ADMINISTERED)
            .malformed(
                Probe::new("non-uuid mission id", ADMIN)
                    .merge_body(json!({"mission_id": "mission-1"}))
                    .expect(400),
            )
            .malformed(
                Probe::new("missing start time", ADMIN)
                    .change(Change::RemoveField("start_time"))
                    .expect(400),
            )
            .boundary(
                Probe::new("unknown mission", ADMIN)
                    .merge_body(json!({"mission_id": UNKNOWN_MISSION}))
                    .expect(404),
            )
            .boundary(
                Probe::new("archived mission", ADMIN)
                    .fixture("archived-mission")
                    .expect(409),
            )
            .boundary(
                Probe::new("ORBAT without seats", ADMIN)
                    .merge_body(orbat("BLUFOR", json!([])))
                    .expect(400),
            )
            .boundary(
                Probe::new("padded ORBAT faction", ADMIN)
                    .merge_body(orbat(" BLUFOR", json!([{"role": "Rifleman"}])))
                    .expect(400),
            ),
        RouteSpec::role("DELETE /api/v1/events/{id}/missions/{emid}", Role::Admin)
            .ok(204, Contract::NoBody)
            .no_ownership(ADMINISTERED)
            .boundary(
                Probe::new("attachment of another event", ADMIN)
                    .fixture("attachment-of-another-event")
                    .expect(404),
            ),
        with_probes(
            orbat_read,
            RouteSpec::ownership,
            visibility("restricted-orbat"),
        ),
    ]
}

fn access_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::role("GET /api/v1/events/{id}/access", Role::Admin)
            .ok(
                200,
                Contract::schema(ACCESS_FILE, "EventAccessAdministration"),
            )
            .decodes(round_trip::<EventAccessAdministration>)
            .no_ownership(ADMINISTERED)
            .boundary(
                Probe::new("deleted event", ADMIN)
                    .fixture("deleted-event")
                    .expect(404),
            ),
        RouteSpec::role("GET /api/v1/events/{id}/access/participants", Role::Admin)
            .ok(
                200,
                Contract::schema_items(ACCESS_FILE, "ParticipantAccessExplanation"),
            )
            .decodes(round_trip::<Vec<ParticipantAccessExplanation>>)
            .no_ownership(ADMINISTERED)
            .boundary(
                Probe::new("deleted event", ADMIN)
                    .fixture("deleted-event")
                    .expect(404),
            ),
        policy_change("PUT /api/v1/events/{id}/access-policy"),
        access_change(
            "PUT /api/v1/events/{id}/reservation-quotas",
            200,
            "ReservationQuotaChange",
        )
        .malformed(
            Probe::new("missing open pool", ADMIN)
                .merge_body(json!({"reservation_quotas": {
                    "member": {"seats": null, "opens_at": "2026-01-01T00:00:00Z"},
                    "guest": {"seats": 0, "opens_at": "2026-01-01T00:00:00Z"}}}))
                .expect(400),
        )
        .boundary(
            Probe::new("seat limit above 32 bits", ADMIN)
                .merge_body(json!({"reservation_quotas": {
                    "member": {"seats": 4_294_967_296_u64, "opens_at": "2026-01-01T00:00:00Z"},
                    "guest": {"seats": 0, "opens_at": "2026-01-01T00:00:00Z"},
                    "open": {"seats": 0, "opens_at": "2026-01-01T00:00:00Z"}}}))
                .expect(400),
        ),
        access_change("POST /api/v1/events/{id}/groups", 201, "EventGroupCreation")
            .malformed(
                Probe::new("unknown source kind", ADMIN)
                    .merge_body(json!({"source": {"kind": "everyone"}}))
                    .expect(400),
            )
            .boundary(
                Probe::new("name of 128 bytes", ADMIN).merge_body(json!({"name": "g".repeat(128)})),
            )
            .boundary(
                Probe::new("name of 129 bytes", ADMIN)
                    .merge_body(json!({"name": "g".repeat(129)}))
                    .expect(400),
            )
            .boundary(
                Probe::new("padded name", ADMIN)
                    .merge_body(json!({"name": " Guests"}))
                    .expect(400),
            ),
        access_change(
            "PATCH /api/v1/events/{id}/groups/{groupId}",
            200,
            "EventGroupChange",
        )
        .boundary(
            Probe::new("group of another event", ADMIN)
                .fixture("group-of-another-event")
                .expect(404),
        ),
        access_removal("DELETE /api/v1/events/{id}/groups/{groupId}").boundary(
            Probe::new("group named by the event policy", ADMIN)
                .fixture("group-named-by-the-event-policy")
                .expect(409),
        ),
        access_change(
            "PUT /api/v1/events/{id}/groups/{groupId}/members/{discordId}",
            200,
            "RosterMemberChange",
        )
        .text_param("discordId")
        .boundary(
            Probe::new("unknown account", ADMIN)
                .param("discordId", "route-acceptance-unknown-account")
                .expect(404),
        )
        .boundary(
            Probe::new("partner-guild group has no roster", ADMIN)
                .fixture("partner-guild-group")
                .expect(409),
        ),
        access_removal("DELETE /api/v1/events/{id}/groups/{groupId}/members/{discordId}")
            .text_param("discordId")
            .boundary(
                Probe::new("account not on the roster", ADMIN)
                    .fixture("account-not-on-the-roster")
                    .expect(404),
            ),
        policy_change("PUT /api/v1/event-missions/{emid}/squads/{faction}/{squad}/access-policy")
            .text_param("faction")
            .text_param("squad")
            .boundary(
                Probe::new("unknown squad", ADMIN)
                    .param("squad", "Bravo")
                    .expect(404),
            ),
        access_removal(
            "DELETE /api/v1/event-missions/{emid}/squads/{faction}/{squad}/access-policy",
        )
        .text_param("faction")
        .text_param("squad")
        .boundary(
            Probe::new("unknown squad", ADMIN)
                .param("squad", "Bravo")
                .expect(404),
        ),
        policy_change("PUT /api/v1/event-missions/{emid}/slots/{slotId}/access-policy"),
        access_removal("DELETE /api/v1/event-missions/{emid}/slots/{slotId}/access-policy"),
    ]
}

fn directory_and_leave_specs() -> Vec<RouteSpec> {
    let members = RouteSpec::role("GET /api/v1/members", Role::Leader)
        .ok(
            200,
            Contract::schema("member-directory.schema.json", "MemberSearchPage"),
        )
        .no_ownership(DIRECTORY)
        .malformed(
            Probe::new("non-numeric limit", LEADER)
                .query("limit=x")
                .expect(400),
        )
        .boundary(
            Probe::new("search of 200 characters matches nobody", LEADER)
                .query(format!("q={}", "z".repeat(200)))
                .expect_with(Expect::success().json_at("/total", json!(0))),
        );
    let leave_queue = RouteSpec::role("GET /api/v1/admin/leave-requests", Role::Admin)
        .ok(200, Contract::schema(LEAVE_FILE, "LeaveRequestPage"))
        .no_ownership(LEAVE_CONSOLE)
        .malformed(
            Probe::new("non-numeric limit", ADMIN)
                .query("limit=x")
                .expect(400),
        );
    vec![
        with_probes(members, RouteSpec::boundary, page_limits(LEADER)),
        with_probes(leave_queue, RouteSpec::boundary, page_limits(ADMIN)),
        RouteSpec::role("PATCH /api/v1/admin/leave-requests/{id}", Role::Admin)
            .ok(200, Contract::schema(LEAVE_FILE, "LeaveReviewOutcome"))
            .request_contract(Contract::schema(LEAVE_FILE, "LeaveReview"))
            .no_ownership(LEAVE_CONSOLE)
            .malformed(
                Probe::new("unknown decision", ADMIN)
                    .merge_body(json!({"status": "maybe"}))
                    .expect(400),
            )
            .malformed(
                Probe::new("missing decision", ADMIN)
                    .change(Change::RemoveField("status"))
                    .expect(400),
            )
            .boundary(
                Probe::new("denial", ADMIN)
                    .merge_body(json!({"status": "denied"}))
                    .expect_with(Expect::success().json_at("/status", json!("denied"))),
            ),
    ]
}

/// Every spec of the `operations_events` part.
pub(crate) fn specs() -> Vec<RouteSpec> {
    [event_specs(), access_specs(), directory_and_leave_specs()]
        .into_iter()
        .flatten()
        .collect()
}
