//! Route specs of the `identity_and_core` part: the router's own routes, the identity and
//! access routes, and every `/me*` route.
//!
//! The reference spec file: each spec supplies only what the framework cannot derive — the
//! success status and contract, the fixture-dependent probes, and a reason for every dimension
//! that does not apply.

use contract_schema_types::identity_and_access::current_profile::CurrentProfileResponse;
use serde_json::json;

use super::super::contracts::round_trip;
use super::super::spec::{Actor, Contract, Dimension, Executor, Expect, Probe, Role, RouteSpec};

const SELF_SCOPED: &str = "self-scoped: the route acts on the caller's own account only";
const NO_INPUT: &str = "the route reads no path parameter, query or body";
const NO_BOUNDARY: &str = "no parameter, body or paging: the route reads the caller's own rows";
const FILE_MOUNT: &str = "a file mount addresses public files by name, not owned resources";
const FILE_NAME: &str = "the tail names a file: every string is a file name or a miss";
const STATIC_MISS: &str = "ServeDir answers a missing file with an empty 404: the file mounts \
     are outside the JSON API";
const SIGN_IN: &str = "a sign-in step addresses no resource";
const CREDENTIAL: &str = "the refresh token is its own credential; it addresses no other account";
const GUEST: Actor = Actor::User(Role::Guest);
const ENLISTED: Actor = Actor::User(Role::Enlisted);

fn session_token() -> Contract {
    Contract::schema("session-token.schema.json", "SessionTokenRequest")
}

fn missing_file() -> Probe {
    Probe::new("missing file", Actor::Anonymous)
        .param("*path", "route-acceptance-missing.json")
        .expect_with(Expect::status(404).without_envelope(STATIC_MISS))
}

fn file_mount(key: &'static str, content_type: &'static str) -> RouteSpec {
    RouteSpec::public(key)
        .ok(200, Contract::Binary(content_type))
        .no_ownership(FILE_MOUNT)
        .not_applicable(Dimension::Malformed, FILE_NAME)
        .boundary(missing_file())
}

fn own_read(key: &'static str, contract: Contract) -> RouteSpec {
    RouteSpec::authenticated(key)
        .ok(200, contract)
        .no_ownership(SELF_SCOPED)
        .not_applicable(Dimension::Malformed, NO_INPUT)
        .not_applicable(Dimension::Boundary, NO_BOUNDARY)
}

fn callback_refusal(name: &str, fixture: &'static str, reason: &'static str) -> Probe {
    Probe::new(name, Actor::Anonymous)
        .fixture(fixture)
        .expect_with(Expect::status(302).location_contains(reason))
}

