# Match Telemetry Handlers (`match_telemetry/handlers/`)

HTTP endpoint controllers for high-frequency game tick ingestion, combat event reporting, player session reservation tokens, and After-Action Review (AAR) telemetry streaming.

---

## 1. Handlers & Route Mappings

### `tick_ingest.rs` (<400 LOC)
- **`POST /api/v1/telemetry/ticks`** (`ingest_ticks`):
  - Ingests batch arrays of positional player ticks directly from the game server runtime.
  - Authenticated via dedicated server secret token.
  - Buffered and dispatched to the asynchronous database copy worker.

### `combat_events.rs` (<380 LOC)
- **`POST /api/v1/telemetry/events`** (`ingest_combat_events`):
  - Ingests gameplay combat events (kills, injuries, friendly-fire strikes, vehicle destructions).
  - Validates session existence and participant references.
  - Broadcasts live killfeed notifications to the real-time event hub.

### `session_tokens.rs` (<280 LOC)
- **`POST /api/v1/telemetry/session-token`** (`generate_session_token`):
  - Mints an ephemeral, single-use reservation token for authenticated players connecting to the game server.
  - Verifies ORBAT slot assignment and active operation status.
  - Game server verifies this token upon client handshake to assign player identity and reserved slot.

### `aar_replay.rs` (<420 LOC)
- **`GET /api/v1/telemetry/sessions/{id}/replay`** (`stream_replay`):
  - Streams time-indexed binary delta frames for a completed match session.
  - Supports range requests and chunked streaming for the frontend 2D/3D AAR visualizer.
