//! The dashboard's view of the configured fleet: every active server with its status, and the
//! fleet totals.
//!
//! **Role:** selects the configured fleet (`servers.is_active`) and aggregates it.
//! **Position:** read by `handlers::live_dashboard`; reads `servers` and `server_statuses`
//! through the server-infrastructure status projection.
//! **Signals & state:** none; one read per call.
//! **Invariants:** the fleet is exactly the active servers, ordered by name then id; an inactive
//! server never appears in the list or the totals; a server without a status row is listed with
//! no status and counts as offline; totals sum only online servers' players and capacity and
//! every reported queue's backlog and drops.

use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::server_infrastructure::models::server::{ServerStatus, ServerStatusRow};
use crate::server_infrastructure::services::status_broadcast::SELECT_FLEET_STATUSES;

/// One server of the fleet.
#[derive(Debug, Serialize)]
pub struct FleetServer {
    pub server_id: Uuid,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ServerStatus>,
}

/// Aggregates over the fleet.
#[derive(Debug, Default, PartialEq, Eq, Serialize)]
pub struct FleetTotals {
    pub configured: i64,
    pub online: i64,
    pub players: i64,
    pub max_players: i64,
    pub telemetry_backlog: i64,
    pub telemetry_dropped_total: i64,
}

/// The configured fleet and its totals.
#[derive(Debug, Serialize)]
pub struct FleetOverview {
    pub servers: Vec<FleetServer>,
    pub totals: FleetTotals,
}

#[derive(sqlx::FromRow)]
struct FleetServerName {
    id: Uuid,
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
pub async fn load_fleet_overview(pool: &PgPool) -> sqlx::Result<FleetOverview> {
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

#[cfg(test)]
#[path = "tests/fleet_overview.rs"]
mod tests;
