# Command Center Handlers (`command_center/handlers/`)

HTTP endpoint handlers for the Command Center dashboard bento, ranked leaderboards, and user career statistics cards.

---

## 1. Handlers & Route Mappings

### `live_dashboard.rs` (<220 LOC)
- **`GET /api/v1/dashboard`** (`get_dashboard`):
  - **Auth**: `AuthUser`
  - **Action**: Executes batched queries:
    1. Selects the next upcoming operation from `events` + `event_missions`, prioritizing operations authored or joined by the caller.
    2. Queries `orbat_slots` for any active assignment held by the caller (`assigned_to = me`).
    3. Fetches the primary online dedicated server status (`server_statuses LIMIT 1`).
    4. Retrieves the current modpack manifest (`modpacks WHERE is_current = true`).
    5. Selects the 3 most recent published announcements.
  - **Response**: Emits `DashboardPulse`.

### `leaderboards.rs` (<260 LOC)
- **`GET /api/v1/leaderboards`** (`get_leaderboards`):
  - **Auth**: `AuthUser`
  - **Query**: `category` (`kd`, `command_win`, `missions`, `longest_kill`, `team_kills`), `limit`, `offset`.
  - **SQL**: Reads strictly from `leaderboard_totals` materialized view.
  - **Tie-Breaker**: Appends `, lt.discord_id ASC` to ensure deterministic paging order across identical stats.

### `user_stats.rs` (<140 LOC)
- **`GET /api/v1/users/{discordId}/stats`** (`get_user_stats`):
  - **Auth**: `AuthUser`
  - **Action**: Queries `users` and `leaderboard_totals` to assemble the member's complete career dossier.

---

## 2. Invariants & Rules

1. **Law 7 Compliance**: All handler files remain strictly under 300 LOC.
2. **Zero Aggregation on View**: No `SUM` or `COUNT` queries are run across raw telemetry ticks during dashboard or leaderboard requests. All stats are served from the pre-aggregated `leaderboard_totals` materialized view.