/// Every spec of the `identity_and_core` part.
pub fn specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::public("GET /healthz")
            .ok(
                200,
                Contract::schema("service-health.schema.json", "PublicHealth"),
            )
            .unauthorized(Probe::new(
                "wrong token gets the public shape",
                Actor::WrongObservability,
            ))
            .no_ownership("an operational probe addresses no resource")
            .not_applicable(Dimension::Malformed, NO_INPUT)
            .boundary(
                Probe::new("observability bearer gets the detail", Actor::Observability)
                    .expect_with(Expect::status(200).contract(Contract::schema(
                        "service-health.schema.json",
                        "DetailedHealth",
                    ))),
            ),
        RouteSpec::observability("GET /metrics")
            .ok(200, Contract::Binary("text/plain"))
            .no_ownership("an operational scrape addresses no resource")
            .not_applicable(Dimension::Malformed, NO_INPUT)
            .not_applicable(
                Dimension::Boundary,
                "the exposition takes no parameter, body or paging",
            ),
        file_mount("GET /uploads/{*path}", "image/png"),
        file_mount("GET /map-assets/{*path}", "application/json"),
        file_mount("GET /map-assets/glyphs/{*path}", "application/json"),
        RouteSpec::development_only("GET /api/v1/auth/dev-login")
            .ok_redirect(302, "#access_token=")
            .no_ownership(SIGN_IN)
            .malformed(
                Probe::new("duplicate role", Actor::Anonymous)
                    .query("role=guest&role=admin")
                    .expect(400),
            )
            .boundary(
                Probe::new("unknown role signs in as administrator", Actor::Anonymous)
                    .query("role=quartermaster"),
            ),
        RouteSpec::public("GET /api/v1/auth/discord/login")
            .ok_redirect(307, "client_id=route-acceptance-client")
            .no_ownership(SIGN_IN)
            .not_applicable(Dimension::Malformed, NO_INPUT)
            .not_applicable(
                Dimension::Boundary,
                "the redirect is built from configuration only",
            ),
        RouteSpec::public("GET /api/v1/auth/discord/callback")
            .ok_redirect(302, "#access_token=")
            .unauthorized(callback_refusal(
                "forged state",
                "callback-forged-state",
                "error=invalid_state",
            ))
            .no_ownership(SIGN_IN)
            .guest(
                Probe::new("guild nonmember signs in", Actor::Anonymous)
                    .fixture("callback-nonmember"),
            )
            .ban(callback_refusal(
                "banned account",
                "callback-banned",
                "error=banned",
            ))
            .malformed(
                Probe::new("missing code", Actor::Anonymous)
                    .query("state=route-acceptance-oauth-state")
                    .expect_with(Expect::status(302).location_contains("error=missing_code")),
            )
            .malformed(
                Probe::new("duplicate code", Actor::Anonymous)
                    .query("code=1&code=2&state=route-acceptance-oauth-state")
                    .expect_with(Expect::status(302).location_contains("error=")),
            )
            .boundary(callback_refusal(
                "discord unreachable",
                "callback-unreachable",
                "error=discord_unreachable",
            )),
        RouteSpec::public("POST /api/v1/auth/refresh")
            .ok(
                200,
                Contract::schema("session-token.schema.json", "SessionTokenPair"),
            )
            .request_contract(session_token())
            .unauthorized(
                Probe::new("unknown refresh token", Actor::Anonymous)
                    .body(json!({"refresh_token": "route-acceptance-unknown"}))
                    .expect(401),
            )
            .no_ownership(CREDENTIAL)
            .guest(Probe::new("guest session rotates", GUEST))
            .ban(
                Probe::new("banned account's token", Actor::Anonymous)
                    .fixture("refresh-banned")
                    .expect(401),
            )
            .malformed(
                Probe::new("empty token", Actor::Anonymous)
                    .body(json!({"refresh_token": ""}))
                    .expect(400),
            )
            .boundary(
                Probe::new("replayed token", Actor::Anonymous)
                    .fixture("refresh-replayed")
                    .expect(401),
            ),
        RouteSpec::public("POST /api/v1/auth/logout")
            .ok(204, Contract::NoBody)
            .request_contract(session_token())
            .no_ownership(CREDENTIAL)
            .malformed(
                Probe::new("empty token", Actor::Anonymous)
                    .body(json!({"refresh_token": ""}))
                    .expect(400),
            )
            .boundary(
                Probe::new("unknown token is a no-op", Actor::Anonymous)
                    .body(json!({"refresh_token": "route-acceptance-unknown"}))
                    .expect(204),
            ),
        own_read(
            "GET /api/v1/me",
            Contract::schema_root("current-profile.schema.json"),
        )
        .decodes(round_trip::<CurrentProfileResponse>),
        own_read(
            "PATCH /api/v1/me",
            Contract::schema("profile-update.schema.json", "UpdatedProfile"),
        ),
        RouteSpec::authenticated("POST /api/v1/me/link")
            .ok(201, Contract::schema("arma-link.schema.json", "LinkCode"))
            .no_ownership(SELF_SCOPED)
            .not_applicable(Dimension::Malformed, NO_INPUT)
            .boundary(Probe::new("a second code supersedes the first", ENLISTED))
            .boundary(Probe::new("a third code supersedes the second", ENLISTED)),
        own_read(
            "GET /api/v1/me/link/status",
            Contract::schema("arma-link.schema.json", "LinkStatus"),
        ),
        RouteSpec::authenticated("DELETE /api/v1/me/link")
            .ok(
                200,
                Contract::schema("arma-link.schema.json", "LinkRemoval"),
            )
            .no_ownership(SELF_SCOPED)
            .not_applicable(Dimension::Malformed, NO_INPUT)
            .boundary(Probe::new("unlink a linked account", ENLISTED))
            .boundary(Probe::new("unlink again is idempotent", ENLISTED)),
        RouteSpec::machine("POST /api/v1/ingest/link-confirm", Executor::ModRuntime)
            .ok(
                200,
                Contract::schema("arma-link.schema.json", "LinkConfirmation"),
            )
            .request_contract(Contract::schema(
                "arma-link.schema.json",
                "LinkConfirmRequest",
            ))
            .no_ownership("a link code belongs to its account; any mod runtime may confirm it")
            .malformed(
                Probe::new("unknown field", Actor::Machine(Executor::ModRuntime))
                    .merge_body(json!({"server": "elsewhere"}))
                    .expect(400),
            )
            .malformed(
                Probe::new("five-digit code", Actor::Machine(Executor::ModRuntime))
                    .merge_body(json!({"code": "12345"}))
                    .expect(400),
            )
            .boundary(
                Probe::new(
                    "arma identity over 128 bytes",
                    Actor::Machine(Executor::ModRuntime),
                )
                .merge_body(json!({"arma_id": "a".repeat(129)}))
                .expect(400),
            )
            .boundary(
                Probe::new("unknown code", Actor::Machine(Executor::ModRuntime))
                    .merge_body(json!({"code": "000000"}))
                    .expect(404),
            )
            .boundary(
                Probe::new(
                    "replayed confirmation is idempotent",
                    Actor::Machine(Executor::ModRuntime),
                )
                .fixture("link-confirm-consumed"),
            )
            .boundary(
                Probe::new(
                    "consumed code for another identity",
                    Actor::Machine(Executor::ModRuntime),
                )
                .fixture("link-confirm-consumed")
                .merge_body(json!({"arma_id": "route-acceptance-other-arma"}))
                .expect(409),
            ),
        own_read(
            "GET /api/v1/me/deployments",
            Contract::schema("service-record.schema.json", "ServiceRecord"),
        ),
        own_read(
            "GET /api/v1/me/leave-requests",
            Contract::schema("leave-request.schema.json", "LeaveRequestList"),
        ),
        RouteSpec::authenticated("POST /api/v1/me/leave-requests")
            .ok(
                201,
                Contract::schema("leave-request.schema.json", "LeaveRequest"),
            )
            .request_contract(Contract::schema(
                "leave-request.schema.json",
                "LeaveRequestSubmission",
            ))
            .no_ownership(SELF_SCOPED)
            .malformed(
                Probe::new("blank reason", ENLISTED)
                    .merge_body(json!({"reason": "  "}))
                    .expect(400),
            )
            .malformed(
                Probe::new("unparseable date", ENLISTED)
                    .merge_body(json!({"starts_on": "01/10/2026"}))
                    .expect(400),
            )
            .boundary(
                Probe::new("ends before it starts", ENLISTED)
                    .merge_body(json!({"ends_on": "2026-09-30"}))
                    .expect(400),
            )
            .boundary(
                Probe::new("single-day leave", ENLISTED)
                    .merge_body(json!({"ends_on": "2026-10-01"})),
            ),
    ]
}
