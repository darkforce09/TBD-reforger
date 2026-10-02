**Status:** live

# Verification completeness

Design for the verification requirements that prove the website [API](/documentation/glossary/a_to_f.md#api)
as a whole rather than one domain: `verification_route_acceptance`, `verification_contract_parity`,
`verification_property_invariants`, `verification_controlled_races`,
`verification_failure_injection` and `verification_engineering_laws`, with the `repository_quality`
replay `cargo xtask ci ci-local` beside them. It states the semantics the suites implement;
acceptance evidence is the command output recorded in
[progress_checkpoint.md](progress_checkpoint.md).

Every `identity_*` and `administration_*` check stands at its minimum with every named case, so
this work adds no [identity and access](/documentation/glossary/g_to_m.md#identity-and-access)
or [administration](/documentation/glossary/a_to_f.md#administration) scope.

Source paths below are relative to `apps/api/src/`, test paths to `apps/api/`.
Every refusal a route handler answers carries the envelope `{error, details?}`.

## Route acceptance

Every route the API serves has one executable specification, and every specification is held
against the route table the source declares at test time.

### Route table

The table is read from source when the tests run, never kept by hand
(`tests/route_acceptance_support/route_table.rs`):

- `core/http_router.rs` gives the top-level routes (`/healthz`, `/metrics`, `/uploads`,
  `/map-assets*`) and `api_v1_routes`, which merges with the `/api/v1` nest.
- Every `<domain>/routes.rs` gives its `pub fn routes(` body. Each
  `.merge(handlers::<x>::routes())` is followed into `handlers/<x>/mod.rs` or `handlers/<x>.rs`,
  and its `.nest(prefix, …)` applies to the rows it yields.
- A row is (method, full path, handler fn, development-only, route body limit). A row is
  development-only exactly when an enclosing `if` tests the configuration's development flag; a
  registration under any other condition is an error, never a skipped row.
- Cross-check: the parsed set equals the set of column-0 `/// @route METHOD PATH` tags bound to
  handler fns in `src/`, in both directions.

The twelve `/api/v1/debug/equipment-data/*` routes of the equipment data viewer are registered in
`community_content/routes.rs` only under a development configuration, as `/auth/dev-login` is.
Their `DevelopmentOnly` specifications live in `specs/administration_center_content.rs`, and
`tests/debug_routes_are_development_only.rs` proves a production router's 404 and a development
router's registration for every `@route GET /api/v1/debug/…` tag in `src/`.

### Route specifications

A `RouteSpec` per row lives in `tests/route_acceptance_support/specs/<part>.rs`. It names the
route's access class — `Public`, `DevelopmentOnly`, `Authenticated`, `Role(min)`,
`Machine(executor)` or `Observability` — and states each of the seven dimensions either as probes
or as `NotApplicable("<reason>")`.

| # | Dimension | Probe | Expected answer |
|---|---|---|---|
| 1 | authorized | the least-privileged allowed actor, the request built from the world | the documented success status and its contract (see [success contracts](#success-contracts)); a JSON body validates through the contract parity check |
| 2 | unauthorized | anonymous; one [role](/documentation/glossary/n_to_z.md#role) rank below the minimum; machine routes: no, invalid or revoked credential; machine routes: wrong executor or server; observability routes: a wrong token | 401; 403; 401; 403; 401 |
| 3 | ownership | a non-owner of equal role; the owner; an admin where the handler allows an override | 403 (or a documented 404); success; success |
| 4 | guest | a guest-role token | 403 on role-gated routes, success on `Authenticated` routes |
| 5 | ban | the account banned after its token is issued | 401 on every protected route |
| 6 | malformed | a non-UUID path id; invalid JSON; a missing or invalid content type; an unknown field on a `deny_unknown_fields` body; an invalid query | 400; 400; 415; 400; 400 |
| 7 | boundary | at least one per route: a body over the route limit, paging clamps and limits, maximum-length strings, a nonexistent id | 413 `request_too_large`; the documented clamp or limit; the documented length answer; 404 |

### Success contracts

A specification's success is a status and one contract:

| Contract | The authorized answer |
|---|---|
| `Schema(file, definition)` | a JSON body shaped by `file#/definitions/<definition>`, or by the root of `file` |
| `SchemaItems(file, definition)` | a top-level JSON array whose every element is shaped by `file#/definitions/<definition>` |
| `EventStream(file, definition)` | a `text/event-stream` whose first frame's `data` is shaped by the definition |
| `Binary(content type)` | a non-JSON body whose `Content-Type` starts with the value |
| `NoBody` | no body, as on 204 and redirects |
| `RefusalEnvelope(error)` | exactly `{"error": error}`, no `details` |

`RefusalEnvelope` comes only from `RouteSpec::refusal_only(status, error, reason)`, for a route
that refuses every well-formed request by documented intent. Its `reason` names the module,
relative to `src/`, whose `@route` handler documents the refusal, and the coverage binary checks
that module. `PATCH /api/v1/admin/users/{discordId}` is the one such route: Discord owns every
website role, so `update_user` in `administration/handlers/role_management.rs` validates the body
and refuses every valid role change with 409.

### Refusal envelopes

The malformed and boundary dimensions hold one API-wide rule: no extractor answers axum's
plain-text rejection body.

- Path segments: every handler takes `core::http::path_parameters::PathParams` in place of axum's
  `Path`; a segment that does not decode answers 400 `invalid path parameter: <reason>`
  (`ApiError::from_path_rejection`), and an extractor that does not match its route answers a
  logged 500, never a 400 that blames the caller.
- JSON bodies (`ApiError::from_json_rejection`): over the limit 413 with
  `details.code = request_too_large`, a missing or non-JSON `Content-Type` 415, anything else 400.
- Query strings (`ApiError::from_query_rejection`): 400 `invalid <query name>: <reason>`.

`tests/json_rejection_envelopes.rs` proves the JSON rule on every handler that decodes its body
before it reads a stored row (415 without a content type, 413 one byte over the limit), and
`tests/query_rejection_envelopes.rs` the query rule on every typed query, the equipment viewer's
included; the derived probes of the part binaries cover the rest.

### Derived probes

The framework derives from the route row and the access class every probe that needs no domain
knowledge: anonymous 401, rank-below 403, guest, ban 401, the machine
[credential](/documentation/glossary/g_to_m.md#machine-credential) refusals, observability
401, non-UUID path id 400, invalid JSON 400 and missing content type 415 on body routes, over-limit
body 413 and nonexistent path id 404.

A part specification supplies only what cannot be derived: the world setup, which returns the
path parameters and a valid body; the success status and contract; the ownership probe; the extra
malformed probes (unknown field, bad query) and the domain boundary probes (paging, clamps,
lengths); the `NotApplicable` reasons; and a per-route override wherever the real, documented
behaviour differs from a derived default.

Every probe carries a fresh `ConnectInfo` peer, so the per-peer rate limiter, which answers 429
from the fortieth request of one peer, never answers a probe.

### Coverage

The `route_acceptance_coverage` binary holds the table and the specifications together:

- every parsed route has exactly one specification, and every specification matches a route;
- every specification declares all seven dimensions;
- every `NotApplicable` has a non-empty reason;
- every JSON success names a contract, and every refusal-only specification names its
  documenting module;
- development-only rows carry the `DevelopmentOnly` access class;
- the route table reader and the round-trip comparison below pass their self-tests.

### Part binaries

Each part runs as `tests/route_acceptance_<part>.rs`: `identity_and_core`, `operations_events`,
`operations_reservations`, `operations_ballistics`, `missions_library`, `missions_reviews`,
`fleet_and_telemetry` and `administration_center_content`. Each has one test fn per dimension named
`route_acceptance_<part>_<dimension>_…`, plus `contract_parity_<part>_responses_match_their_contracts`:
every authorized JSON response validates against its schema definition, decodes into the
generated type where one exists and re-serialises to an equal value, and every request body an
authorized probe sends validates against its request definition where the schema has one.

The re-serialised value compares exactly, with one rule for instants: a string the schema declares
`format: date-time` compares as the instant it names, and the live string must also be the API's
own spelling of that instant (`core::wire_format::rfc3339_utc`: UTC, `Z`, trailing fractional
zeros trimmed), so the comparison never relaxes the wire format
(`tests/route_acceptance_support/round_trip_comparison.rs`).

## Contract parity

Every shape the API answers is pinned three ways: by a golden the seeded API reproduces, by a
schema definition, and by the consumers that decode it.

### Golden reproduction

The `contract_parity_goldens` binary replays the frontend goldens in
`contracts/fixtures/api_goldens/` against a live router:

1. Mint the [dev login](/documentation/glossary/a_to_f.md#dev-login) admin token, then apply
   `seeds/registry_dev.sql` and `seeds/content_golden.sql` to the binary's database, in the
   recipe order `content_golden.sql` records at its end.
2. Boot the router with the recipe configuration.
3. For every `_index.tsv` row, send its method and path; writes carry their stored request bodies
   and run in recipe order.
4. The status equals the index status, and the JSON equals the golden as a `serde_json::Value`
   (key order irrelevant). A `.sse.txt` golden equals the live stream's leading frames.
5. Every golden validates against its route contract and decodes into the generated type where
   one exists.

The seeded API reproduces every golden byte for byte, except at the fields where a write answers
with a value the server generates per request: an id, a secret, or a time stamped at the request.
Each such field is a row of the normalisation table below (route, field, reason), committed as
`apps/api/tests/contract_parity_support/normalised_fields.rs`. At such a field the
live value must exist, be a string and match its kind's format, and the golden stores the kind's
fixed placeholder; every other byte compares exactly. The index case fails on a row whose golden
does not hold that placeholder at that field (a dead or unknown row), and on a placeholder a
golden holds where no row names it. A golden never holds a real secret.

| Kind | The live value must be | The golden stores |
|---|---|---|
| `server_uuid` | a lowercase, hyphenated version 4 UUID | `99999999-9999-4999-9999-999999999999` |
| `request_time` | an RFC 3339 instant inside the capture window, give or take 1 s | `2000-01-01T00:00:00Z` |
| `request_time + N s` | an RFC 3339 instant N seconds after a `request_time` | `2000-01-01T00:00:00Z` |
| `access_token_jwt` | three non-empty base64url segments | `normalised.access.token` |
| `opaque_token_hex64` | 64 lowercase hexadecimal digits | 64 zeros |
| `link_code_six_digits` | six decimal digits | `000000` |
| `machine_credential_secret` | `tbdm_`, the 32 hex digits of a version 4 UUID, `_`, 64 hex digits | `tbdm_99999999999949999999999999999999_` and 64 zeros |

The capture window runs from just before the first indexed request to just after the last answer.

#### Normalisation table

| Route | Field | Reason |
|---|---|---|
| `POST /api/v1/auth/refresh` | `/access_token` | `access_token_jwt`: a signed token minted per request |
| `POST /api/v1/auth/refresh` | `/expires_at` | `request_time + 900 s`: an expiry or deadline counted from the request |
| `POST /api/v1/auth/refresh` | `/refresh_token` | `opaque_token_hex64`: a random refresh token |
| `POST /api/v1/me/link` | `/code` | `link_code_six_digits`: a random one-time code |
| `POST /api/v1/me/link` | `/expires_at` | `request_time + 600 s`: an expiry or deadline counted from the request |
| `POST /api/v1/modpacks` | `/id` | `server_uuid`: an id the server generates for the created row |
| `POST /api/v1/modpacks` | `/created_at` | `request_time`: stamped when the request runs |
| `POST /api/v1/modpacks` | `/mods/0/id` | `server_uuid`: an id the server generates for the created row |
| `POST /api/v1/modpacks` | `/mods/0/modpack_id` | `server_uuid`: an id the server generates for the created row |
| `POST /api/v1/vehicle-database` | `/id` | `server_uuid`: an id the server generates for the created row |
| `PUT /api/v1/wiki/night-operations` | `/id` | `server_uuid`: an id the server generates for the created row |
| `PUT /api/v1/wiki/night-operations` | `/updated_at` | `request_time`: stamped when the request runs |
| `POST /api/v1/approvals/00000000-0000-4000-c000-000000000004/reject` | `/reviewed_at` | `request_time`: stamped when the request runs |
| `POST /api/v1/approvals/00000000-0000-4000-c000-000000000004/reject` | `/updated_at` | `request_time`: stamped when the request runs |
| `POST /api/v1/missions/00000000-0000-4000-c000-000000000004/submit` | `/updated_at` | `request_time`: stamped when the request runs |
| `POST /api/v1/approvals/00000000-0000-4000-c000-000000000004/approve` | `/reviewed_at` | `request_time`: stamped when the request runs |
| `POST /api/v1/approvals/00000000-0000-4000-c000-000000000004/approve` | `/updated_at` | `request_time`: stamped when the request runs |
| `POST /api/v1/missions/00000000-0000-4000-c000-000000000004/review-comments` | `/id` | `server_uuid`: an id the server generates for the created row |
| `POST /api/v1/missions/00000000-0000-4000-c000-000000000004/review-comments` | `/created_at` | `request_time`: stamped when the request runs |
| `POST /api/v1/servers/00000000-0000-4000-d000-000000000001/deployments` | `/id` | `server_uuid`: an id the server generates for the created row |
| `POST /api/v1/servers/00000000-0000-4000-d000-000000000001/deployments` | `/fleet_command_id` | `server_uuid`: an id the server generates for the created row |
| `POST /api/v1/servers/00000000-0000-4000-d000-000000000001/deployments` | `/requested_at` | `request_time`: stamped when the request runs |
| `POST /api/v1/servers/00000000-0000-4000-d000-000000000001/deployments` | `/deadline_at` | `request_time + 1200 s`: an expiry or deadline counted from the request |
| `POST /api/v1/servers/00000000-0000-4000-d000-000000000002/deployments/00000000-0000-4000-f400-000000000003/cancel` | `/finished_at` | `request_time`: stamped when the request runs |
| `POST /api/v1/factions` | `/id` | `server_uuid`: an id the server generates for the created row |
| `POST /api/v1/factions` | `/created_at` | `request_time`: stamped when the request runs |
| `POST /api/v1/factions` | `/updated_at` | `request_time`: stamped when the request runs |
| `PUT /api/v1/factions/00000000-0000-4000-b100-000000000003` | `/updated_at` | `request_time`: stamped when the request runs |
| `POST /api/v1/fire-missions` | `/fire_mission/id` | `server_uuid`: an id the server generates for the created row |
| `POST /api/v1/fire-missions` | `/fire_mission/created_at` | `request_time`: stamped when the request runs |
| `GET /api/v1/ballistics-catalogs` | `/data/0/uploaded_at` | `request_time`: stamped when the capture, as the administrator, uploads the committed vanilla catalog pair through `POST /api/v1/ballistics-catalogs` just before the first row of that path, so every earlier read answers over the seeds alone |
| `POST /api/v1/events/c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7/groups` | `/access/groups/1/id` | `server_uuid`: an id the server generates for the created row |
| `POST /api/v1/events/c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7/groups` | `/access/groups/1/provenance/created_at` | `request_time`: stamped when the request runs |
| `POST /api/v1/me/leave-requests` | `/id` | `server_uuid`: an id the server generates for the created row |
| `POST /api/v1/me/leave-requests` | `/created_at` | `request_time`: stamped when the request runs |
| `POST /api/v1/servers/00000000-0000-4000-d000-000000000001/commands` | `/id` | `server_uuid`: an id the server generates for the created row |
| `POST /api/v1/servers/00000000-0000-4000-d000-000000000001/commands` | `/requested_at` | `request_time`: stamped when the request runs |
| `POST /api/v1/servers/00000000-0000-4000-d000-000000000001/commands` | `/expires_at` | `request_time + 300 s`: an expiry or deadline counted from the request |
| `POST /api/v1/servers/00000000-0000-4000-d000-000000000001/commands/00000000-0000-4000-f200-000000000003/cancel` | `/finished_at` | `request_time`: stamped when the request runs |
| `POST /api/v1/servers/00000000-0000-4000-d000-000000000001/credentials` | `/credential/id` | `server_uuid`: an id the server generates for the created row |
| `POST /api/v1/servers/00000000-0000-4000-d000-000000000001/credentials` | `/credential/created_at` | `request_time`: stamped when the request runs |
| `POST /api/v1/servers/00000000-0000-4000-d000-000000000001/credentials` | `/secret` | `machine_credential_secret`: a random secret shown once |
| `DELETE /api/v1/servers/00000000-0000-4000-d000-000000000002/credentials/00000000-0000-4000-e000-000000000004?reason=Host%20retired` | `/revoked_at` | `request_time`: stamped when the request runs |
| `PUT /api/v1/fleet/scenarios/everon` | `/updated_at` | `request_time`: stamped when the request runs |
| `POST /api/v1/servers` | `/id` | `server_uuid`: an id the server generates for the created row |

### Equipment data viewer goldens

The `contract_parity_equipment_viewer` binary reproduces every answer of the twelve
development-only equipment routes from a committed export rather than a local dataset. The
production importer (`community_content::services::equipment_data_viewer`) imports
`tests/fixtures/equipment_data_viewer/diagnostic_export/` and the gameplay publication under
`tests/fixtures/equipment_data_viewer/export_source/` into a temporary data directory, and a
development router answers one request per route. Each JSON answer equals its golden as a
`serde_json::Value` with nothing normalised, validates against its route's schema and decodes into
the generated type; the download equals the committed document byte for byte. The goldens are
the six `positive/` pages of `contracts/fixtures/equipment-data-viewer/` and the answers under
`tests/fixtures/equipment_data_viewer/route_responses/`, and a second case requires every positive
page and every route to have one. The committed publications carry their own `bytes` and `sha256`
manifest and pointer digest, so an edited document must update both, or the importer refuses it.

### Response schemas

Every JSON success response has a schema definition in `contracts/definitions/`; the coverage
binary refuses a JSON success whose specification names no contract.

`contract_parity_registry_row_constraints_match_the_catalogue_schemas`, in
`contract_parity_goldens`, holds the stored registry row definitions of
`arsenal-envelopes.schema.json` to every constraint of the catalogue definitions they copy
(`registry-items.schema.json#/$defs/item`, `registry-compat.schema.json#/$defs/edge`): the `kind`
and `edge_type` vocabularies, patterns, minimums and required names. A row may add keywords and
storage columns but never drops or changes a copied constraint
(`tests/contract_parity_support/catalogue_row_constraints.rs`).

### Frontend chain

Every route the frontend calls whose response decodes into a typed DTO in
`apps/frontend/src/v2/core/api/dto/` has a golden that the seeded API reproduces (the
golden reproduction above) and that the DTO round-trips, with every wire key either claimed by a
field or listed as deliberately unclaimed; the frontend `r_api` tests hold the round trip. The
modpacks, announcements, CMS announcements and leaderboards responses decode into typed DTOs, not
`Value`. New golden tests live in the existing `dto/tests/r_api_*.rs` submodules and new DTO
types in the existing DTO modules.

### Mod wire structs

The `contract_parity_mod_wire` binary reads the [mod](/documentation/glossary/g_to_m.md#mod)
scripts:

- every EnfScript class carrying `//! @contract` has field names, and `//!<` JSON key bindings,
  that are properties of the cited definition, and carries every required property unless its tag
  says `partial`;
- every body builder emits only keys of its cited definition, and every constant group holds only
  values of its cited schema enum;
- `TBD_RosterLoader.c` `WIRE_VERSION` equals the schema's const and the `version` the backend emits;
- the mod's accepted mission `schemaVersion` window contains the mission compiler's version;
- every mod JSON DTO that crosses the API boundary cites a contract.

Tags follow the closed grammar `@contract <schema>#<pointer>[ (<sub-path>)][ partial]` that the
[documentation standards](/documentation/standards/documentation_standards.md#5-rust-comments)
state and `tests/enfscript_source_support/contract_tag.rs` parses; any other shape fails the case.

## Property invariants

Each property runs through `common::property_evidence::run_property(id, cases, &strategy, |v| …)`
with 256 cases; the test fn is named by the property id, and the recorder prints the
`property-run: {…}` line that [property_test_evidence.md](property_test_evidence.md) describes. A
database-backed property builds its own runtime and blocks on it per case. Property ids are unique
across every binary; `cargo xtask db test-it` sets `PROPTEST_RNG_SEED` (default 2026092201).

| | Property id and binary | Generated input | Invariant |
|---|---|---|---|
| (a) | `protected_actions_require_effective_session_authority`, `tests/session_authority_properties.rs` | account and session states (role, banned, deleted, revoked, expired, development session under production configuration, membership fresh, stale, grace or override, confirmed nonmember) × required rank | the production `authorize_session` and role gate allow iff the session is live and the effective role rank is at least the minimum; refusals are 401 or 403 exactly as the oracle says |
| (b) | `refresh_replay_revokes_concurrently_issued_successor`, `tests/session_authority_properties.rs` | rotate, replay and logout sequences with concurrent pairs | at most one live successor per family; replaying a consumed token revokes the family, including concurrently issued successors, whose access tokens answer 401 |
| (c) | `approval_and_deployment_share_immutable_artifact`, `tests/mission_artifact_properties.rs` | submit, approve, reject, resubmit, request-deployment and confirm sequences | every [mission deployment](/documentation/glossary/g_to_m.md#mission-deployment) references the approved [artifact](/documentation/glossary/a_to_f.md#artifact) id and sha256; artifact rows never change; a stale approval never deploys a newer artifact |
| (d) | `telemetry_revisions_contribute_exactly_once`, `tests/telemetry_revision_properties.rs` | revision streams with duplicates, reorders, digest conflicts and finalisation | aggregates equal the oracle of the highest applied revision; duplicates are inert; conflicts answer 409; a finalized match stays finalized |
| (e) | `command_executor_fencing_preserves_observed_outcomes`, `tests/fleet_command_properties.rs` | claim, executing, result, reconcile and stale-executor interleavings | each observed outcome of a [fleet command](/documentation/glossary/a_to_f.md#fleet-command) is recorded once; stale fencing tokens are refused; an uncertain command is never re-executed |
| (f) | `audit_publication_preserves_committed_event_delivery`, `tests/audit_publication_properties.rs` | [audit](/documentation/glossary/a_to_f.md#audit-logs) appends, some rolled back, with generated commit orders and batch sizes | every committed row is published exactly once with contiguous sequences; a rolled-back row never is; delivery yields sequence order |
| (g) | `reservation_transactions_conserve_slots_and_participants`, `tests/reservation_transaction_properties.rs` | claim, release, assign, withdraw and promote operations through the production transactions | occupied seats never exceed capacity; at most one seat per participant per attached mission of an [event](/documentation/glossary/a_to_f.md#event); counts are conserved |

Invariant (g) is per attached mission because a claim releases the participant's other seats of
the same mission only (`release_other_seats` in
`operations/services/event_reservations/seat_claims.rs`); a single-mission event is the same.

### Register fix

The property list of `verification_property_invariants` names
`reservation_transactions_conserve_slots_and_participants`, but its case pattern does not, so a
run never counts it. The fix adds the id to the check's case pattern and raises its minimum from
25 to 26.

## Failpoints

Failpoints live in `core/failpoints/`. They exist only under the Cargo feature `failpoints`,
which is not a default feature and which only the `api` self dev-dependency enables. The
release and deploy build, `cargo build --release -p api --bin api`, compiles them out:
the macro expands to `Ok(())`-equivalent code with no registry, no names and no state. An unarmed
failpoint is inert.

With the feature on, `arm(Failpoint, FailAction) -> ArmGuard` arms one failpoint in the
process-global registry until the guard drops; the actions are `Fail`, `FailOnce` and
`Pause(PauseHandle)`, whose handle offers `reached().await` and `release()`. Only the
`failure_injection*` and `controlled_races*` binaries arm failpoints, and each case takes the
suite lock (`lock_suite`) first, so no two armed cases run at once.

### Catalogue

The enum `Failpoint` holds the catalogue. "AfterCommit" means the transaction committed and the
response is lost: the handler answers 500.

| Failpoint | Boundary | Placement |
|---|---|---|
| `SessionRotationBeforeCommit` | refresh-token rotation, before its transaction commits | `identity_and_access/services/session_rotation.rs::rotate_session` |
| `SessionRotationAfterCommit` | refresh-token rotation, after commit, response lost | `identity_and_access/services/session_rotation.rs::rotate_session` |
| `SessionLogoutBeforeCommit` | logout, before its transaction commits | `identity_and_access/services/session_rotation.rs::logout_session` |
| `ReservationClaimBeforeCommit` | reservation claim, before commit | `operations/handlers/slot_registration.rs::register_for_event_mission` |
| `ReservationClaimAfterCommit` | reservation claim, after commit, response lost | `operations/handlers/slot_registration.rs::register_for_event_mission` |
| `ReviewDecisionBeforeCommit` | mission review decision, before commit | `missions/services/mission_reviews.rs::decide_review` |
| `DeploymentRequestBeforeCommit` | mission deployment request, before commit | `missions/services/mission_deployments/deployment_requests.rs::request_deployment` |
| `DeploymentRequestAfterCommit` | mission deployment request, after commit, response lost | `missions/handlers/mission_deployments.rs::request_server_deployment`, `missions/handlers/game_runtime_missions.rs::relayed_deployment_request` |
| `ResultsRevisionBeforeCommit` | match results revision, before commit | `match_telemetry/services/match_results_ingest.rs::ingest_results_revision` |
| `ResultsRevisionAfterCommit` | match results revision, after commit, response lost | `match_telemetry/services/match_results_ingest.rs::ingest_results_revision` |
| `FleetCommandClaimAfterCommit` | fleet command claim, after commit, response lost | `server_infrastructure/handlers/fleet_executor.rs::claim_fleet_command` |
| `FleetCommandResultBeforeCommit` | fleet command result, before commit | `server_infrastructure/services/fleet_commands/executor_claims.rs::record_result` |
| `FleetCommandResultAfterCommit` | fleet command result, after commit, response lost | `server_infrastructure/handlers/fleet_executor.rs::finish_fleet_command` |
| `AuditPublicationBeforeCommit` | audit publication, before commit | `administration/services/audit_publication.rs::publish_audit_batch` |
| `AuditDeliveryRead` | audit stream delivery read, during reconnect | `administration/services/audit_delivery.rs::read_window` |
| `DiscordRoleSyncBeforeEffect` | Discord role sync, before the external effect | `identity_and_access/services/discord_rest_reconciliation.rs::reconcile_one` |
| `DiscordRoleSyncAfterEffect` | Discord role sync, after the external effect | `identity_and_access/services/discord_rest_reconciliation.rs::reconcile_one` |
| `IdentityLinkConfirmBeforeCommit` | Arma identity link confirmation, before commit | `identity_and_access/services/identity_linking.rs::confirm_identity` |

The placement column names the function that hosts each failpoint, by its path under
`apps/api/src/`.

## Controlled races

Each race runs in both interleavings, so either contender can win.

| Race | Contenders | Case (binary) |
|---|---|---|
| last seat | two claimants, one seat | `controlled_races_last_seat_has_one_winner_in_both_orders` (`controlled_races_reservations`) |
| assignment vs withdrawal | an assignment and a withdrawal | `controlled_races_assignment_and_withdrawal_serialise_in_both_orders` (`controlled_races_reservations`) |
| refresh winner | two rotations of one token | `controlled_races_refresh_rotation_has_one_winner_and_the_replay_revokes_its_successor` (`controlled_races_identity`) |
| replay | a spent token replayed after its successor is used | `controlled_races_replay_after_successor_use_revokes_the_family_in_both_orders` (`controlled_races_identity`) |
| linking | two accounts, one Arma identity | `controlled_races_linking_one_identity_has_one_holder_in_both_orders` (`controlled_races_identity`) |
| approval | approve and reject of one review | `controlled_races_approval_and_rejection_of_one_review_decide_it_once` (`controlled_races_missions_and_telemetry`) |
| ingest | duplicate and corrected results revisions | `controlled_races_duplicate_identical_revisions_apply_once`, `controlled_races_corrected_revisions_in_either_order_end_at_the_newer_revision` (`controlled_races_missions_and_telemetry`) |
| commit ordering | audit rows committed in inverted id order | `controlled_races_audit_inverted_commit_order_publishes_and_delivers_every_row_once` (`controlled_races_audit`) |

Ordering never rests on timing. It comes from:

- a failpoint `Pause`, which holds one contender at a named boundary until the test releases it;
- held row locks, with `pg_blocking_pids` polled until the second contender blocks, then released
  in order through a oneshot (`tests/failpoint_and_race_support/row_lock_barrier.rs`, the pattern
  of `tests/reservation_guard_support`);
- `tokio::sync::Barrier`, which starts contenders together.

The invariants are checked on every observed result and on the final persisted state.

Two races are serialised by locks a red-proving perturbation must target:

- Refresh rotation: `rotate_session` (`identity_and_access/services/session_rotation.rs`) takes the
  account lock (`lock_account`, `SELECT … FROM users … FOR UPDATE` in
  `identity_and_access/services/account_authority.rs`) before it reads the token row, so every
  rotation of one account queues there. Removing the refresh-token row lock alone leaves the race
  green; removing `lock_account` from `rotate_session` is the defect that turns it red.
- Review decision: an approval and a rejection are serialised twice, by the mission row lock that
  `lock_pending` in `missions/handlers/approvals_queue.rs` takes (`FOR NO KEY UPDATE`) and by the
  pending review row lock in `missions/services/mission_reviews.rs`. Either alone keeps the case
  green, so a perturbation removes the mission row lock.

## Failure injection

For each catalogue failpoint the `failure_injection_<area>` binaries (`identity`, `operations`,
`missions`, `telemetry`, `fleet`, `audit`, `discord`) assert the outcome its boundary promises:

| Boundary | Outcome |
|---|---|
| before commit | rollback: no business row, no audit row, no pending publication; a clean retry succeeds |
| after commit | the retry is idempotent, or explicitly indeterminate as documented |
| before an external effect | no effect |
| after an external effect | exactly one effect after recovery |
| during reconnect | the stream resumes from the cursor without loss or duplicate |

The documented indeterminate outcome is refresh rotation: a lost rotation pair makes the retry a
replay of the consumed token, which revokes the whole family.

`failure_injection_self_checks` proves the support the failure and race suites stand on,
`failure_injection_failpoints_are_inert_until_armed` among its cases: an unarmed failpoint changes
nothing, and the arming helpers fail, fail once and pause exactly as named.

## Engineering laws

`tests/engineering_laws.rs` checks the repository laws through `verification_core::repository_laws`,
which shares its roots and rules with `cargo xtask verify file-length` and
`cargo xtask verify engine-layers`:

- production files stay at or under 500 lines, test files at or under 1000;
- the law-7 rules (size and test placement) have no exemption mechanism: no allowlist files, no
  allowlist comments and no grandfather tables; the frontend `doc_audit` grandfather table is
  gone;
- unit tests live only in sibling files; no inline test-module body exists;
- `graphics_engine` knows no map concept and has no `map_engine` dependency,
  `map_engine` has no UI framework dependency anywhere in the crate, and
  `frontend` does not depend on `graphics_engine`;
- `api` depends on neither `graphics_engine` nor `frontend`; its one
  engine dependency is `map_engine` with the mission tier alone;
- the engine-layer walls of the
  [engine boundary rules](/documentation/standards/engine_boundary_rules.md) and the crate
  directions hold;
- the `failpoints` feature is test-only: not a default feature, enabled only by the self
  dev-dependency, and the deploy build passes no feature flags.

## Readiness fingerprint

`cargo xtask verify api-readiness` fingerprints a tracked symlink by its link text (the content of
its git mode 120000 blob) instead of refusing it. An untracked symlink, a symlink that resolves
outside the repository, a dangling one, and any evidence-storage path through a symlink stay
refused. Red-first self-tests in the `api_readiness` suite hold the rule.

## Issue triage

Findings follow the operator rule. FIX NOW: the finding turns a verification or register check
red, is a fail-open test or gate on this surface, or is small (about 50 lines or fewer) in a file
the fixer owns, with a clear fix and a red-first test. NOTE: it needs an operator decision, is a
feature, UX change or refactor, sits in a file another change owns, or is large; it is recorded
for a [ticket](/documentation/glossary/n_to_z.md#ticket) (`cargo xtask ticket add`) or a
[known bug](/documentation/known_bugs/README.md) entry, and never leaves a verification check
red. CLOSE: an open ticket the code already fixes, closed with the test name or `file:line`. The
[Milestone V findings](verification_findings.md) table records every finding.
T-1041 (remove the doc-audit grandfather allowlist) closes with the removal of the frontend
`doc_audit` grandfather table, held by `engineering_laws_no_exemption_mechanism_exists`.

## Gates and evidence

The work is accepted when every gate below passes and its output is recorded in the checkpoint:

- `cargo xtask db test-it`: every binary, with a runtime target of 900 s for the whole run (the
  register timeout is 1800 s); a property binary stays at or under 60 s and any binary at or under
  120 s. It sets `TBD_API_VERIFICATION=true`, and the readiness check counts (executable, test
  name) pairs whose fn name starts with the check's prefix.
- `cargo test -p api --lib`.
- `cargo clippy -p api --all-targets -- -D warnings` (failpoints on) and
  `cargo clippy -p api --lib --bins -- -D warnings` (failpoints off).
- `cargo build --release -p api --bin api`, which compiles failpoints out.
- The frontend `r_api` tests, `cargo xtask verify api-readiness`, `cargo xtask ci ci-local` (the
  `repository_quality` replay) and `cargo xtask ci verify-documentation`.

A new suite runs twice before it is accepted, so a flaky case shows. Every new check carries a
perturbation proof, recorded in the checkpoint: a deliberate defect in the code under test, the
red case names it produces, and the restoration proved by `git diff` or an equal sha256.

## Tests

Each requirement's check runs its command and counts the passing cases its pattern names.

| Requirement | Suites | Cases |
|---|---|---|
| `verification_route_acceptance` | `tests/route_acceptance_coverage.rs`, the seven `tests/route_acceptance_<part>.rs` part binaries and `tests/debug_routes_are_development_only.rs` | `route_acceptance_*` |
| `verification_contract_parity` | `tests/contract_parity_goldens.rs`, `tests/contract_parity_equipment_viewer.rs`, `tests/contract_parity_mod_wire.rs`, `tests/json_rejection_envelopes.rs`, `tests/query_rejection_envelopes.rs` and the part binaries | `contract_parity_*` |
| `verification_property_invariants` | `tests/session_authority_properties.rs`, `tests/mission_artifact_properties.rs`, `tests/telemetry_revision_properties.rs`, `tests/fleet_command_properties.rs`, `tests/audit_publication_properties.rs`, `tests/reservation_transaction_properties.rs` and the earlier property suites | the property ids, (a)–(g) above among them |
| `verification_controlled_races` | `tests/controlled_races_identity.rs`, `tests/controlled_races_reservations.rs`, `tests/controlled_races_missions_and_telemetry.rs`, `tests/controlled_races_audit.rs` | `controlled_races_*` |
| `verification_failure_injection` | `tests/failure_injection_<area>.rs` for the seven areas and `tests/failure_injection_self_checks.rs` | `failure_injection_*` |
| `verification_engineering_laws` | `tests/engineering_laws.rs` | `engineering_laws_*` |
| `repository_quality` | `cargo xtask ci ci-local` | every passing test line |

The [API README](/apps/api/README.md#verification-suites) maps each group to its
binaries and its shared `tests/*_support/` folders.
