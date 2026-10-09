//! Route specs of the `identity_and_core` part: the router's own routes, the identity and
//! access routes, and every `/me*` route.
//!
//! Each spec supplies only what the framework cannot derive: the success status and contract,
//! the authorized caller and fixture where the defaults do not fit, and the unauthorized probes
//! and overrides the access class does not imply.

use serde_json::json;

use super::super::spec::{Actor, Contract, Executor, Expect, Probe, RouteSpec};

fn file_mount(key: &'static str, content_type: &'static str) -> RouteSpec {
    RouteSpec::public(key).ok(200, Contract::Binary(content_type))
}

fn own_read(key: &'static str, contract: Contract) -> RouteSpec {
    RouteSpec::authenticated(key).ok(200, contract)
}

fn callback_refusal(name: &str, fixture: &'static str, reason: &'static str) -> Probe {
    Probe::new(name, Actor::Anonymous)
        .fixture(fixture)
        .expect_with(Expect::status(302).location_contains(reason))
}

/// Every spec of the `identity_and_core` part.
pub(crate) fn specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::public("GET /healthz")
            .ok(
                200,
                Contract::schema("service-health.schema.json", "PublicHealth"),
            )
            .unauthorized(Probe::new(
                "wrong token gets the public shape",
                Actor::WrongObservability,
            )),
        RouteSpec::observability("GET /metrics").ok(200, Contract::Binary("text/plain")),
        file_mount("GET /uploads/{*path}", "image/png"),
        file_mount("GET /map-assets/{*path}", "application/json"),
        file_mount("GET /map-assets/glyphs/{*path}", "application/json"),
        RouteSpec::development_only("GET /api/v1/auth/dev-login")
            .ok_redirect(302, "#access_token="),
        RouteSpec::public("GET /api/v1/auth/discord/login")
            .ok_redirect(307, "client_id=route-acceptance-client"),
        RouteSpec::public("GET /api/v1/auth/discord/callback")
            .ok_redirect(302, "#access_token=")
            .unauthorized(callback_refusal(
                "forged state",
                "callback-forged-state",
                "error=invalid_state",
            )),
        RouteSpec::public("POST /api/v1/auth/refresh")
            .ok(
                200,
                Contract::schema("session-token.schema.json", "SessionTokenPair"),
            )
            .unauthorized(
                Probe::new("unknown refresh token", Actor::Anonymous)
                    .body(json!({"refresh_token": "route-acceptance-unknown"}))
                    .expect(401),
            ),
        RouteSpec::public("POST /api/v1/auth/logout").ok(204, Contract::NoBody),
        own_read(
            "GET /api/v1/me",
            Contract::schema_root("current-profile.schema.json"),
        ),
        own_read(
            "PATCH /api/v1/me",
            Contract::schema("profile-update.schema.json", "UpdatedProfile"),
        ),
        RouteSpec::authenticated("POST /api/v1/me/link")
            .ok(201, Contract::schema("arma-link.schema.json", "LinkCode")),
        own_read(
            "GET /api/v1/me/link/status",
            Contract::schema("arma-link.schema.json", "LinkStatus"),
        ),
        RouteSpec::authenticated("DELETE /api/v1/me/link").ok(
            200,
            Contract::schema("arma-link.schema.json", "LinkRemoval"),
        ),
        RouteSpec::machine("POST /api/v1/ingest/link-confirm", Executor::ModRuntime).ok(
            200,
            Contract::schema("arma-link.schema.json", "LinkConfirmation"),
        ),
        own_read(
            "GET /api/v1/me/deployments",
            Contract::schema("service-record.schema.json", "ServiceRecord"),
        ),
        own_read(
            "GET /api/v1/me/leave-requests",
            Contract::schema("leave-request.schema.json", "LeaveRequestList"),
        ),
        RouteSpec::authenticated("POST /api/v1/me/leave-requests").ok(
            201,
            Contract::schema("leave-request.schema.json", "LeaveRequest"),
        ),
    ]
}
