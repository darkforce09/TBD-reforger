# API

REST communication layer interfacing the game server with the `website-api` platform.

### Roles & Responsibilities
- `TBD_BackendConfig.c`: Server configuration loader reading connection endpoints and security tokens from `$profile:TBD_BackendConfig.json`.
- `TBD_PlayerIdentity.c`: Canonical accessor for the player engine GUID (`arma_id`), guaranteeing byte-for-byte consistency across all backend endpoints.
- `TBD_IdentityLink.c`: Handles in-game `#tbd link <code>` chat commands and issues confirmation requests to the platform.
- `TBD_ResultsReporter.c`: End-of-round match telemetry reporter compiling player attendance, roles, and match outcomes.

### Call Flow & Contracts
Authority-only outbound HTTP client using Enfusion's `RestContext`. Authenticates via `X-Service-Token` to `website-api` ingest endpoints (`/api/v1/ingest/link-confirm`, `/api/v1/ingest/match-results`).
