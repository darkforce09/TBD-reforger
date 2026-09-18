# Match Telemetry Subsystem (`match_telemetry/`)

High-frequency game-server heartbeat ingestion, combat event tracking, player statistics counters, event mission attendance attribution, and AAR replay streaming.

---

## 1. Subsystem Topology & Responsibilities

The `match_telemetry/` domain isolates dedicated game-server telemetry ingestion from user-facing dashboard queries (which are re-homed to `command_center/`):

```text
src/match_telemetry/
├── README.md                           <-- Domain documentation (this document)
├── routes.rs                           <-- /api/v1/ingest sub-router (<60 LOC)
│
├── models/
│   ├── mod.rs
│   └── telemetry.rs                    <-- Match, MatchPlayerStats, ServerStatusHistory, CombatEvent (<160 LOC)
│
├── handlers/
│   ├── mod.rs
│   ├── server_heartbeat.rs             <-- Heartbeat tick receiver & low-FPS warnings (<250 LOC)
│   ├── match_results.rs                <-- Combat event ingest & player counter updates (<320 LOC)
│   ├── attendance_attribution.rs       <-- Event mission attendance backfill & retraction (<200 LOC)
│   └── replay_streamer.rs              <-- AAR replay URL validation & tick streaming (<180 LOC)
│
└── tests/                              <-- Non-inline sibling unit tests
    ├── server_heartbeat.rs
    ├── match_results.rs
    └── attendance_attribution.rs
```

---

## 2. HTTP Route Catalog

| Verb | Path | Handler | Auth Extractor | Description |
|:---|:---|:---|:---|:---|
| `POST` | `/api/v1/ingest/server-status` | `server_heartbeat::ingest_server_status` | `ServiceAuth` | Upsert live server status, record history, warn on low-FPS threshold, fan out to SSE. |
| `POST` | `/api/v1/ingest/match-results` | `match_results::ingest_match_results` | `ServiceAuth` | Ingest match completion stats, update player kill/death/TK counters, attribute attendance. |

---

## 3. Key Invariants & Ingestion Rules

### 3.1 Non-Destructive Heartbeat Merging (`COALESCE`)
`server_heartbeat::ingest_server_status` receives periodic heartbeats from running game servers. To ensure an incomplete heartbeat (e.g. reporting player count without map coordinates) never overwrites existing state with default zeroes, the upsert query reads bind parameters against existing table columns:
```sql
INSERT INTO server_statuses (server_id, is_online, player_count, max_players, server_fps, ...)
VALUES ($1, COALESCE($2, false), COALESCE($3, 0), ...)
ON CONFLICT (server_id) DO UPDATE SET
  is_online = COALESCE($2, server_statuses.is_online),
  player_count = COALESCE($3, server_statuses.player_count),
  server_fps = COALESCE($5::float8::numeric, server_statuses.server_fps),
  ...
```

### 3.2 Low-FPS Edge Warning
An audit log warning (`server.low_fps` at `AuditSeverity::Warn`) is edge-triggered strictly when server performance drops across the 20.0 FPS boundary (`prev_fps >= 20.0 && current_fps < 20.0`), preventing notification flooding on persistent degraded performance.

### 3.3 Exact Attendance Attribution
Attendance is marked `attended` in `event_registrations` strictly for the exact `(event_id, mission_id)` verified by the match result payload. If a match result is later retracted or re-pointed, `attendance_attribution::retract_attendance` safely resets registration states.

### 3.4 Unlinked Player Retention
If an Arma player is not yet linked to a Discord identity, their telemetry counters and kill events are preserved in `match_player_stats` under a null `discord_id`. When the player subsequently confirms their account link via `/ingest/link-confirm`, these historical stats are automatically backfilled.
