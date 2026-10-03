//! Background workers: the interval tasks the API binary arms at boot and never awaits.
//!
//! **Role:** each worker keeps shared state current when no request would (expired credentials,
//! the stored event status, the leaderboard view, live server status, silent runtime sessions,
//! fleet commands and mission deployments in flight, queued reservation re-evaluations,
//! unpublished audit facts, Discord membership and roles, new equipment export publications);
//! [`spawn_all`] arms them all and returns their [`WorkerHandles`].
//! **Position:** above `api_state` (the application state the workers read), `api_http_layer`
//! (the realtime hub, the durable rate limiter), `api_member_activity`, `api_equipment_datasets`
//! and the domain crates whose services each pass calls (`api_administration`,
//! `api_identity_and_access`, `api_missions`, `api_operations`, `api_server_infrastructure`). The
//! API binary (`apps/api/src/bin/api.rs`) is its one production caller; integration suites run
//! single passes directly.
//! **Signals & state:** one Tokio task per worker, each owning the clone of the state, the pool
//! or the hub it was handed; the work queues and leases live in Postgres.
//! **Invariants:** a worker owns its schedule, never its work: every pass calls a service of the
//! domain that owns the data; a failed pass is logged and the next one retries; the workers that
//! poll stop once the pool is closed.

pub mod audit_publication_worker;
pub mod discord_membership_reconciler;
pub mod discord_role_synchronizer;
pub mod equipment_export_watcher;
mod error;
pub mod event_lifecycle_sweeper;
pub mod event_reservation_reevaluator;
pub mod fleet_command_reconciler;
pub mod leaderboard_refresher;
pub mod mission_deployment_reconciler;
pub mod prelude;
pub mod ratelimit_cleanup_worker;
pub mod runtime_session_expiry;
pub mod server_status_publisher;
pub mod token_purge_worker;
pub mod worker_set;

pub use error::{Error, Result};
pub use worker_set::{WorkerHandles, spawn_all};
