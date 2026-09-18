# Server Infrastructure Models (`server_infrastructure/models/`)

Database entities, wire transfer contracts, and internal status representations for dedicated game servers, RCON protocol sessions, and cluster telemetry.

---

## 1. Domain Entities & Schemas

### `server.rs` (<350 LOC)
- **Database Table**: `servers`
- **Fields**:
  - `id: Uuid`: Unique server identifier.
  - `name: String`: Human-readable server label (e.g., "TBD Primary Dedicated #1").
  - `ip_address: String`: IPv4 / IPv6 network address.
  - `game_port: u16`: Enfusion game traffic UDP port.
  - `query_port: u16`: A2S / Steam query UDP port.
  - `rcon_port: u16`: BattlEye / Enfusion RCON TCP port.
  - `rcon_password_hash: String`: Encrypted or hashed RCON credential.
  - `current_modpack_id: Option<Uuid>`: Foreign key referencing active modpack manifest.
  - `is_active: bool`: Administrative enable/disable switch.
  - `created_at: DateTime<Utc>`, `updated_at: DateTime<Utc>`.

### `server_status.rs` (<250 LOC)
- **Status Enum**: `ServerStatus` (`Online`, `Offline`, `Degraded`, `Updating`, `MatchInProgress`).
- **Heartbeat Contract**:
  - `player_count: u32`: Number of connected players.
  - `max_players: u32`: Max server capacity.
  - `server_fps: f32`: Current Enfusion simulation tick rate.
  - `current_mission_id: Option<Uuid>`: Active loaded mission.
  - `uptime_seconds: u64`: Continuous daemon run time.
  - `last_heartbeat_at: DateTime<Utc>`: Timestamp of latest ping.

### `rcon_payloads.rs` (<200 LOC)
- **`RconCommandRequest`**:
  - `command: String`: Sanitized console instruction (e.g., `#kick <id>`, `#restart`, `say <msg>`).
  - `timeout_ms: Option<u64>`: Request timeout override.
- **`RconCommandResponse`**:
  - `output: String`: Standard output text returned by the server console.
  - `execution_duration_ms: u64`: Round-trip execution latency.
  - `success: bool`: Command outcome indicator.
