//! Aggregate figures: the dashboard summary with its fleet, and the leaderboards.
//!
//! **Role:** the derived numbers the platform reports back — what the dashboard shows at a
//! glance (the configured fleet and its totals among it, and the newest announcements), and how
//! the leaderboards rank.
//! **Position:** deserialised straight from the backend's JSON and handed to the pages that
//! render it; re-serialised unchanged by the round-trip tests.
//! **Signals & state:** none — these are plain data.
//! **Invariants:** the fleet totals are the backend's sums over the active servers and are
//! rendered as sent, never re-added from the rows. The fleet's server statuses carry the
//! telemetry queue reading the contract defines. A null the backend sends — an unmeasured K/D
//! ratio — stays an explicit `null` when serialising.
//! @contract match-telemetry.schema.json#/definitions/TelemetryQueueStatus
//! @contract command-center.schema.json#/definitions/Dashboard
//! @contract command-center.schema.json#/definitions/LeaderboardPage

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::content::{Announcement, ModpackDto};
use super::identifiers::{DiscordUserId, ServerId};
use super::servers::ServerStatusDto;

/// Everything the dashboard renders in one payload.
/// @contract command-center.schema.json#/definitions/Dashboard
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct DashboardResponse {
    /// The next scheduled event, as the API sends it; absent when none is scheduled.
    pub next_event: Option<Value>,
    /// The caller's slot in the next event, as the API sends it; absent when the caller holds none.
    pub my_assignment: Option<Value>,
    /// The game-server fleet's overview.
    pub fleet: FleetOverviewDto,
    /// The modpack the community plays now; absent when none is marked current.
    pub current_modpack: Option<ModpackDto>,
    /// The three newest published announcements, newest first.
    pub recent_announcements: Vec<Announcement>,
}

/// The configured fleet: every active server, ordered by name then id, with the fleet totals.
/// @contract command-center.schema.json#/definitions/FleetOverview
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FleetOverviewDto {
    /// Every registered game server with its live state.
    pub servers: Vec<FleetServerDto>,
    /// The fleet-wide counts.
    pub totals: FleetTotalsDto,
}

/// One active server of the fleet.
/// @contract command-center.schema.json#/definitions/FleetServer
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FleetServerDto {
    /// The server's registry id.
    pub server_id: ServerId,
    /// The server's display name.
    pub name: String,
    /// Absent when the server has no status row yet; such a server counts as offline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<ServerStatusDto>,
}

/// Aggregates over the fleet. `online`, `players` and `max_players` count online servers only;
/// the two telemetry figures sum every reported queue reading.
/// @contract command-center.schema.json#/definitions/FleetTotals
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetTotalsDto {
    /// How many active servers the fleet has.
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

/// One page of a leaderboard, already ranked and filtered by the backend.
/// @contract command-center.schema.json#/definitions/LeaderboardPage
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Leaderboard {
    /// The ordering the page was ranked by: `kd`, `command_win`, `missions`, `longest_kill` or
    /// `team_kills`.
    pub category: String,
    /// The ranked rows, best first.
    pub data: Vec<LeaderboardRow>,
}

/// One member's combat totals on a leaderboard page.
///
/// `kd_ratio` is kills per death rounded to two places, `None` when no match recorded a death
/// count; `command_win_rate` is a fraction between zero and one; `rank` is the 1-based position
/// on the requested page.
/// @contract command-center.schema.json#/definitions/LeaderboardRow
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LeaderboardRow {
    /// The player's Discord account.
    pub discord_id: DiscordUserId,
    /// The player's display name; empty when the account has none.
    pub username: String,
    /// The avatar address; empty when the member has none.
    pub avatar_url: String,
    /// Kills over every recorded match.
    pub kills: i64,
    /// Deaths over every recorded match.
    pub deaths: i64,
    /// `NULL` when no `match_player_stats` row for this player has a measured `deaths` reading.
    /// Distinct from `0.0` (measured zero-death / flawless aggregate).
    pub kd_ratio: Option<f64>,
    /// Team kills over every recorded match.
    pub team_kills: i64,
    /// The longest recorded kill, in metres.
    pub longest_kill_m: i64,
    /// Vehicles destroyed over every recorded match.
    pub vehicles_destroyed: i64,
    /// Distinct matches the player has a stat line in.
    pub missions_played: i64,
    /// Command-role lines that recorded a command win.
    pub command_wins: i64,
    /// Command wins over command-role lines, from 0 to 1, rounded to three places; 0 with no
    /// command-role line.
    pub command_win_rate: f64,
    /// The 1-based place on the board the request reads, numbered by the API; 0 on the
    /// statistics card.
    pub rank: i64,
}
