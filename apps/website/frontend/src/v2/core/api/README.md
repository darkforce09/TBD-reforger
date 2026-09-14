# API & Network Core (`src/v2/core/api`)

## Responsibilities
- **HTTP Client (`client.rs`):** Gloo-net fetch wrapper with single-flight refresh token rotation (preventing double-spend).
- **SSE Client (`sse.rs`):** Real-time server-sent event listener for live notifications and telemetry.
- **DTOs (`dto/`):** Strongly typed models mirroring the backend's snake_case JSON schemas, broken down by domain (`events.rs`, `missions.rs`, `servers.rs`, `auth.rs`).
