# Wire types

The data transfer objects: one Rust shape per JSON body the [API](/documentation_v2/glossary/a_to_f.md#api)
sends or accepts, grouped by domain and re-exported flat from `mod.rs`, so a caller names the type
(`MissionDetail`) rather than the file it lives in.

## Contents

```text
apps/website/frontend/src/v2/core/api/dto/
├── auth.rs                         the viewer's profile, the Arma link, member and roster rows
├── common.rs                       the list envelopes: `Paginated`, `DataEnvelope`, `CursorList`
├── content.rs                      modpack rows and the current modpack
├── event_access_administration.rs  an event's access policies, groups, pools and every change body
├── event_viewer_access.rs          what the viewer may see and reserve in one event
├── events.rs                       the event list, ORBAT and hub, the service record, leave requests
├── fleet_commands.rs               fleet command receipts, their list and the request constructors
├── fleet_scenarios.rs              the terrain-to-mission-header entries and the body that sets one
├── mission_deployments.rs          a mission deployment, a server's page of them, the request body
├── mission_reviews.rs              review history and thread, decisions, artifacts, review workspace
├── missions.rs                     mission cards, rows, detail and versions; armory; approval rows
├── mod.rs                          the module tree; re-exports every DTO flat
├── registry.rs                     registry items, compatibility edges, cargo defaults and factions
├── servers.rs                      server rows, the live status frame and its decoder, credentials
├── telemetry.rs                    the dashboard summary, leaderboards and a fire solution
└── tests/                          unit tests for the golden round trips and the fixture-free shapes
```

## How it works

The DTOs mirror the snake_case models of the API in `apps/website/api_v2/src/<domain>/models/`,
and the API wins a disagreement. They are plain `serde` data, and all of them compile into the
native test build. `tests/r_api.rs` holds each DTO to a captured answer from
`apps/website/frontend/tests/fixtures/api/`: re-serialising reproduces the capture canonically,
byte for byte, and the keys no named field reads (the ones a `#[serde(flatten)]` catch-all sweeps
up, found by poisoning each value) are exactly the ones the test lists. The `tests/r_api_*.rs`
files hold one domain's goldens each; `tests/shapes.rs` checks the shapes that need no capture.

- A value set the API may extend (review states,
  [fleet command](/documentation_v2/glossary/a_to_f.md#fleet-command) actions and states,
  [mission deployment](/documentation_v2/glossary/g_to_m.md#mission-deployment) states, leave statuses,
  reservation values) travels as a string, so a new value lists instead of failing the read.
- A null the capture carries stays explicit when serialising: an unclaimed
  [ORBAT](/documentation_v2/glossary/n_to_z.md#orbat) [slot](/documentation_v2/glossary/n_to_z.md#slot), a
  finding with no subject, a pool with no limit.
- The [event](/documentation_v2/glossary/a_to_f.md#event) access conditions and group sources are tagged
  by `kind` and refuse unknown fields, because the
  [event manager](/documentation_v2/glossary/a_to_f.md#event-manager) sends them back and must not
  rewrite a shape it does not know; `MissionDetail` has no catch-all either.
- `IssuedMachineCredential`, the one answer that carries a
  [machine credential](/documentation_v2/glossary/g_to_m.md#machine-credential)'s secret, derives no
  `Debug`, so the secret cannot reach a log line.
- `decode_server_status_frame` turns one [SSE](/documentation_v2/glossary/n_to_z.md#sse) frame into a
  status or a named rejection, here rather than beside the browser-only stream reader so the
  native tests reach it; `compiled_meta` on `MissionDetail` and `ArtifactMetadata` gives the
  metadata the shared [mission](/documentation_v2/glossary/g_to_m.md#mission) compiler reads.

## Boundaries

- Depends on: `serde` and `serde_json`; `User` and `Role` of `crate::v2::core::auth`;
  `website_map_engine::data`, for the compiler metadata and the re-exported `FactionDoc`,
  `FactionRole`, `FactionVehicle` and `MissionEnv`.
- Used by: the client, endpoint calls and live status stream in
  `apps/website/frontend/src/v2/core/api/`, the auth store in
  `apps/website/frontend/src/v2/core/auth/store.rs`, the pages under
  `apps/website/frontend/src/v2/pages/`, and the
  [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) under
  `apps/website/frontend/src/v2/apps/editor/`.
- Rules: the API model changes first and the DTO follows; a golden round-trips, and its unread
  keys match its list, so drift either way fails (`cargo test -p website-frontend`);
  `byte_equality_alone_cannot_see_a_dropped_field_under_flatten` in `tests/r_api.rs` shows why the
  key check sits beside the byte comparison.

## Related documentation

- [Documentation standards](/documentation_v2/standards/documentation_standards.md#2-contracts-behind-the-tags)
  — how the API's models, the schemas and these DTOs stay one contract.
