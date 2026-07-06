//! Webhook + Discord service HTTP behavior against a local axum mock server —
//! ports the Go `webhook_test.go` + `discord_test.go` httptest suites (success,
//! disabled, server-error, bounded 429 retry, embed caps, OAuth exchange, fetch).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use axum::routing::{get, post};
use chrono::Utc;
use reforger_backend::models::{Announcement, AnnouncementStatus, AnnouncementTag};
use reforger_backend::services::{DiscordService, WebhookService};
use serde_json::{Value, json};
use uuid::Uuid;

async fn spawn(router: Router) -> String {
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(l, router).await.unwrap() });
    format!("http://{addr}")
}

fn ann(title: &str, body: &str, snippet: &str) -> Announcement {
    Announcement {
        id: Uuid::new_v4(),
        title: title.into(),
        body: body.into(),
        snippet: snippet.into(),
        tag: AnnouncementTag::Update,
        thumbnail_url: String::new(),
        author_id: "u".into(),
        status: AnnouncementStatus::Published,
        is_pinned: false,
        pushed_to_discord: false,
        discord_message_id: String::new(),
        published_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

// --- webhook ---

#[tokio::test]
async fn webhook_push_success_returns_message_id() {
    let base =
        spawn(Router::new().route("/wh", post(|| async { Json(json!({ "id": "msg-42" })) }))).await;
    let wh = WebhookService::new(format!("{base}/wh"));
    let id = wh
        .push_announcement(&ann("Op Redwood", "body", "snip"))
        .await
        .unwrap();
    assert_eq!(id, "msg-42");
}

#[tokio::test]
async fn webhook_disabled_errors() {
    let wh = WebhookService::new(String::new());
    assert!(!wh.enabled());
    assert!(wh.push_announcement(&ann("t", "b", "s")).await.is_err());
}

#[tokio::test]
async fn webhook_server_error_errors() {
    let base =
        spawn(Router::new().route("/wh", post(|| async { StatusCode::INTERNAL_SERVER_ERROR })))
            .await;
    let wh = WebhookService::new(format!("{base}/wh"));
    assert!(wh.push_announcement(&ann("t", "b", "s")).await.is_err());
}

#[tokio::test]
async fn webhook_retries_on_429_then_succeeds() {
    async fn h(State(hits): State<Arc<AtomicUsize>>) -> axum::response::Response {
        if hits.fetch_add(1, Ordering::SeqCst) == 0 {
            (
                StatusCode::TOO_MANY_REQUESTS,
                [("retry-after", "0")],
                "slow down",
            )
                .into_response()
        } else {
            Json(json!({ "id": "msg-after-retry" })).into_response()
        }
    }
    let hits = Arc::new(AtomicUsize::new(0));
    let base = spawn(Router::new().route("/wh", post(h)).with_state(hits.clone())).await;
    let wh = WebhookService::new(format!("{base}/wh"));
    let id = wh.push_announcement(&ann("t", "b", "s")).await.unwrap();
    assert_eq!(id, "msg-after-retry");
    assert_eq!(hits.load(Ordering::SeqCst), 2, "one 429 + one success");
}

#[tokio::test]
async fn webhook_caps_embed_title_to_256_runes() {
    async fn h(
        State(seen): State<Arc<std::sync::Mutex<Option<Value>>>>,
        Json(v): Json<Value>,
    ) -> Json<Value> {
        *seen.lock().unwrap() = Some(v);
        Json(json!({ "id": "ok" }))
    }
    let seen = Arc::new(std::sync::Mutex::new(None));
    let base = spawn(Router::new().route("/wh", post(h)).with_state(seen.clone())).await;
    let wh = WebhookService::new(format!("{base}/wh"));
    let long_title = "A".repeat(300);
    wh.push_announcement(&ann(&long_title, "b", "s"))
        .await
        .unwrap();
    let body = seen.lock().unwrap().clone().unwrap();
    let title = body["embeds"][0]["title"].as_str().unwrap();
    assert_eq!(title.chars().count(), 256, "title capped to 256 runes");
}

// --- discord ---

fn discord(base: &str) -> DiscordService {
    let mut d = DiscordService::new(
        "client-1".into(),
        "secret-1".into(),
        "http://localhost/cb".into(),
        "guild-1".into(),
    );
    d.set_api_base(base);
    d
}

#[tokio::test]
async fn discord_exchange_code_returns_token() {
    let base = spawn(Router::new().route(
        "/oauth2/token",
        post(|| async { Json(json!({ "access_token": "tok-1", "token_type": "Bearer" })) }),
    ))
    .await;
    let tok = discord(&base).exchange_code("the-code").await.unwrap();
    assert_eq!(tok.access_token, "tok-1");
}

#[tokio::test]
async fn discord_exchange_bad_code_errors() {
    let base = spawn(Router::new().route(
        "/oauth2/token",
        post(|| async { (StatusCode::BAD_REQUEST, "invalid_grant") }),
    ))
    .await;
    assert!(discord(&base).exchange_code("bad").await.is_err());
}

#[tokio::test]
async fn discord_fetch_user_derived_fields() {
    let base = spawn(Router::new().route(
        "/users/@me",
        get(|| async {
            Json(json!({ "id": "u1", "username": "Bob", "global_name": "Bobby", "discriminator": "0", "avatar": "abc" }))
        }),
    ))
    .await;
    let u = discord(&base).fetch_user("tok").await.unwrap();
    assert_eq!(u.id, "u1");
    assert_eq!(u.username, "Bob");
    assert_eq!(u.avatar, "abc");
}

#[tokio::test]
async fn discord_fetch_guild_member_roles_and_404() {
    // Member present → roles returned.
    let base = spawn(Router::new().route(
        "/users/@me/guilds/guild-1/member",
        get(|| async { Json(json!({ "nick": "B", "roles": ["r1", "r2"] })) }),
    ))
    .await;
    let m = discord(&base).fetch_guild_member("tok").await.unwrap();
    assert_eq!(m.unwrap().roles, ["r1", "r2"]);

    // 404 → None (non-member login still succeeds).
    let base404 = spawn(Router::new().route(
        "/users/@me/guilds/guild-1/member",
        get(|| async { StatusCode::NOT_FOUND }),
    ))
    .await;
    assert!(
        discord(&base404)
            .fetch_guild_member("tok")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn discord_retries_on_429_then_succeeds() {
    async fn h(State(hits): State<Arc<AtomicUsize>>) -> axum::response::Response {
        if hits.fetch_add(1, Ordering::SeqCst) == 0 {
            (
                StatusCode::TOO_MANY_REQUESTS,
                [("retry-after", "0")],
                "slow",
            )
                .into_response()
        } else {
            Json(json!({ "access_token": "tok-retry", "token_type": "Bearer" })).into_response()
        }
    }
    let hits = Arc::new(AtomicUsize::new(0));
    let base = spawn(
        Router::new()
            .route("/oauth2/token", post(h))
            .with_state(hits.clone()),
    )
    .await;
    let tok = discord(&base).exchange_code("c").await.unwrap();
    assert_eq!(tok.access_token, "tok-retry");
    assert_eq!(hits.load(Ordering::SeqCst), 2);
}
