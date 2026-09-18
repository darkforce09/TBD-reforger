# Server Infrastructure Services (`server_infrastructure/services/`)

Core networking protocols and health monitoring engines for managing Enfusion dedicated server instances.

---

## 1. Domain Services

### `rcon_protocol_client.rs` (<420 LOC)
- **Purpose**: Low-level TCP client implementing the BattlEye/Enfusion RCON protocol specification.
- **Key Functions**:
  - `connect(addr: SocketAddr, password: &str) -> Result<RconSession, RconError>`: Performs handshake, sends login packet (`0xFF 0x00`), and verifies authentication token.
  - `send_command(session: &mut RconSession, command: &str) -> Result<String, RconError>`: Dispatches command packet, awaits fragmented response packets, and reassembles ASCII payload.
- **Invariants**:
  - Handles connection timeouts gracefully (default 5000ms deadline).
  - Mutex-locked session pools per server to prevent socket multiplexing corruption.

### `server_health_monitor.rs` (<350 LOC)
- **Purpose**: Tracks server liveness, detects network partitions, and marks unresponsive instances.
- **Key Functions**:
  - `evaluate_server_health(last_heartbeat: DateTime<Utc>, current_time: DateTime<Utc>) -> ServerStatus`: Returns `Online` if pinged within 30s, `Degraded` within 90s, and `Offline` thereafter.
  - `trigger_server_offline_alert(server_id: Uuid, name: &str)`: Emits alert to administration audit stream and Discord operations channel.
