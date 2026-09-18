# Server Infrastructure Tests (`server_infrastructure/tests/`)

Sibling unit and mock integration test specifications for dedicated server CRUD, RCON TCP socket protocol, and heartbeat state transitions.

---

## 1. Test Modules

Declared via Monorepo Law #7 (`#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`).

### `dedicated_servers.rs`
- **Coverage**: Server registration validation (valid IP/port ranges), uniqueness constraints on name and ports, pagination, and role-based access enforcement (`admin` only).

### `server_heartbeat.rs`
- **Coverage**: Heartbeat ingestion rate, authentication token verification, updating server player counts, and payload validation for simulation tick rates.

### `rcon_console.rs`
- **Coverage**: Command sanitization (blocking dangerous shell escapes), mock RCON command execution, timeout responses, and audit log generation.

### `rcon_protocol_client.rs`
- **Coverage**: Mock TCP server handshake, CRC32 packet checksum verification, multi-packet response reassembly, and connection drop recovery.

### `server_health_monitor.rs`
- **Coverage**: Status decay thresholds (30s online, 90s degraded, offline), timestamp drift resilience, and heartbeat alert triggers.
