//! Shared world for the session-authority properties.
//!
//! **Role:** Opens the binary's database once and serves it through a development-configured
//! and a production-configured application state that share one signing secret, seeds fresh
//! accounts, and sends bearer-authenticated requests through the real router.
//!
//! **Position:** Test support compiled into `tests/session_authority_properties.rs`.
//! [`authority_cases`] and [`authority_oracle`] serve the protected-action property;
//! [`refresh_families`] and [`persisted_families`] serve the refresh-replay property.
//!
//! **Signals & state:** a process-global peer counter; each [`PropertyWorld`] owns one pool
//! and the two routers built over it.
//!
//! **Invariants:** every request carries a peer address no earlier request in the process
//! used, so the per-peer rate limiter never answers in place of the handler, and a 429 fails
//! the case by name; every seeded account id is unique in the binary database.

pub mod authority_cases;
pub mod authority_oracle;
pub mod persisted_families;
pub mod refresh_families;

use authority_cases::ServingEnvironment;
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::{Request, StatusCode, header},
};
use serde_json::Value;
use sqlx::PgPool;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU32, Ordering};
use tower::ServiceExt;
use uuid::Uuid;
use website_api::core::{
    application_state::AppState, configuration::Config, database, http_router,
};

/// The signing secret both configurations share, so either router accepts either's credentials.
pub const JWT_SECRET: &str = "session-authority-properties";

/// The next synthetic peer; starts at 1 so no request presents the unspecified address.
static NEXT_PEER: AtomicU32 = AtomicU32::new(1);

/// One database served by both deployment configurations.
pub struct PropertyWorld {
    /// The development-configured state: it issues every session, including development ones.
    pub development: AppState,
    development_router: Router,
    production_router: Router,
}

impl PropertyWorld {
    /// Provision and migrate the binary database, then build both routers over one pool.
    pub async fn open() -> Self {
        let url = crate::common::require_test_database_url()
            .expect("the session authority properties need TEST_DATABASE_URL");
        let pool = database::connect(&url)
            .await
            .expect("connect the test database");
        database::migrate(&pool)
            .await
            .expect("migrate the test database");
        let development = AppState::new(pool.clone(), Config::for_tests(url.clone(), JWT_SECRET));
        let mut production_config = Config::for_tests(url, JWT_SECRET);
        production_config.env = "production".into();
        let production = AppState::new(pool, production_config);
        Self {
            development_router: http_router::router(development.clone()),
            production_router: http_router::router(production),
            development,
        }
    }

    /// The router serving requests under `environment`'s configuration.
    pub fn router(&self, environment: ServingEnvironment) -> &Router {
        match environment {
            ServingEnvironment::Development => &self.development_router,
            ServingEnvironment::Production => &self.production_router,
        }
    }
}

/// Seed a fresh account whose stored `users.role` column is `admin`.
///
/// Site authority comes from guild membership rows alone, so a rank read from that column
/// instead shows up as an over-grant in the protected-action property.
pub async fn seed_account(pool: &PgPool, prefix: &str) -> String {
    let discord_id = format!("{prefix}-{}", Uuid::new_v4());
    crate::common::seed_user(
        pool,
        &discord_id,
        "Session Authority Property",
        &crate::common::unique_arma(prefix),
        "admin",
    )
    .await;
    discord_id
}

fn next_peer() -> SocketAddr {
    let [_, b, c, d] = NEXT_PEER.fetch_add(1, Ordering::Relaxed).to_be_bytes();
    SocketAddr::from((IpAddr::from([10, b, c, d]), 41_000))
}

/// Send `GET uri` bearing `access_credential` from a fresh peer; the status and the JSON body
/// (`Null` when the body is not JSON).
pub async fn send_get(router: &Router, uri: &str, access_credential: &str) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {access_credential}"))
        .body(Body::empty())
        .expect("build the probe request");
    request.extensions_mut().insert(ConnectInfo(next_peer()));
    let response = router
        .clone()
        .oneshot(request)
        .await
        .expect("the router answers every request");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read the probe response body");
    assert_ne!(
        status,
        StatusCode::TOO_MANY_REQUESTS,
        "GET {uri}: the rate limiter answered before the handler; the peer key is not varying"
    );
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}
