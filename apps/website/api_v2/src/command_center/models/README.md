# Command Center Domain Models (`command_center/models/`)

Presentation DTOs and data models for the community Command Center dashboard, live match pulse, and ranked leaderboards.

---

## 1. Model Catalog

### `DashboardPulse` (`command_center.rs`)
- **JSON Wire Contract**:
  ```rust
  #[derive(Debug, Serialize)]
  pub struct DashboardPulse {
      pub next_event: Option<EventMissionDossier>,
      pub my_slot: Option<OrbatSlotAssignment>,
      pub primary_server: Option<ServerIntelDto>,
      pub current_modpack: Option<ModpackDto>,
      pub recent_announcements: Vec<AnnouncementSummary>,
  }
  ```
- **Invariants**:
  - Null-tolerant: If no operation is scheduled or no server is active, fields serialize as explicit `null` rather than failing the request.

### `LeaderboardRow` (`command_center.rs`)
- **Query Source**: `leaderboard_totals` materialized view
- **Fields**:
  - `rank: i64`
  - `discord_id: String`
  - `username: String`
  - `avatar_url: Option<String>`
  - `kills: i64`
  - `deaths: i64`
  - `kd_ratio: f64` (Float projection of `numeric(5,2)`)
  - `longest_kill_m: i64`
  - `team_kills: i64`
  - `missions_played: i64`
  - `command_win_rate: Option<f64>` (Tri-state command win rate)
  - `attendance_rate: f64`

### `UserStatsCard` (`command_center.rs`)
- **Query Source**: `leaderboard_totals` joined with `users`
- **Fields**:
  - `discord_id: String`
  - `username: String`
  - `role: UserRole`
  - `total_deployments: i64`
  - `attendance_rate: f64`
  - `kd_ratio: f64`
  - `favorite_weapon: Option<String>`
