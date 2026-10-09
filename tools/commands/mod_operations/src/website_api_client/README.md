# Website API client for the mod tooling

A small client that drives the website [API](/documentation/glossary/a_to_f.md#api) the way an
administrator does in development: log in, publish a [mission](/documentation/glossary/g_to_m.md#mission),
provision the fleet and request a [deployment](/documentation/glossary/a_to_f.md#deployment). It also
stages a compiled mission document in a profile's artifact cache and reads back the telemetry the
platform recorded for a server and its match.

## Contents

```text
tools/commands/mod_operations/src/website_api_client/
├── api_client.rs               `ApiClient`: base URL, bearer token, expected-status calls, refusal messages
├── development_login.rs        the development login and its access-token redirect
├── fleet_provisioning.rs       fleet scenarios, server rows, machine credentials, deployments, commands
├── http_exchange.rs            `ApiTransport`, the `curl` transport, answers, headers, URL encoding
├── mission_artifact_cache.rs   stage or clear `TBD_MissionArtifactCache/` in a mod profile
├── mission_publication.rs      create, read, submit, approve and delete missions; artifact documents
├── mod.rs                      the module tree; re-exports the client, flows and cache helpers
└── runtime_telemetry_reads.rs  a server's current match and telemetry queue reading; a match's events
```

## How it works

Every request goes through the `ApiTransport` trait. `CurlTransport` runs `curl -sS` with a
timeout, writes the response headers and body to its scratch folder, follows no redirect, and
returns every HTTP status as an answer; only no answer at all is an error. `ApiClient` names the
status each call expects and turns any other status into an error carrying the refusal code and
the start of the body.

The routes it calls, all under the API base (default `http://127.0.0.1:8080`):

- Login: `GET /api/v1/auth/dev-login?role=<role>`, which answers 302 with the access token in the
  URL fragment and exists only while the API runs with `APP_ENV=development`.
- Missions: `POST /api/v1/missions`, `GET` and `DELETE /api/v1/missions/{id}`,
  `GET /api/v1/missions?scope=mine`, `POST /api/v1/missions/{id}/submit`,
  `GET /api/v1/missions/{id}/reviews`, `POST /api/v1/approvals/{id}/approve`,
  `GET /api/v1/missions/{id}/artifacts/{artifact_id}` and its `/document`.
- Fleet: `GET /api/v1/fleet/scenarios`, `PUT /api/v1/fleet/scenarios/{terrainKey}`,
  `GET` and `POST /api/v1/servers`, `PATCH /api/v1/servers/{id}`,
  `POST /api/v1/servers/{id}/credentials`, `DELETE /api/v1/servers/{id}/credentials/{credentialId}`,
  `POST` and `GET /api/v1/servers/{id}/deployments[/{deploymentId}]`,
  `POST /api/v1/servers/{id}/commands/{commandId}/cancel`.
- Telemetry: `GET /api/v1/servers/{id}/status` (its `status.current_match_id` and
  `status.telemetry_queue {backlog, capacity, dropped_total, oldest_age_seconds, reported_at}`)
  and `GET /api/v1/matches/{matchId}/events?limit=1`, whose non-empty `items` means the match
  holds an acknowledged event. A present queue reading missing any field is an error.

`stage_artifact_cache` writes `document.json` and `identity.json` (the artifact id, mission id,
terrain key and the document's SHA-256) into `<profile>/TBD_MissionArtifactCache/`. The mod loads
the cached document only when its bytes hash to the recorded SHA-256.

## Boundaries

- Depends on: `curl` on the path; `process_runner`, `serde_json` and `content_digest`.
- Used by: `tools/commands/mod_operations/src/playtest_server/platform_deployment.rs`,
  `tools/commands/mod_operations/src/world_boot.rs` and its `compiled_lane.rs`,
  `tools/commands/mod_operations/src/mission_test.rs` and
  `tools/commands/mod_operations/src/game_runtime_api_smoke.rs`.
- Rules: a fetched artifact document must hash to its entity tag; a refused submission names its
  code; a partial queue reading is an error, never a zero; URL components are percent-encoded
  except unreserved bytes.

## Related documentation

- [Local development](/documentation/runbooks/local_development.md) — running the database and
  the API these flows need.
