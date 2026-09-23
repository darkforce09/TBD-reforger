# API & Network Core (`src/v2/core/api`)

## Responsibilities
- **HTTP Client (`client/`):** Gloo-net fetch wrapper with single-flight refresh token rotation (preventing double-spend).
  The plain verbs report a failure as `(status, message)`; the refusal-keeping verbs
  (`api_post_keeping_refusal` and its `PUT`, `PATCH` and `DELETE` siblings) report it as an
  `ApiRefusal` (`client/refusals.rs`) that keeps the `details.code` reason and its fields, for the
  routes whose callers branch on why a request was refused.
- **Endpoints (`endpoints/`):** one typed call per route of operation access administration
  (`event_access_administration.rs`), registration and waiting-list promotion
  (`event_registration.rs`), machine credentials (`machine_credentials.rs`), mission submission,
  reviews, artifacts and the review workspace (`mission_reviews.rs`), fleet commands
  (`fleet_commands.rs`), mission deployments and the reads their request form chooses from
  (`mission_deployments.rs`), and the fleet scenario registry (`fleet_scenarios.rs`), with pure
  path builders the native tests hold against the backend's route tables.
- **SSE Client (`sse.rs`):** Real-time server-sent event listener for live notifications and telemetry.
- **DTOs (`dto/`):** Strongly typed models mirroring the backend's snake_case JSON schemas, broken down by domain (`events.rs`, `event_viewer_access.rs`, `event_access_administration.rs`, `missions.rs`, `mission_reviews.rs`, `mission_deployments.rs`, `fleet_commands.rs`, `fleet_scenarios.rs`, `servers.rs`, `auth.rs`).
