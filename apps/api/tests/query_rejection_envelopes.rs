//! A malformed or missing query parameter answers 400 in the `{error}` envelope on every route
//! that parses a typed query.
//!
//! **Role:** one case per handler: the route's least caller — a signed-in actor of its least role,
//! or an anonymous caller on the development-only equipment data viewer reads — sends one
//! malformed parameter (a non-numeric `limit`, `offset`, `field_id` or
//! `expected_access_revision`) or omits a required one (the credential revocation `reason`), and
//! the answer is 400 with a JSON object whose `error` carries the rejection's own reason for that
//! parameter (`<parameter>: <reason>`, or `missing field` and the quoted name for an omitted one).
//! **Position:** boots `core::http_router::router` over this binary's own test database with the
//! development test configuration and mints each actor's session through `common::access_token`;
//! the handlers reach the envelope through `ApiError::from_query_rejection`.
//! **Signals & state:** none beyond the per-binary database; every actor is a suite-owned row
//! named after its area, and every request carries a fresh synthetic peer so the rate limiter
//! never answers in the handler's place.
//! **Invariants:** a plain-text 400 (axum's bare `Query` rejection) fails the case; so does any
//! status but 400, which is what an unreached handler (401, 403, 404) would answer, and so does a
//! fixed message that names or describes the parameter without the rejection's reason.

mod common;

use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU32, Ordering};

use api::core::application_state::AppState;
use api::core::configuration::Config;
use api::core::database;
use api::core::http_router;
use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use tower::ServiceExt;

const SUITE: &str = "query_rejection_envelopes";

/// Any well-formed id: every case's handler answers the query rejection before it looks a path id
/// up.
const PATH_ID: &str = "5f0c4b9e-3a57-4d8e-9a4c-2f1e6d7b8a90";

