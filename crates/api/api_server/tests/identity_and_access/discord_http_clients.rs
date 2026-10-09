//! Webhook + Discord service HTTP behavior against a local axum mock server: success,
//! disabled, server-error, bounded 429 retry, embed caps, OAuth exchange, fetch.

use api_community_content::handlers::announcement_discord_push::webhook_announcement;
use api_community_content::models::announcement::{
    Announcement, AnnouncementStatus, AnnouncementTag,
};
use api_discord::discord_client::DiscordService;
use api_discord::discord_webhook::WebhookService;
use api_identifiers::DiscordMessageId;
use axum::Router;
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::{get, post};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

async fn spawn(router: Router) -> String {
    let l = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a loopback port binds");
    let addr = l
        .local_addr()
        .expect("the listener reports its local address");
    tokio::spawn(async move {
        axum::serve(l, router)
            .await
            .expect("the stand-in HTTP server keeps serving")
    });
    format!("http://{addr}")
}

fn ann(title: &str, body: &str, snippet: &str) -> Announcement {
    Announcement {
        id: Uuid::new_v4().into(),
        title: title.into(),
        body: body.into(),
        snippet: snippet.into(),
        tag: AnnouncementTag::Update,
        thumbnail_url: String::new(),
        author_id: "u".into(),
        status: AnnouncementStatus::Published,
        is_pinned: false,
        pushed_to_discord: false,
        discord_message_id: DiscordMessageId::default(),
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
        .push_announcement(&webhook_announcement(&ann("Op Redwood", "body", "snip")))
        .await
        .unwrap();
    assert_eq!(id, "msg-42");
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
        get(|| async {
            (
                StatusCode::NOT_FOUND,
                axum::Json(serde_json::json!({"code": 10007})),
            )
        }),
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
