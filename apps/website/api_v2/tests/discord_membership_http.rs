//! Discord membership classification and rate-limit behavior against local HTTP servers.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use tokio::task::JoinHandle;
use website_api::identity_and_access::services::discord_client::DiscordService;

const BOT_ROUTE: &str = "/guilds/partner-guild/members/player-123";
const OAUTH_ROUTE: &str = "/users/@me/guilds/main-guild/member";

struct MockServer {
    base: String,
    task: JoinHandle<()>,
}

impl Drop for MockServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn serve(router: Router) -> MockServer {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock Discord server");
    let base = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("mock HTTP server");
    });
    MockServer { base, task }
}

fn client(server: &MockServer) -> DiscordService {
    let mut client = DiscordService::new(
        "client-id".into(),
        "client-secret".into(),
        "http://localhost/callback".into(),
        "main-guild".into(),
    );
    client.set_api_base(&server.base);
    client
}

#[derive(Clone)]
struct Reply {
    status: StatusCode,
    body: &'static str,
    retry_after: Option<&'static str>,
}

async fn respond(State(reply): State<Reply>) -> Response {
    let mut response = (reply.status, reply.body).into_response();
    if let Some(retry_after) = reply.retry_after {
        response
            .headers_mut()
            .insert("retry-after", retry_after.parse().unwrap());
    }
    response
}

async fn fixed_response(
    status: StatusCode,
    body: &'static str,
    retry_after: Option<&'static str>,
) -> MockServer {
    serve(
        Router::new()
            .route(BOT_ROUTE, get(respond))
            .route(OAUTH_ROUTE, get(respond))
            .with_state(Reply {
                status,
                body,
                retry_after,
            }),
    )
    .await
}

#[tokio::test]
async fn discord_membership_bot_uses_scoped_endpoint_and_bot_authorization() {
    async fn handler(headers: HeaderMap) -> Response {
        if headers
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            != Some("Bot bot-secret")
        {
            return StatusCode::UNAUTHORIZED.into_response();
        }
        (
            StatusCode::OK,
            r#"{"roles":["role-1","role-2"],"nick":null}"#,
        )
            .into_response()
    }
    let server = serve(Router::new().route(BOT_ROUTE, get(handler))).await;
    let result = client(&server)
        .fetch_member_with_bot("bot-secret", "partner-guild", "player-123")
        .await;
    let member = result
        .unwrap_or_else(|error| panic!("bot lookup failed: {}", error.reason))
        .expect("verified member");
    assert_eq!(member.roles, ["role-1", "role-2"]);
    assert_eq!(member.nick, "");
}

