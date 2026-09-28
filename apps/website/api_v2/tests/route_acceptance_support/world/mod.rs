//! The world a route acceptance binary probes: the shared core and the part builder trait.
//!
//! **Role:** builds the shared core — the migrated per-binary database, the application state,
//! every [`Actors`] caller (seeded accounts with verified guild membership, a banned account,
//! machine credentials) and the router — and defines [`PartWorld`], which each part implements
//! to supply the rows and requests its routes need.
//!
//! **Position:** test support; the dimension runner builds one [`World`] per test function.
//! A part's world lives in `world/<part>.rs` and is mounted by that part's binary with
//! `#[path = "route_acceptance_support/world/<part>.rs"] mod <part>_world;`, so it may use the
//! support modules only that binary declares; this module declares no part world.
//!
//! **Signals & state:** the world owns its rows in the binary's database; nothing is shared
//! between worlds except that database and the process-wide route table.
//!
//! **Invariants:** the database comes from `common::require_test_database_url`, which panics
//! when `TEST_DATABASE_URL` is unset, so no world builds without its database; a part adjusts
//! the configuration in [`PartWorld::configure`] and the state in [`PartWorld::build`] before
//! the router is assembled from it; each world's namespace keeps its accounts apart from every
//! other world's.

use axum::Router;
use serde_json::Value;
use website_api::core::application_state::AppState;
use website_api::core::configuration::Config;
use website_api::core::{database, http_router};

use super::actors::Actors;
use super::spec::Actor;
use crate::common;

/// The JWT secret every route acceptance world signs with.
const JWT_SECRET: &str = "route-acceptance-secret";

/// The request a probe starts from: path parameters, query, JSON body and extra headers.
#[derive(Debug, Clone, Default)]
pub struct Fixture {
    pub params: Vec<(String, String)>,
    /// The query string, without `?`.
    pub query: Option<String>,
    pub body: Option<Value>,
    /// A non-JSON body (e.g. a multipart form) and its `Content-Type`; sent instead of `body`.
    pub raw_body: Option<(Vec<u8>, String)>,
    pub headers: Vec<(String, String)>,
}

impl Fixture {
    /// An empty request.
    pub fn new() -> Fixture {
        Fixture::default()
    }

    /// Set path parameter `name` (as the route spells it, e.g. `id` or `*path`).
    pub fn param(mut self, name: &str, value: impl Into<String>) -> Fixture {
        self.params.push((name.to_string(), value.into()));
        self
    }

    /// Set the query string.
    pub fn query(mut self, query: impl Into<String>) -> Fixture {
        self.query = Some(query.into());
        self
    }

    /// Set the JSON body.
    pub fn body(mut self, body: Value) -> Fixture {
        self.body = Some(body);
        self
    }

    /// Set a non-JSON body with its `Content-Type`.
    pub fn raw_body(mut self, bytes: Vec<u8>, content_type: impl Into<String>) -> Fixture {
        self.raw_body = Some((bytes, content_type.into()));
        self
    }

    /// Add a request header.
    pub fn header(mut self, name: &str, value: impl Into<String>) -> Fixture {
        self.headers.push((name.to_string(), value.into()));
        self
    }
}

/// The shared core of every world.
pub struct WorldCore {
    /// The router under test, assembled from [`WorldCore::state`].
    pub app: Router,
    pub state: AppState,
    pub actors: Actors,
    /// The binary's name, for fixture failure messages.
    pub suite: &'static str,
}

/// What a part supplies: its rows and the requests its routes are probed with.
pub trait PartWorld: Sized {
    /// Adjust the configuration before the state is built (e.g. a smaller body limit).
    fn configure(_config: &mut Config) {}

    /// Seed the part's rows once per world. `state` may still be adjusted here (e.g. an HTTP
    /// base for an external client); a router built from it here serves setup requests only.
    async fn build(state: &mut AppState, actors: &Actors) -> Self;

    /// A fresh request for fixture `key` (a spec key such as `GET /api/v1/me`, or a name a
    /// probe chose), sent as `actor`; `None` for a route that needs no parameters or body.
    async fn fixture(&self, core: &WorldCore, key: &str, actor: Actor) -> Option<Fixture>;
}

/// A built world: the shared core and the part's own rows.
pub struct World<W> {
    pub core: WorldCore,
    pub part: W,
}

impl<W: PartWorld> World<W> {
    /// Build the core and the part world in namespace `namespace` (0–99, one per test function
    /// of a binary).
    pub async fn build(suite: &'static str, namespace: u8) -> World<W> {
        let url = common::require_test_database_url()
            .expect("TEST_DATABASE_URL is required for the route acceptance binaries");
        let pool = database::connect(&url).await.expect("connect");
        database::migrate(&pool).await.expect("migrate");
        let mut config = Config::for_tests(url, JWT_SECRET);
        W::configure(&mut config);
        let mut state = AppState::new(pool, config);
        let actors = Actors::create(&state, suite, namespace).await;
        let part = W::build(&mut state, &actors).await;
        let app = http_router::router(state.clone());
        World {
            core: WorldCore {
                app,
                state,
                actors,
                suite,
            },
            part,
        }
    }
}
