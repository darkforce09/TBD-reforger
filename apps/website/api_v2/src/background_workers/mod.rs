//! Long-running tasks the API binary spawns at boot and never awaits: each one polls or
//! recomputes something on an interval so a quiet request path cannot leave shared state stale.

use tokio::task::JoinHandle;

use crate::core::application_state::AppState;

pub mod audit_publication_worker;
pub mod discord_membership_reconciler;
pub mod discord_role_synchronizer;
pub mod event_lifecycle_sweeper;
pub mod event_reservation_reevaluator;
pub mod fleet_command_reconciler;
pub mod leaderboard_refresher;
pub mod mission_deployment_reconciler;
pub mod ratelimit_cleanup_worker;
pub mod runtime_session_expiry;
pub mod server_status_publisher;
pub mod token_purge_worker;

/// The handles [`spawn_all`] returns, one per worker. Holding them keeps the set addressable
/// (a handle can be aborted to stop that worker alone); dropping them detaches the tasks,
/// which keep running until the runtime stops.
pub struct WorkerHandles {
    pub token_purge: token_purge_worker::PurgeHandle,
    pub event_lifecycle: event_lifecycle_sweeper::LifecycleHandle,
    pub leaderboard_refresh: JoinHandle<()>,
    pub server_status_publish: JoinHandle<()>,
    pub discord_role_resync: JoinHandle<()>,
    pub discord_membership_reconcile: JoinHandle<()>,
    pub ratelimit_cleanup: JoinHandle<()>,
    pub audit_publication: JoinHandle<()>,
    pub event_reservation_reevaluation: JoinHandle<()>,
    pub runtime_session_expiry: JoinHandle<()>,
    pub fleet_command_reconciliation: JoinHandle<()>,
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
