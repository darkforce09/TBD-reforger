# Command Center Subsystem (`command_center/`)

Public community operations dashboard, real-time match pulse, ranked leaderboards, and member career service statistics.

---

## 1. Subsystem Topology & Responsibilities

The `command_center/` domain decouples user-facing presentation from raw telemetry ingest:

```text
src/command_center/
├── README.md                           <-- Domain documentation (this document)
├── routes.rs                           <-- /api/v1/dashboard & /api/v1/leaderboards sub-router (<60 LOC)
│
├── models/
│   ├── mod.rs
│   └── command_center.rs               <-- DashboardPulse, LeaderboardRow, UserStatsCard (<120 LOC)
│
├── handlers/
│   ├── mod.rs
│   ├── live_dashboard.rs               <-- Home bento aggregator: next op, slot, modpack, news (<220 LOC)
│   ├── leaderboards.rs                 <-- Ranked player/team leaderboards with paging (<260 LOC)
│   └── user_stats.rs                   <-- Individual player career statistics readout (<140 LOC)
│
├── services/
│   └── user_stats.rs                   <-- Recalculates denormalized attendance rate & deployments (<135 LOC)
│
└── tests/                              <-- Non-inline sibling unit tests
    ├── live_dashboard.rs
    ├── leaderboards.rs
    └── user_stats.rs
```

---

## 2. HTTP Route Catalog

| Verb | Path | Handler | Auth Extractor | Description |
|:---|:---|:---|:---|:---|
| `GET` | `/api/v1/dashboard` | `live_dashboard::get_dashboard` | `AuthUser` | Single-query composite bento card: next operation, user's slot, server status, modpack, news. |
| `GET` | `/api/v1/leaderboards` | `leaderboards::get_leaderboards` | `AuthUser` | Ranked community leaderboards (`kd`, `command_win`, `missions`, `longest_kill`, `team_kills`). |
| `GET` | `/api/v1/users/{discordId}/stats`| `user_stats::get_user_stats` | `AuthUser` | Individual member service statistics readout from `leaderboard_totals` view. |

---

## 3. Key Invariants & Query Architecture

### 3.1 Materialized View Separation (`leaderboard_totals`)
All leaderboard queries read strictly from the `leaderboard_totals` materialized view. This eliminates heavy aggregations over millions of combat ticks during page views. The view is refreshed periodically by the dedicated `background_workers::leaderboard_refresher` task.

### 3.2 Deterministic Ordering Tie-Breaker
To prevent pagination jitter where players with identical stats flip positions across page boundaries, every whitelisted `ORDER BY` clause appends a deterministic tie-breaker:
```sql
ORDER BY kd_ratio DESC, lt.discord_id ASC
LIMIT $1 OFFSET $2
```

### 3.3 Dashboard Null Tolerance
If no server is online, no match is in progress, or no operations are scheduled, `live_dashboard::get_dashboard` returns structured JSON `null` values for those bento tiles rather than failing the request with HTTP 500.
