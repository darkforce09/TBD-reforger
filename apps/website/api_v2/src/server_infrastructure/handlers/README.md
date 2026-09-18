# Server Infrastructure Handlers (`server_infrastructure/handlers/`)

HTTP endpoint controllers managing dedicated game servers, automated heartbeats, RCON remote console execution, and modpack manifest bindings.

---

## 1. Handlers & Route Mappings

### `dedicated_servers.rs` (<420 LOC)
- **`GET /api/v1/servers`** (`list_servers`): Lists all registered game servers with latest cached health and player counts.
- **`POST /api/v1/servers`** (`create_server`): Registers a new dedicated server host into the cluster. Requires `admin` role.
- **`GET /api/v1/servers/{id}`** (`get_server`): Detailed profile for a specific server instance.
- **`PATCH /api/v1/servers/{id}`** (`update_server`): Updates server configuration, ports, and display metadata. Requires `admin` role.
- **`DELETE /api/v1/servers/{id}`** (`delete_server`): De-registers server instance from the cluster. Requires `admin` role.

### `server_heartbeat.rs` (<350 LOC)
- **`POST /api/v1/servers/{id}/heartbeat`** (`ingest_heartbeat`):
  - Ingests periodic telemetry pings from the dedicated server agent.
  - Updates `last_heartbeat_at`, tick rate, and online player count in Redis/Postgres.
  - Authenticates via dedicated server machine bearer token.

### `rcon_console.rs` (<410 LOC)
- **`POST /api/v1/servers/{id}/rcon`** (`execute_rcon_command`):
  - Sends a remote console command to the live Enfusion server via TCP.
  - Requires `admin` role.
  - Validates command safety (prevents unauthorized RCON injection attacks).
  - Emits an administrative audit log for every dispatched command.

### `modpack_binding.rs` (<280 LOC)
- **`PUT /api/v1/servers/{id}/modpack`** (`bind_server_modpack`):
  - Assigns or updates the active modpack manifest for the dedicated server.
  - Triggers mod delta verification on the game host.