#[tokio::test]
async fn discord_membership_oauth_uses_bearer_authorization_and_main_guild() {
    async fn handler(headers: HeaderMap) -> Response {
        if headers
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            != Some("Bearer oauth-secret")
        {
            return StatusCode::UNAUTHORIZED.into_response();
        }
        (StatusCode::OK, r#"{"roles":["main-role"]}"#).into_response()
    }
    let server = serve(Router::new().route(OAUTH_ROUTE, get(handler))).await;
    let member = client(&server)
        .fetch_guild_member("oauth-secret")
        .await
        .expect("OAuth membership lookup")
        .expect("verified member");
    assert_eq!(member.roles, ["main-role"]);
}

#[tokio::test]
async fn discord_membership_empty_roles_mean_member_for_both_clients() {
    let server = fixed_response(StatusCode::OK, r#"{"roles":[]}"#, None).await;
    let client = client(&server);
    let bot = client
        .fetch_member_with_bot("token", "partner-guild", "player-123")
        .await
        .unwrap_or_else(|error| panic!("bot lookup failed: {}", error.reason));
    assert!(
        bot.expect("empty role list still establishes membership")
            .roles
            .is_empty()
    );
    let oauth = client.fetch_guild_member("token").await.unwrap();
    assert!(
        oauth
            .expect("empty role list still establishes membership")
            .roles
            .is_empty()
    );
}

#[tokio::test]
async fn discord_membership_malformed_success_never_becomes_nonmembership() {
    for body in [
        "not json",
        "null",
        r#"{"nick":"member"}"#,
        r#"{"roles":null}"#,
        r#"{"roles":"role-1"}"#,
        r#"{"roles":[7]}"#,
    ] {
        let server = fixed_response(StatusCode::OK, body, None).await;
        let client = client(&server);
        let failure = client
            .fetch_member_with_bot("token", "partner-guild", "player-123")
            .await
            .expect_err("malformed roles cannot establish membership");
        assert!(!failure.rate_limited);
        assert_eq!(failure.retry_after.num_seconds(), 60);
        assert!(
            client.fetch_guild_member("token").await.is_err(),
            "body: {body}"
        );
    }
}

#[tokio::test]
async fn discord_membership_only_unknown_member_code_confirms_departure() {
    let server = fixed_response(
        StatusCode::NOT_FOUND,
        r#"{"code":10007,"message":"Unknown Member"}"#,
        None,
    )
    .await;
    let client = client(&server);
    let bot = client
        .fetch_member_with_bot("token", "partner-guild", "player-123")
        .await
        .unwrap_or_else(|error| panic!("bot lookup failed: {}", error.reason));
    assert!(bot.is_none());
    assert!(client.fetch_guild_member("token").await.unwrap().is_none());
}

#[tokio::test]
async fn discord_membership_other_404_responses_preserve_uncertainty() {
    for body in [
        r#"{"code":10004,"message":"Unknown Guild"}"#,
        r#"{"code":"10007"}"#,
        r#"{"message":"not found"}"#,
        "not found",
        "",
    ] {
        let server = fixed_response(StatusCode::NOT_FOUND, body, None).await;
        let client = client(&server);
        let failure = client
            .fetch_member_with_bot("token", "partner-guild", "player-123")
            .await
            .expect_err("unclassified 404 is unavailable verification");
        assert!(!failure.rate_limited);
        assert!(
            client.fetch_guild_member("token").await.is_err(),
            "body: {body}"
        );
    }
}

#[tokio::test]
async fn discord_membership_bot_rate_limit_preserves_body_and_header_retry_delays() {
    for (body, header, expected_millis) in [
        (r#"{"retry_after":1.25}"#, Some("9"), 1_250),
        ("rate limited", Some("2.75"), 2_750),
        ("", Some("3.1"), 3_100),
        (r#"{"retry_after":"invalid"}"#, Some("4"), 4_000),
        (r#"{"retry_after":-1}"#, Some("4"), 4_000),
        (r#"{"retry_after":604801}"#, Some("4"), 4_000),
        (r#"{"retry_after":0}"#, None, 1_000),
        ("", Some("NaN"), 60_000),
        ("", None, 60_000),
    ] {
        let server = fixed_response(StatusCode::TOO_MANY_REQUESTS, body, header).await;
        let failure = client(&server)
            .fetch_member_with_bot("token", "partner-guild", "player-123")
            .await
            .expect_err("rate-limited lookup must fail explicitly");
        assert!(failure.rate_limited, "body: {body}; header: {header:?}");
        assert_eq!(
            failure.retry_after.num_milliseconds(),
            expected_millis,
            "body: {body}; header: {header:?}"
        );
    }
}

#[tokio::test]
async fn discord_membership_server_errors_never_confirm_departure() {
    for body in [r#"{"roles":[]}"#, r#"{"code":10007}"#, "unavailable"] {
        let server = fixed_response(StatusCode::INTERNAL_SERVER_ERROR, body, None).await;
        let client = client(&server);
        let failure = client
            .fetch_member_with_bot("token", "partner-guild", "player-123")
            .await
            .expect_err("server failure cannot establish membership");
        assert!(!failure.rate_limited);
        assert_eq!(failure.retry_after.num_seconds(), 60);
        assert!(client.fetch_guild_member("token").await.is_err());
    }
}

#[tokio::test]
async fn discord_membership_oauth_rate_limit_retries_are_bounded() {
    for body in ["rate limited", r#"{"retry_after":0}"#] {
        let hits = Arc::new(AtomicUsize::new(0));
        let observed = hits.clone();
        let server = serve(Router::new().route(
            OAUTH_ROUTE,
            get(move || {
                let observed = observed.clone();
                async move {
                    observed.fetch_add(1, Ordering::SeqCst);
                    (StatusCode::TOO_MANY_REQUESTS, [("retry-after", "0")], body)
                }
            }),
        ))
        .await;
        assert!(client(&server).fetch_guild_member("token").await.is_err());
        assert_eq!(hits.load(Ordering::SeqCst), 3, "OAuth retries are bounded");
    }
}
