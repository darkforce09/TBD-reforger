//! The identity and core part's world: a stand-in Discord, an uploaded file, and fresh
//! sessions, link codes and OAuth callbacks per probe.
//!
//! **Role:** implements [`PartWorld`] for the identity and access routes, the `/me*` routes and
//! the router's own routes (`/healthz`, `/metrics`, `/uploads`, `/map-assets`); it is the
//! reference world for the other parts.
//!
//! **Position:** a part world declared by [`super`]; its
//! specs are `specs/identity_and_core.rs`.
//!
//! **Signals & state:** a local HTTP server answering the three Discord calls the OAuth
//! callback makes (aborted when the world drops), and a counter for fresh Discord ids.
//!
//! **Invariants:** the stand-in Discord derives everything from the authorization code: the
//! code is the access token, the Discord id is the code (a `nonmember-` prefix answers "unknown
//! member"), and the code `unreachable` fails the token exchange; every refresh token,
//! link code and callback is minted per probe, so no probe consumes another's credential.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use api_configuration::configuration::Config;
use api_identity_and_access::services::session_issuance::issue_session;
use api_state::AppState;
use axum::extract::Form;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};

use crate::route_acceptance_support::actors::Actors;
use crate::route_acceptance_support::requests::{Outgoing, send};
use crate::route_acceptance_support::spec::{Actor, Role};
use crate::route_acceptance_support::world::{Fixture, PartWorld, WorldCore};

/// The file the world places in the upload directory.
pub(crate) const UPLOADED_FILE: &str = "route-acceptance-probe.png";
/// The OAuth client id the world configures; the login redirect names it.
pub(crate) const CLIENT_ID: &str = "route-acceptance-client";
/// The CSRF state the callback fixtures present in both the cookie and the query.
const OAUTH_STATE: &str = "route-acceptance-oauth-state";
/// The eight-byte PNG signature: enough for `ServeDir` to serve a non-empty `image/png`.
const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// The identity and core world.
pub(crate) struct IdentityAndCoreWorld {
    discord: tokio::task::JoinHandle<()>,
    discord_ids: Arc<AtomicU32>,
}

impl Drop for IdentityAndCoreWorld {
    fn drop(&mut self) {
        self.discord.abort();
    }
}

fn bearer_of(headers: &HeaderMap) -> String {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or_default()
        .to_string()
}

/// The stand-in for the Discord API the OAuth callback calls.
fn stand_in_discord() -> Router {
    Router::new()
        .route(
            "/oauth2/token",
            post(|Form(form): Form<HashMap<String, String>>| async move {
                let code = form.get("code").cloned().unwrap_or_default();
                if code == "unreachable" {
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({}))).into_response();
                }
                Json(
                    json!({"access_token": code, "token_type": "Bearer", "expires_in": 600,
                    "refresh_token": "stand-in", "scope": "identify guilds.members.read"}),
                )
                .into_response()
            }),
        )
        .route(
            "/users/@me",
            get(|headers: HeaderMap| async move {
                let token = bearer_of(&headers);
                let id = token
                    .strip_prefix("nonmember-")
                    .unwrap_or(&token)
                    .to_string();
                Json(
                    json!({"id": id, "username": "route-acceptance", "global_name": null,
                    "discriminator": "0", "avatar": null}),
                )
            }),
        )
        .route(
            "/users/@me/guilds/{guild}/member",
            get(|headers: HeaderMap| async move {
                if bearer_of(&headers).starts_with("nonmember-") {
                    let unknown = json!({"code": 10007, "message": "Unknown Member"});
                    return (StatusCode::NOT_FOUND, Json(unknown)).into_response();
                }
                Json(json!({"nick": null, "roles": []})).into_response()
            }),
        )
}

impl IdentityAndCoreWorld {
    fn fresh_discord_id(&self, core: &WorldCore) -> String {
        let n = self.discord_ids.fetch_add(1, Ordering::Relaxed);
        format!("96{:02}{n:014}", core.actors.namespace)
    }

    fn callback(code: &str, state: &str) -> Fixture {
        Fixture::new()
            .query(format!("code={code}&state={state}"))
            .header("cookie", format!("oauth_state={OAUTH_STATE}"))
    }

