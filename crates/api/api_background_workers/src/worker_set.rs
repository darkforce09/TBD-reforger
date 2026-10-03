//! The set of background workers: the one call that arms them all and the handles it returns.
//!
//! **Role:** [`spawn_all`] starts every interval task of the crate against one application state
//! and returns their [`WorkerHandles`].
//! **Position:** the API binary calls [`spawn_all`] once at boot, after the migrations and before
//! it builds the router; each worker module it arms calls a service of the domain that owns the
//! data.
//! **Signals & state:** none of its own; each spawned Tokio task owns the clone of the state or
//! the pool it was handed, and runs until the runtime stops or its handle is aborted.
//! **Invariants:** every worker is armed exactly once per call; the boot log states the interval
//! each worker resolved, the three tunable ones included.

use tokio::task::JoinHandle;

use api_state::AppState;

use crate::{
    audit_publication_worker, discord_membership_reconciler, discord_role_synchronizer,
    equipment_export_watcher, event_lifecycle_sweeper, event_reservation_reevaluator,
    fleet_command_reconciler, leaderboard_refresher, mission_deployment_reconciler,
    ratelimit_cleanup_worker, runtime_session_expiry, server_status_publisher, token_purge_worker,
};

/// The handles [`spawn_all`] returns, one per worker. Holding them keeps the set addressable
/// (a handle can be aborted to stop that worker alone); dropping them detaches the tasks,
/// which keep running until the runtime stops.
pub struct WorkerHandles {
    /// The equipment export watcher.
    pub equipment_exports: JoinHandle<()>,
    /// The refresh token purge.
    pub token_purge: token_purge_worker::PurgeHandle,
    /// The event lifecycle sweeper.
    pub event_lifecycle: event_lifecycle_sweeper::LifecycleHandle,
    /// The leaderboard view refresher.
    pub leaderboard_refresh: JoinHandle<()>,
    /// The server status republisher.
    pub server_status_publish: JoinHandle<()>,
    /// The Discord role resync.
    pub discord_role_resync: JoinHandle<()>,
    /// The Discord membership reconciler.
    pub discord_membership_reconcile: JoinHandle<()>,
    /// The durable rate-limit bucket pruner.
    pub ratelimit_cleanup: JoinHandle<()>,
    /// The audit publication worker.
    pub audit_publication: JoinHandle<()>,
    /// The event reservation reevaluator.
    pub event_reservation_reevaluation: JoinHandle<()>,
    /// The runtime session expiry.
    pub runtime_session_expiry: JoinHandle<()>,
    /// The fleet command reconciler.
    pub fleet_command_reconciliation: JoinHandle<()>,
    /// The mission deployment reconciler.
    pub mission_deployment_reconciliation: JoinHandle<()>,
}

/// Arm every background worker against `state`, logging the cadence each one actually got so
/// the boot log states the resolved intervals rather than leaving the env-tunable ones implicit.
pub fn spawn_all(state: &AppState) -> WorkerHandles {
    let leaderboard = leaderboard_refresher::leaderboard_refresh_interval();
    let server_status = server_status_publisher::server_status_publish_interval();
    let role_resync = discord_role_synchronizer::role_resync_interval();
    tracing::info!(
        secs = leaderboard.as_secs(),
        "leaderboard MV scheduled refresh armed"
    );
    tracing::info!(
        secs = server_status.as_secs(),
        "server-status SSE republish armed"
    );
    tracing::info!(secs = role_resync.as_secs(), "discord role resync armed");
    tracing::info!(
        refresh_token_purge_secs = token_purge_worker::PURGE_INTERVAL.as_secs(),
        event_lifecycle_secs = event_lifecycle_sweeper::LIFECYCLE_INTERVAL.as_secs(),
        rate_limit_prune_secs = ratelimit_cleanup_worker::RATE_LIMIT_PRUNE_INTERVAL.as_secs(),
        runtime_session_expiry_secs =
            runtime_session_expiry::RUNTIME_SESSION_EXPIRY_INTERVAL.as_secs(),
        fleet_command_reconciliation_secs =
            fleet_command_reconciler::FLEET_COMMAND_RECONCILIATION_INTERVAL.as_secs(),
        mission_deployment_reconciliation_secs =
            mission_deployment_reconciler::MISSION_DEPLOYMENT_RECONCILIATION_INTERVAL.as_secs(),
        "fixed-cadence workers armed"
    );

    WorkerHandles {
        equipment_exports: equipment_export_watcher::start(state.equipment_data.clone()),
        discord_membership_reconcile:
            discord_membership_reconciler::start_membership_reconciliation(state.clone()),
        audit_publication: audit_publication_worker::start_audit_publication(state.pool.clone()),
        event_reservation_reevaluation:
            event_reservation_reevaluator::start_event_reservation_reevaluation(state.clone()),
        runtime_session_expiry: runtime_session_expiry::start_runtime_session_expiry(state.clone()),
        fleet_command_reconciliation: fleet_command_reconciler::start_fleet_command_reconciliation(
            state.pool.clone(),
        ),
        mission_deployment_reconciliation:
            mission_deployment_reconciler::start_mission_deployment_reconciliation(
                state.pool.clone(),
            ),
        token_purge: token_purge_worker::start_refresh_token_purge(state.pool.clone()),
        event_lifecycle: event_lifecycle_sweeper::start_event_lifecycle(state.pool.clone()),
        leaderboard_refresh: leaderboard_refresher::start_leaderboard_refresh(
            state.pool.clone(),
            leaderboard,
        ),
        server_status_publish: server_status_publisher::start_server_status_publisher(
            state.pool.clone(),
            state.hub.clone(),
            server_status,
        ),
        discord_role_resync: discord_role_synchronizer::start_role_resync(
            state.pool.clone(),
            state.cfg.discord_guild_id.clone(),
            role_resync,
        ),
        ratelimit_cleanup: ratelimit_cleanup_worker::start_rate_limit_prune(
            state.pool.clone(),
            ratelimit_cleanup_worker::RATE_LIMIT_BUCKET_TTL,
            ratelimit_cleanup_worker::RATE_LIMIT_PRUNE_INTERVAL,
        ),
    }
}
