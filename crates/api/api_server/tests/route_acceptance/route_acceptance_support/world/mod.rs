//! The world a route acceptance binary probes: the shared core and the part builder trait.
//!
//! **Role:** builds the shared core — the migrated per-binary database, the application state,
//! every [`Actors`] caller (seeded accounts with verified guild membership, a banned account,
//! machine credentials) and the router — and defines [`PartWorld`], which each part implements
//! to supply the rows and requests its routes need.
//!
//! **Position:** test support; the dimension runner builds one [`World`] per test function.
//! A part's world lives in `world/<part>.rs`.
//!
//! **Signals & state:** the world owns its rows in the binary's database; nothing is shared
//! between worlds except that database.
//!
//! **Invariants:** the database comes from `common::require_test_database_url`, which panics
//! when `TEST_DATABASE_URL` is unset, so no world builds without its database; a part adjusts
//! the configuration in [`PartWorld::configure`] and the state in [`PartWorld::build`] before
//! the router is assembled from it; each world's namespace keeps its accounts apart from every
//! other world's.

use api_configuration::configuration::Config;
use api_server::router::router;
use api_state::AppState;
use axum::Router;
use serde_json::Value;

pub(crate) mod administration_center_content;
pub(crate) mod fleet_and_telemetry;
pub(crate) mod identity_and_core;
pub(crate) mod missions_library;
pub(crate) mod missions_reviews;
pub(crate) mod operations_ballistics;
pub(crate) mod operations_events;
pub(crate) mod operations_reservations;

use super::actors::Actors;
use super::spec::Actor;
use crate::common;

/// The JWT secret every route acceptance world signs with.
const JWT_SECRET: &str = "route-acceptance-secret";

/// The request a probe starts from: path parameters, query, JSON body and extra headers.
#[derive(Debug, Clone, Default)]
pub(crate) struct Fixture {
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
    pub(crate) fn new() -> Fixture {
        Fixture::default()
    }

    /// Set path parameter `name` (as the route spells it, e.g. `id` or `*path`).
    pub(crate) fn param(mut self, name: &str, value: impl Into<String>) -> Fixture {
        self.params.push((name.to_string(), value.into()));
        self
    }

    /// Set the query string.
    pub(crate) fn query(mut self, query: impl Into<String>) -> Fixture {
        self.query = Some(query.into());
        self
    }

    /// Set the JSON body.
    pub(crate) fn body(mut self, body: Value) -> Fixture {
        self.body = Some(body);
        self
    }

    /// Set a non-JSON body with its `Content-Type`.
    pub(crate) fn raw_body(mut self, bytes: Vec<u8>, content_type: impl Into<String>) -> Fixture {
        self.raw_body = Some((bytes, content_type.into()));
        self
    }

    /// Add a request header.
    pub(crate) fn header(mut self, name: &str, value: impl Into<String>) -> Fixture {
        self.headers.push((name.to_string(), value.into()));
        self
    }
}

/// The shared core of every world.
pub(crate) struct WorldCore {
    /// The router under test, assembled from [`WorldCore::state`].
    pub app: Router,
    pub state: AppState,
    pub actors: Actors,
    /// The binary's name, for fixture failure messages.
    pub suite: &'static str,
}

/// What a part supplies: its rows and the requests its routes are probed with.
pub(crate) trait PartWorld: Sized {
    /// The scope of an isolated database for a part whose outcomes depend on rows another
    /// part writes (e.g. the current modpack an artifact compiles against); `None` shares the
    /// binary's database.
    const ISOLATED_DATABASE: Option<&'static str> = None;

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
pub(crate) struct World<W> {
    pub core: WorldCore,
    pub part: W,
}

impl<W: PartWorld> World<W> {
    /// Build the core and the part world in namespace `namespace` (0–99, one per test function
    /// of the binary).
    pub(crate) async fn build(suite: &'static str, namespace: u8) -> World<W> {
        let url = match W::ISOLATED_DATABASE {
            Some(scope) => common::require_isolated_test_database_url(scope),
            None => common::require_test_database_url()
                .expect("TEST_DATABASE_URL is required for the route acceptance binary"),
        };
        let pool = api_database::connect(&url).await.expect("connect");
        api_database::migrate(&pool).await.expect("migrate");
        let mut config = Config::for_tests(url, JWT_SECRET);
        W::configure(&mut config);
        let mut state = api_server::composition::application_state(pool, config);
        let actors = Actors::create(&state, suite, namespace).await;
        let part = W::build(&mut state, &actors).await;
        let app = router(state.clone());
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
