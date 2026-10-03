# Typed endpoint calls

One typed call per [API](/documentation/glossary/a_to_f.md#api) route for
[event](/documentation/glossary/a_to_f.md#event) access and registration,
[machine credentials](/documentation/glossary/g_to_m.md#machine-credential),
[mission](/documentation/glossary/g_to_m.md#mission) reviews,
[fleet commands](/documentation/glossary/a_to_f.md#fleet-command),
[mission deployments](/documentation/glossary/g_to_m.md#mission-deployment),
[fleet scenarios](/documentation/glossary/a_to_f.md#fleet-scenario) and the game-server registry, each
naming its route's path, body and answer in one place: a page calls
`put_event_access_policy(store, event, &change)` instead of assembling a path, a body and a verb.

## Contents

```text
apps/frontend/src/foundation/transport/endpoints/
├── event_access_administration.rs  an event's access policies, reservation pools, groups and rosters
├── event_registration.rs           registering for a place in a mission, and promoting the waitlist
├── fleet_commands.rs               request, list, read and cancel a server's fleet commands
├── fleet_scenarios.rs              list, register or replace, and remove the fleet scenarios
├── machine_credentials.rs          list, issue and revoke a server's machine credentials
├── mission_deployments.rs          a server's mission deployments and the request form's choices
├── mission_reviews.rs              submission, review history and comments, artifacts, decisions
├── mod.rs                          the module tree, the percent-encoder and the JSON body helper
├── server_registry.rs              list, register, change and deactivate game servers; the modpack choices
└── tests/                          unit tests for the paths, the encoding and the registration body
```

## How it works

Each file has two halves. Its path builders are pure functions compiled into every build, and
each passes a segment or query value taken from data through `encode_path_segment`, which
percent-encodes everything but the RFC 3986 unreserved characters, since a squad name may hold a
space or a slash. Its calls sit in a `calls` module compiled for `wasm32` only and re-exported
from the file; each is one of the client's verbs, so it shares the bearer token, the single flight
and the single retry. A change answers an `ApiRefusal` on failure, because its callers branch on
the reason; a read answers the plain `(status, message)` pair. A `DELETE` of an access policy, a
group or a group member names the access revision it was prepared against in its query
(`with_expected_revision`), and a registration body names a seat, or with an empty `slot_id` asks
for a seatless place that joins the waitlist when none is free. A server registration and a
server change answer the server's row in the shape `GET /api/v1/servers` lists, and a
deactivation answers 204 with no body.

| File | Routes it calls |
|---|---|
| `event_access_administration.rs` | `GET /api/v1/events/{id}/access`, `GET /api/v1/events/{id}/access/participants`, `PUT /api/v1/events/{id}/access-policy`, `PUT` and `DELETE /api/v1/event-missions/{emid}/squads/{faction}/{squad}/access-policy`, `PUT` and `DELETE /api/v1/event-missions/{emid}/slots/{slotId}/access-policy`, `PUT /api/v1/events/{id}/reservation-quotas`, `POST /api/v1/events/{id}/groups`, `PATCH` and `DELETE /api/v1/events/{id}/groups/{groupId}`, `PUT` and `DELETE /api/v1/events/{id}/groups/{groupId}/members/{discordId}` |
| `event_registration.rs` | `POST /api/v1/event-missions/{emid}/register`, `POST /api/v1/event-missions/{emid}/waitlist/promote` |
| `fleet_commands.rs` | `GET` and `POST /api/v1/servers/{id}/commands`, `GET /api/v1/servers/{id}/commands/{commandId}`, `POST /api/v1/servers/{id}/commands/{commandId}/cancel` |
| `fleet_scenarios.rs` | `GET /api/v1/fleet/scenarios`, `PUT` and `DELETE /api/v1/fleet/scenarios/{terrainKey}` |
| `machine_credentials.rs` | `GET` and `POST /api/v1/servers/{id}/credentials`, `DELETE /api/v1/servers/{id}/credentials/{credentialId}` with the reason in its query |
| `mission_deployments.rs` | `GET` and `POST /api/v1/servers/{id}/deployments`, `GET /api/v1/servers/{id}/deployments/{deploymentId}`, `POST /api/v1/servers/{id}/deployments/{deploymentId}/cancel`, and for the request form `GET /api/v1/missions?limit=100`, `GET /api/v1/events?scope=upcoming&limit=100` and `GET /api/v1/events/{id}` |
| `mission_reviews.rs` | `POST /api/v1/missions/{id}/submit`, `GET /api/v1/missions/{id}/reviews`, `POST /api/v1/missions/{id}/review-comments`, `GET /api/v1/missions/{id}/artifacts/{artifact_id}`, `GET /api/v1/missions/{id}/artifacts/{artifact_id}/workspace`, `POST /api/v1/approvals/{id}/approve`, `POST /api/v1/approvals/{id}/reject` |
| `server_registry.rs` | `GET` and `POST /api/v1/servers`, `PATCH` and `DELETE /api/v1/servers/{id}`, and for the registration form `GET /api/v1/modpacks` |

## Boundaries

- Depends on: the verbs and `ApiRefusal` of `crate::foundation::transport::client`, the DTOs of
  `crate::foundation::transport::dto`, `crate::foundation::transport::token_provider::TokenProvider`
  (each call takes the session as `impl TokenProvider`), and `serde` and `serde_json` for the
  bodies.
- Used by: the pages under `apps/frontend/src/pages/`: the
  [approvals](/documentation/glossary/a_to_f.md#approvals) review drawer, the
  [event manager](/documentation/glossary/a_to_f.md#event-manager)'s access panel and game server
  choice, the [server control](/documentation/glossary/n_to_z.md#server-control) panels and server
  registry, the mission hub's submit
  action, review record and review workspace, and the event hub's registration and waitlist
  controls.
- Rules: every path a builder produces fits a route template of the API's route tables
  (`every_endpoint_path_lands_on_a_registered_route` in `tests/endpoints.rs`, which reads them
  through `crate::foundation::test_support::fixtures::api_route_source`), and the server registry's
  paths name its exact route templates
  (`server_registry_paths_name_the_registered_route_templates`); data in a path or a query
  is percent-encoded (`path_segments_and_query_values_are_percent_encoded`); a removal names the
  access revision it was prepared against (`a_removal_names_its_revision_in_the_query`).

## Related documentation

- [API overview](/documentation/apps/api/api_overview.md) — the routes of every API
  domain.
