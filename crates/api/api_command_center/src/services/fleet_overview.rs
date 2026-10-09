//! The dashboard's view of the configured fleet: every active server with its status, and the
//! fleet totals.
//!
//! **Role:** selects the configured fleet (`servers.is_active`) and aggregates it.
//! **Position:** read by [`crate::handlers::live_dashboard`]; reads `servers` and `server_statuses`
//! through the server-infrastructure status projection.
//! **Signals & state:** none; one read per call.
//! **Invariants:** the fleet is exactly the active servers, ordered by name then id; an inactive
//! server never appears in the list or the totals; a server without a status row is listed with
//! no status and counts as offline; totals sum only online servers' players and capacity and
//! every reported queue's backlog and drops.
//!
//! @contract command-center.schema.json#/definitions/FleetOverview
//! @contract command-center.schema.json#/definitions/FleetServer
//! @contract command-center.schema.json#/definitions/FleetTotals

use api_identifiers::ServerId;
use serde::Serialize;
use sqlx::PgPool;

use api_server_infrastructure::models::server::{ServerStatus, ServerStatusRow};
use api_server_infrastructure::services::status_broadcast::SELECT_FLEET_STATUSES;

/// One server of the fleet.
#[derive(Debug, Serialize)]
pub struct FleetServer {
    /// The server's registry id.
    pub server_id: ServerId,
    /// The server's display name.
    pub name: String,
    /// The server's latest status; absent when it has never reported one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ServerStatus>,
}

/// Aggregates over the fleet.
#[derive(Debug, Default, PartialEq, Eq, Serialize)]
pub struct FleetTotals {
    /// The active servers.
    pub configured: i64,
    /// The active servers whose status reads online.
    pub online: i64,
    /// The players on the online servers.
    pub players: i64,
    /// The player capacity of the online servers.
    pub max_players: i64,
    /// The summed backlog of every reported telemetry queue.
    pub telemetry_backlog: i64,
    /// The summed drop count of every reported telemetry queue.
    pub telemetry_dropped_total: i64,
}

/// The configured fleet and its totals.
#[derive(Debug, Serialize)]
pub struct FleetOverview {
    /// The active servers, ordered by name then id.
    pub servers: Vec<FleetServer>,
    /// The totals over [`FleetOverview::servers`].
    pub totals: FleetTotals,
}

#[derive(sqlx::FromRow)]
struct FleetServerName {
    id: ServerId,
    name: String,
}

const SELECT_FLEET_SERVERS: &str =
    "SELECT id, name FROM servers WHERE is_active ORDER BY name ASC, id ASC";

/// Fold the fleet's statuses into its totals.
pub fn fleet_totals(servers: &[FleetServer]) -> FleetTotals {
    let mut totals = FleetTotals {
        configured: servers.len() as i64,
        ..FleetTotals::default()
    };
    for status in servers.iter().filter_map(|server| server.status.as_ref()) {
        if status.is_online {
            totals.online += 1;
            totals.players += status.player_count;
            totals.max_players += status.max_players;
        }
        if let Some(queue) = &status.telemetry_queue {
            totals.telemetry_backlog += queue.backlog;
            totals.telemetry_dropped_total += queue.dropped_total;
        }
    }
    totals
}

/// Load the configured fleet with each server's status and the totals.
///
/// A failed read of the servers or their status rows is [`crate::Error::Database`].
pub async fn load_fleet_overview(pool: &PgPool) -> crate::Result<FleetOverview> {
    let names: Vec<FleetServerName> = sqlx::query_as(SELECT_FLEET_SERVERS).fetch_all(pool).await?;
    let mut statuses: Vec<ServerStatus> =
        sqlx::query_as::<_, ServerStatusRow>(SELECT_FLEET_STATUSES)
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(ServerStatus::from)
            .collect();
    let servers: Vec<FleetServer> = names
        .into_iter()
        .map(|server| {
            let status = statuses
                .iter()
                .position(|status| status.server_id == server.id)
                .map(|index| statuses.swap_remove(index));
            FleetServer {
                server_id: server.id,
                name: server.name,
                status,
            }
        })
        .collect();
    let totals = fleet_totals(&servers);
    Ok(FleetOverview { servers, totals })
}