    /// The account `actor` names, or a fresh enlisted account for a caller without one.
    async fn session_owner(core: &WorldCore, actor: Actor) -> String {
        match core.actors.account(actor) {
            Some(account) => account.discord_id.clone(),
            None => {
                core.actors
                    .fresh_user(&core.state, Role::Enlisted, true)
                    .await
                    .discord_id
            }
        }
    }

    async fn refresh_token(core: &WorldCore, discord_id: &str) -> String {
        issue_session(
            &core.state,
            &api_identifiers::DiscordUserId::new(discord_id),
        )
        .await
        .unwrap_or_else(|error| panic!("issue a session for {discord_id}: {error:?}"))
        .2
    }

    /// A link confirmation body for a fresh unlinked account's newly issued code.
    async fn link_confirmation(core: &WorldCore) -> Value {
        let account = core
            .actors
            .fresh_user(&core.state, Role::Enlisted, false)
            .await;
        let issued = send(
            &core.app,
            &Outgoing::new("POST", "/api/v1/me/link").bearer(&account.token),
        )
        .await;
        let code = issued
            .json()
            .and_then(|body| body["code"].as_str().map(str::to_string));
        let code = code.unwrap_or_else(|| {
            panic!(
                "{}: issue a link code: {} {}",
                core.suite,
                issued.status,
                issued.excerpt()
            )
        });
        json!({"code": code, "arma_id": format!("route-acceptance-arma-{}", account.discord_id),
            "arma_character": "Route Acceptance"})
    }
}

impl PartWorld for IdentityAndCoreWorld {
    fn configure(config: &mut Config) {
        config.discord_client_id = CLIENT_ID.into();
        config.discord_client_secret = "route-acceptance-client-secret".into();
        config.discord_redirect_url = "http://localhost:8080/api/v1/auth/discord/callback".into();
    }

    async fn build(state: &mut AppState, _actors: &Actors) -> Self {
        std::fs::create_dir_all(&state.cfg.upload_dir).expect("create the upload directory");
        let uploaded = std::path::Path::new(&state.cfg.upload_dir).join(UPLOADED_FILE);
        std::fs::write(uploaded, PNG_SIGNATURE).expect("place the uploaded file");
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind the stand-in Discord");
        let base = format!(
            "http://{}",
            listener.local_addr().expect("stand-in address")
        );
        let discord = tokio::spawn(async move {
            axum::serve(listener, stand_in_discord())
                .await
                .expect("serve the stand-in Discord");
        });
        Arc::make_mut(&mut state.discord).set_api_base(&base);
        IdentityAndCoreWorld {
            discord,
            discord_ids: Arc::new(AtomicU32::new(1)),
        }
    }

    async fn fixture(&self, core: &WorldCore, key: &str, actor: Actor) -> Option<Fixture> {
        let fixture = match key {
            "GET /uploads/{*path}" => Fixture::new().param("*path", UPLOADED_FILE),
            "GET /map-assets/{*path}" => Fixture::new().param("*path", "terrain-registry.json"),
            "GET /map-assets/glyphs/{*path}" => Fixture::new().param("*path", "manifest.json"),
            "GET /api/v1/auth/dev-login" => Fixture::new().query("role=guest"),
            "GET /api/v1/auth/discord/callback" => {
                Self::callback(&self.fresh_discord_id(core), OAUTH_STATE)
            }
            "callback-forged-state" => Self::callback(&self.fresh_discord_id(core), "forged-state"),
            "POST /api/v1/auth/refresh" | "POST /api/v1/auth/logout" => {
                let owner = Self::session_owner(core, actor).await;
                let token = Self::refresh_token(core, &owner).await;
                Fixture::new().body(json!({"refresh_token": token}))
            }
            "POST /api/v1/ingest/link-confirm" => {
                Fixture::new().body(Self::link_confirmation(core).await)
            }
            "POST /api/v1/me/leave-requests" => Fixture::new().body(json!({
                "starts_on": "2026-10-01", "ends_on": "2026-10-03", "reason": "Route acceptance"
            })),
            _ => return None,
        };
        Some(fixture)
    }
}
