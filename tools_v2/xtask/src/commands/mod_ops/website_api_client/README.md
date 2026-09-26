# Website API client for the mod tooling

A small client that drives the website [API](/documentation_v2/glossary/a_to_f.md#api) the way an
administrator does in development: log in, publish a [mission](/documentation_v2/glossary/g_to_m.md#mission),
provision the fleet and request a [deployment](/documentation_v2/glossary/a_to_f.md#deployment). It also
stages a compiled mission document in a profile's artifact cache.

## Contents

```text
tools_v2/xtask/src/commands/mod_ops/website_api_client/
├── api_client.rs              `ApiClient`: base URL, bearer token, expected-status calls, refusal messages
├── development_login.rs       the development login and its access-token redirect
├── fleet_provisioning.rs      fleet scenarios, server rows, machine credentials, deployments, commands
├── http_exchange.rs           `ApiTransport`, the `curl` transport, answers, headers, URL encoding
├── mission_artifact_cache.rs  stage or clear `TBD_MissionArtifactCache/` in a mod profile
├── mission_publication.rs     create, read, submit, approve and delete missions; artifact documents
├── mod.rs                     the module tree; re-exports the client, flows and cache helpers
└── tests/                     unit tests for the flows and the transport parsing
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

`stage_artifact_cache` writes `document.json` and `identity.json` (the artifact id, mission id,
terrain key and the document's SHA-256) into `<profile>/TBD_MissionArtifactCache/`. The mod loads
the cached document only when its bytes hash to the recorded SHA-256.

## Boundaries

- Depends on: `curl` on the path; the `anyhow`, `serde_json` and `sha2` crates.
- Used by: `tools_v2/xtask/src/commands/mod_ops/playtest_server/platform_deployment.rs`,
  `tools_v2/xtask/src/commands/mod_ops/world_boot.rs` and its `compiled_lane.rs`,
  `tools_v2/xtask/src/commands/mod_ops/mission_test.rs` and
  `tools_v2/xtask/src/commands/mod_ops/game_runtime_api_smoke.rs`.
- Rules: the flows are tested against a scripted transport, with no network
  (`tests/flows.rs`); a fetched artifact document must hash to its entity tag
  (`artifact_document_must_hash_to_its_entity_tag`); a refused submission names its code
  (`a_refused_submission_names_its_code`); URL components are percent-encoded except unreserved
  bytes (`components_are_percent_encoded_except_unreserved_bytes` in `tests/transport_parsing.rs`).

## Related documentation

- [Local development](/documentation_v2/runbooks/local_development.md) — running the database and
  the API these flows need.