/// The caller a case sends its request as.
enum Caller {
    /// No bearer: the route takes no auth extractor.
    Anonymous,
    /// A signed-in actor of this role, the least one that reaches the handler.
    Role(&'static str),
}

/// What is wrong with the case's query, naming the parameter the rejection is about.
enum Refusal {
    /// The parameter is present but does not decode; the reason reads `<parameter>: <reason>`.
    Malformed(&'static str),
    /// A required parameter is absent; the reason reads ``missing field `<parameter>` ``.
    Missing(&'static str),
}

impl Refusal {
    /// The fragment of the rejection's own reason that the error envelope must carry.
    fn reason_fragment(&self) -> String {
        match self {
            Self::Malformed(parameter) => format!("{parameter}: "),
            Self::Missing(parameter) => format!("missing field `{parameter}`"),
        }
    }
}

/// One request: the area its test is named after, the caller that reaches the handler, its
/// method, its path with the malformed or missing parameter, and what the rejection is about.
struct QueryCase {
    area: &'static str,
    caller: Caller,
    method: &'static str,
    path: &'static str,
    refusal: Refusal,
}

const CASES: [QueryCase; 30] = [
    QueryCase {
        area: "events",
        caller: Caller::Role("enlisted"),
        method: "GET",
        path: "/api/v1/events?limit=many",
        refusal: Refusal::Malformed("limit"),
    },
    QueryCase {
        area: "missions",
        caller: Caller::Role("enlisted"),
        method: "GET",
        path: "/api/v1/missions?limit=many",
        refusal: Refusal::Malformed("limit"),
    },
    QueryCase {
        area: "members",
        caller: Caller::Role("leader"),
        method: "GET",
        path: "/api/v1/members?limit=many",
        refusal: Refusal::Malformed("limit"),
    },
    QueryCase {
        area: "leaderboards",
        caller: Caller::Role("enlisted"),
        method: "GET",
        path: "/api/v1/leaderboards?limit=many",
        refusal: Refusal::Malformed("limit"),
    },
    QueryCase {
        area: "leave_requests",
        caller: Caller::Role("admin"),
        method: "GET",
        path: "/api/v1/admin/leave-requests?limit=many",
        refusal: Refusal::Malformed("limit"),
    },
    QueryCase {
        area: "approvals",
        caller: Caller::Role("admin"),
        method: "GET",
        path: "/api/v1/approvals?limit=many",
        refusal: Refusal::Malformed("limit"),
    },
    QueryCase {
        area: "registry",
        caller: Caller::Role("mission_maker"),
        method: "GET",
        path: "/api/v1/registry?limit=many",
        refusal: Refusal::Malformed("limit"),
    },
    QueryCase {
        area: "registry_compat",
        caller: Caller::Role("mission_maker"),
        method: "GET",
        path: "/api/v1/registry/compat?limit=many",
        refusal: Refusal::Malformed("limit"),
    },
    QueryCase {
        area: "server_deployments",
        caller: Caller::Role("admin"),
        method: "GET",
        path: "/api/v1/servers/{id}/deployments?limit=many",
        refusal: Refusal::Malformed("limit"),
    },
    QueryCase {
        area: "server_commands",
        caller: Caller::Role("admin"),
        method: "GET",
        path: "/api/v1/servers/{id}/commands?offset=many",
        refusal: Refusal::Malformed("offset"),
    },
    QueryCase {
        area: "match_events",
        caller: Caller::Role("enlisted"),
        method: "GET",
        path: "/api/v1/matches/{id}/events?limit=many",
        refusal: Refusal::Malformed("limit"),
    },
    QueryCase {
        area: "audit_logs",
        caller: Caller::Role("admin"),
        method: "GET",
        path: "/api/v1/admin/audit-logs?limit=many",
        refusal: Refusal::Malformed("limit"),
    },
    QueryCase {
        area: "audit_logs_export",
        caller: Caller::Role("admin"),
        method: "GET",
        path: "/api/v1/admin/audit-logs/export.csv?limit=many",
        refusal: Refusal::Malformed("limit"),
    },
    QueryCase {
        area: "equipment_status",
        caller: Caller::Anonymous,
        method: "GET",
        path: "/api/v1/debug/equipment-data/status?field_id=many",
        refusal: Refusal::Malformed("field_id"),
    },
    QueryCase {
        area: "equipment_overview",
        caller: Caller::Anonymous,
        method: "GET",
        path: "/api/v1/debug/equipment-data/overview?field_id=many",
        refusal: Refusal::Malformed("field_id"),
    },
    QueryCase {
        area: "equipment_resources",
        caller: Caller::Anonymous,
        method: "GET",
        path: "/api/v1/debug/equipment-data/resources?field_id=many",
        refusal: Refusal::Malformed("field_id"),
    },
    QueryCase {
        area: "equipment_relationships",
        caller: Caller::Anonymous,
        method: "GET",
        path: "/api/v1/debug/equipment-data/relationships?field_id=many",
        refusal: Refusal::Malformed("field_id"),
    },
    QueryCase {
        area: "equipment_fields",
        caller: Caller::Anonymous,
        method: "GET",
        path: "/api/v1/debug/equipment-data/fields?field_id=many",
        refusal: Refusal::Malformed("field_id"),
    },
    QueryCase {
        area: "equipment_resource_cards",
        caller: Caller::Anonymous,
        method: "GET",
        path: "/api/v1/debug/equipment-data/resource-cards?field_id=many",
        refusal: Refusal::Malformed("field_id"),
    },
    QueryCase {
        area: "equipment_selection",
        caller: Caller::Anonymous,
        method: "GET",
        path: "/api/v1/debug/equipment-data/selection?field_id=many",
        refusal: Refusal::Malformed("field_id"),
    },
    QueryCase {
        area: "equipment_containers",
        caller: Caller::Anonymous,
        method: "GET",
        path: "/api/v1/debug/equipment-data/containers?field_id=many",
        refusal: Refusal::Malformed("field_id"),
    },
    QueryCase {
        area: "equipment_properties",
        caller: Caller::Anonymous,
        method: "GET",
        path: "/api/v1/debug/equipment-data/properties?field_id=many",
        refusal: Refusal::Malformed("field_id"),
    },
    QueryCase {
        area: "equipment_values",
        caller: Caller::Anonymous,
        method: "GET",
        path: "/api/v1/debug/equipment-data/values?field_id=many",
        refusal: Refusal::Malformed("field_id"),
    },
    QueryCase {
        area: "equipment_documents",
        caller: Caller::Anonymous,
        method: "GET",
        path: "/api/v1/debug/equipment-data/documents?field_id=many",
        refusal: Refusal::Malformed("field_id"),
    },
    QueryCase {
        area: "equipment_download",
        caller: Caller::Anonymous,
        method: "GET",
        path: "/api/v1/debug/equipment-data/download?field_id=many",
        refusal: Refusal::Malformed("field_id"),
    },
    QueryCase {
        area: "server_credential_revocation",
        caller: Caller::Role("admin"),
        method: "DELETE",
        path: "/api/v1/servers/{id}/credentials/{id}",
        refusal: Refusal::Missing("reason"),
    },
    QueryCase {
        area: "squad_access_policy_removal",
        caller: Caller::Role("admin"),
        method: "DELETE",
        path: "/api/v1/event-missions/{id}/squads/a/b/access-policy?expected_access_revision=many",
        refusal: Refusal::Malformed("expected_access_revision"),
    },
    QueryCase {
        area: "slot_access_policy_removal",
        caller: Caller::Role("admin"),
        method: "DELETE",
        path: "/api/v1/event-missions/{id}/slots/{id}/access-policy?expected_access_revision=many",
        refusal: Refusal::Malformed("expected_access_revision"),
    },
    QueryCase {
        area: "event_group_removal",
        caller: Caller::Role("admin"),
        method: "DELETE",
        path: "/api/v1/events/{id}/groups/{id}?expected_access_revision=many",
        refusal: Refusal::Malformed("expected_access_revision"),
    },
    QueryCase {
        area: "event_group_member_removal",
        caller: Caller::Role("admin"),
        method: "DELETE",
        path: "/api/v1/events/{id}/groups/{id}/members/1?expected_access_revision=many",
        refusal: Refusal::Malformed("expected_access_revision"),
    },
];

static PEER: AtomicU32 = AtomicU32::new(1);

/// A synthetic client address no other request in this binary has used.
fn next_peer() -> SocketAddr {
    let [_, b, c, d] = PEER.fetch_add(1, Ordering::Relaxed).to_be_bytes();
    SocketAddr::from((IpAddr::from([10, b, c, d]), 42000))
}

/// Send the case's malformed or incomplete query as its caller and require the 400 `{error}`
/// envelope carrying the rejection's own reason.
async fn assert_query_rejection_envelope(area: &str) {
    let case = CASES
        .iter()
        .find(|case| case.area == area)
        .unwrap_or_else(|| panic!("no query case for area `{area}`"));
    let url = common::require_test_database_url()
        .expect("the per-binary test database is provisioned before any case runs");
    let pool = database::connect(&url).await.expect("connect");
    let state = AppState::new(pool, Config::for_tests(url, "query-rejection-secret"));
    let method = case.method;
    let path = case.path.replace("{id}", PATH_ID);
    let mut builder = Request::builder().method(method).uri(&path);
    let caller = match case.caller {
        Caller::Anonymous => "an anonymous caller",
        Caller::Role(role) => {
            let actor = format!("query-rejection-{}", case.area);
            let token = common::access_token(&state, SUITE, &actor, role, true).await;
            builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
            role
        }
    };
    let mut request = builder.body(Body::empty()).expect("request");
    request.extensions_mut().insert(ConnectInfo(next_peer()));
    let response = http_router::router(state)
        .oneshot(request)
        .await
        .expect("infallible");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let text = String::from_utf8_lossy(&bytes);
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "{method} {path} as {caller}: expected 400, body {text}"
    );
    let body: Value = serde_json::from_slice(&bytes).unwrap_or_else(|e| {
        panic!("{method} {path}: the 400 is not the JSON error envelope ({e}); body: {text}")
    });
    let error = body
        .get("error")
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("{method} {path}: the 400 has no string `error`; body: {body}"));
    // The rejection's reason names the parameter and what is wrong with it; a fixed message
    // that merely names or describes the parameter does not say which one failed or why.
    let fragment = case.refusal.reason_fragment();
    assert!(
        error.contains(&fragment),
        "{method} {path}: the error does not carry the rejection's reason `{fragment}`: {error}"
    );
}

#[tokio::test]
async fn contract_parity_query_rejections_answer_the_error_envelope_events() {
    assert_query_rejection_envelope("events").await;
}

#[tokio::test]
async fn contract_parity_query_rejections_answer_the_error_envelope_missions() {
    assert_query_rejection_envelope("missions").await;
}

#[tokio::test]
async fn contract_parity_query_rejections_answer_the_error_envelope_members() {
    assert_query_rejection_envelope("members").await;
}

#[tokio::test]
async fn contract_parity_query_rejections_answer_the_error_envelope_leaderboards() {
    assert_query_rejection_envelope("leaderboards").await;
}

#[tokio::test]
async fn contract_parity_query_rejections_answer_the_error_envelope_leave_requests() {
    assert_query_rejection_envelope("leave_requests").await;
}

#[tokio::test]
async fn contract_parity_query_rejections_answer_the_error_envelope_approvals() {
    assert_query_rejection_envelope("approvals").await;
}

#[tokio::test]
async fn contract_parity_query_rejections_answer_the_error_envelope_registry() {
    assert_query_rejection_envelope("registry").await;
}

#[tokio::test]
async fn contract_parity_query_rejections_answer_the_error_envelope_registry_compat() {
    assert_query_rejection_envelope("registry_compat").await;
}

#[tokio::test]
async fn contract_parity_query_rejections_answer_the_error_envelope_server_deployments() {
    assert_query_rejection_envelope("server_deployments").await;
}

#[tokio::test]
async fn contract_parity_query_rejections_answer_the_error_envelope_server_commands() {
    assert_query_rejection_envelope("server_commands").await;
}

#[tokio::test]
async fn contract_parity_query_rejections_answer_the_error_envelope_match_events() {
    assert_query_rejection_envelope("match_events").await;
}

#[tokio::test]
async fn contract_parity_query_rejections_answer_the_error_envelope_audit_logs() {
    assert_query_rejection_envelope("audit_logs").await;
}

#[tokio::test]
async fn contract_parity_query_rejections_answer_the_error_envelope_audit_logs_export() {
    assert_query_rejection_envelope("audit_logs_export").await;
}

#[tokio::test]
async fn contract_parity_query_rejections_answer_the_error_envelope_equipment_status() {
    assert_query_rejection_envelope("equipment_status").await;
}

#[tokio::test]
async fn contract_parity_query_rejections_answer_the_error_envelope_equipment_overview() {
    assert_query_rejection_envelope("equipment_overview").await;
}

#[tokio::test]
async fn contract_parity_query_rejections_answer_the_error_envelope_equipment_resources() {
    assert_query_rejection_envelope("equipment_resources").await;
}

#[tokio::test]
async fn contract_parity_query_rejections_answer_the_error_envelope_equipment_relationships() {
    assert_query_rejection_envelope("equipment_relationships").await;
}

#[tokio::test]
async fn contract_parity_query_rejections_answer_the_error_envelope_equipment_fields() {
    assert_query_rejection_envelope("equipment_fields").await;
}

#[tokio::test]
async fn contract_parity_query_rejections_answer_the_error_envelope_equipment_resource_cards() {
    assert_query_rejection_envelope("equipment_resource_cards").await;
}

#[tokio::test]
async fn contract_parity_query_rejections_answer_the_error_envelope_equipment_selection() {
    assert_query_rejection_envelope("equipment_selection").await;
}

#[tokio::test]
async fn contract_parity_query_rejections_answer_the_error_envelope_equipment_containers() {
    assert_query_rejection_envelope("equipment_containers").await;
}

#[tokio::test]
async fn contract_parity_query_rejections_answer_the_error_envelope_equipment_properties() {
    assert_query_rejection_envelope("equipment_properties").await;
}

#[tokio::test]
async fn contract_parity_query_rejections_answer_the_error_envelope_equipment_values() {
    assert_query_rejection_envelope("equipment_values").await;
}

#[tokio::test]
async fn contract_parity_query_rejections_answer_the_error_envelope_equipment_documents() {
    assert_query_rejection_envelope("equipment_documents").await;
}

#[tokio::test]
async fn contract_parity_query_rejections_answer_the_error_envelope_equipment_download() {
    assert_query_rejection_envelope("equipment_download").await;
}

#[tokio::test]
async fn contract_parity_query_rejections_answer_the_error_envelope_server_credential_revocation() {
    assert_query_rejection_envelope("server_credential_revocation").await;
}

#[tokio::test]
async fn contract_parity_query_rejections_answer_the_error_envelope_squad_access_policy_removal() {
    assert_query_rejection_envelope("squad_access_policy_removal").await;
}

#[tokio::test]
async fn contract_parity_query_rejections_answer_the_error_envelope_slot_access_policy_removal() {
    assert_query_rejection_envelope("slot_access_policy_removal").await;
}

#[tokio::test]
async fn contract_parity_query_rejections_answer_the_error_envelope_event_group_removal() {
    assert_query_rejection_envelope("event_group_removal").await;
}

#[tokio::test]
async fn contract_parity_query_rejections_answer_the_error_envelope_event_group_member_removal() {
    assert_query_rejection_envelope("event_group_member_removal").await;
}
