# Server Infrastructure Subsystem (`server_infrastructure/`)

Unified dedicated server lifecycle management, modpack bindings, real-time heartbeat monitoring, SSE status streaming, and host agent RCON console execution.

---

## 1. Subsystem Topology & Responsibilities

The `server_infrastructure/` domain unifies server management and RCON, which were previously split across `handlers/telemetry/servers.rs`, `handlers/admin/admin.rs`, and `handlers/telemetry/leaderboards.rs`:

```text
src/server_infrastructure/
├── README.md                           <-- Domain documentation (this document)
├── routes.rs                           <-- /api/v1/servers & /api/v1/admin/servers sub-router (<70 LOC)
│
├── models/
│   ├── mod.rs
│   └── server.rs                       <-- Server, ServerStatus, ServerIntelDto, RconCommand (<120 LOC)
│
├── handlers/
│   ├── mod.rs
│   ├── server_registry.rs              <-- Dedicated server CRUD & modpack binding (<250 LOC)
│   ├── health_monitor.rs               <-- Status queries & live SSE stream (<260 LOC)
│   ├── modpack_binding.rs              <-- Modpack validation & prefetch helpers (<100 LOC)
│   └── rcon_console.rs                 <-- Remote console execution (<250 LOC)
│
├── services/
│   ├── game_agent.rs                   <-- Unix domain socket client communicating with host agent (<200 LOC)
│   └── tests/game_agent.rs             <-- Sibling unit tests (<150 LOC)
│
└── tests/                              <-- Non-inline sibling unit tests
    ├── server_registry.rs
    ├── health_monitor.rs
    └── rcon_console.rs
```

---

## 2. HTTP Route Catalog

| Verb | Path | Handler | Auth Extractor | Description |
|:---|:---|:---|:---|:---|
| `GET` | `/api/v1/servers` | `server_registry::list_servers` | `AuthUser` | List registered game servers with latest status snapshot and modpack. |
| `POST` | `/api/v1/servers` | `server_registry::create_server`| `AdminUser` | Register dedicated server (validates IP, port, modpack binding). |
| `PATCH` | `/api/v1/servers/{id}` | `server_registry::update_server`| `AdminUser` | Update server configuration or toggle active state. |
| `DELETE`| `/api/v1/servers/{id}` | `server_registry::deactivate_server`| `AdminUser`| Soft-deactivate server (`is_active = false`). |
| `GET` | `/api/v1/servers/{id}/status` | `health_monitor::get_server_status`| `AuthUser` | Fetch latest cached status row for specific server. |
| `GET` | `/api/v1/servers/{id}/status/stream`| `health_monitor::stream_server_status`| `AuthUser` (SSE)| Real-time SSE server status feed (transferred from leaderboards). |
| `POST` | `/api/v1/admin/servers/{id}/rcon` | `rcon_console::send_rcon` | `AdminUser` | Remote console execution via host agent Unix socket (replaces admin RCON). |

---

## 3. Host Control Agent Architecture (`game_agent.rs`)

The API communicates with Reforger dedicated game servers via a local Unix domain socket rendered by systemd (`tbd-reforger-agent.sock`):
1. **Security by Operating System**: The socket lives at `$XDG_RUNTIME_DIR/tbd-reforger-agent.sock`, mode `0600`, owned by the service user. The OS permissions guarantee that only the backend process under the same UID can issue verbs.
2. **Strict Protocol Verbs**: Commands (`status`, `start`, `stop`, `restart`) are serialized as single-line strings with immediate write half-close.
3. **Structured Verdicts**: Responses decode into `AgentReply`:
   - `accepted`: Command executed successfully; unit transitioned.
   - `rejected`: Command refused by host agent (e.g., active match running).
   - `unreachable`: Socket transport failed or agent timed out (20s timeout).
4. **Audit Trail**: Every RCON command issues an immutable audit log row recording actor, server name, command detail, and outcome severity.
