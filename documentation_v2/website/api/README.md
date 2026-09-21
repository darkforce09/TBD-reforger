# Backend REST & Realtime API (`website/api/`)

The backend API service is an asynchronous Rust service powered by Axum, SQLx, and PostgreSQL, serving HTTP endpoints on port 8080 and broadcasting real-time state changes via Server-Sent Events (SSE).

## Subsystems
- `auth/`: Discord OAuth2, dev-login bypass for local development, JWT token issuance, session refresh rotation.
- `handlers/`: Modular HTTP handlers separated by resource (missions, events, ORBATs, telemetry, admin, wiki, vehicles).
- `models/`: Strictly typed Serde wire models and database records representing the canonical snake_case API contract.
- `services/`: Core business logic services enforcing database transactions and domain invariants.
- `realtime.rs`: In-memory SSE broadcast hub dispatching domain events to connected web clients.

## Code Mapping
- Source: `apps/website/api_v2/`
- Configuration: `apps/website/api_v2/.env`
- Migrations: `apps/website/api_v2/migrations/`
