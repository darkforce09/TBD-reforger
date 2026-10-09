//! Discord membership classification and rate-limit behavior against local HTTP servers.

use api_discord::discord_client::DiscordService;
use api_identifiers::{DiscordGuildId, DiscordUserId};
use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use tokio::task::JoinHandle;

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
    let base = format!(
        "http://{}",
        listener
            .local_addr()
            .expect("the listener reports its local address")
    );
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
        response.headers_mut().insert(
            "retry-after",
            retry_after
                .parse()
                .expect("the retry-after seconds form a header value"),
        );
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
        .fetch_member_with_bot(
            "bot-secret",
            &DiscordGuildId::new("partner-guild"),
            &DiscordUserId::new("player-123"),
        )
        .await;
    let member = result
        .unwrap_or_else(|error| panic!("bot lookup failed: {}", error.reason))
        .expect("verified member");
    assert_eq!(member.roles, ["role-1", "role-2"]);
    assert_eq!(member.nick, "");
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
        .fetch_member_with_bot(
            "token",
            &DiscordGuildId::new("partner-guild"),
            &DiscordUserId::new("player-123"),
        )
        .await
        .unwrap_or_else(|error| panic!("bot lookup failed: {}", error.reason));
    assert!(bot.is_none());
    assert!(client.fetch_guild_member("token").await.unwrap().is_none());
}
