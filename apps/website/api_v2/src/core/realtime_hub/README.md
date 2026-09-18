# Core Realtime Hub (`core/realtime_hub/`)

Server-Sent Events (SSE) broadcast infrastructure and real-time subscription routing for the platform suite.

---

## 1. Modules

### `broadcaster.rs` (<350 LOC)
- **Purpose**: Wraps `tokio::sync::broadcast` channels with lag handling and active subscriber tracking.
- **Invariants**:
  - Drops outdated messages if a slow client falls behind buffer capacity without stalling other subscribers.

### `sse_stream.rs` (<280 LOC)
- **Purpose**: Converts broadcast receiver streams into standard `axum::response::sse::Event` HTTP response streams.
- **Features**:
  - Implements automatic heartbeat pings (every 15s) to maintain open persistent connections through proxies and NAT gateways.

### `channel_registry.rs` (<320 LOC)
- **Channels**:
  - `Audit`: Real-time administrative audit event stream.
  - `Dashboard`: Live match score, player counts, and server status updates.
  - `Telemetry`: In-game combat events and killfeed ticks.
  - `Announcements`: Real-time alert notifications broadcast across the platform.
