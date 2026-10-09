# Operations handlers

The HTTP handlers of the [operations](/documentation/glossary/n_to_z.md#operations) domain, one module
per surface: the [event](/documentation/glossary/a_to_f.md#event) calendar and its hub, event writes and
[mission](/documentation/glossary/g_to_m.md#mission) attachments, access administration, the
[ORBAT](/documentation/glossary/n_to_z.md#orbat) and its [slots](/documentation/glossary/n_to_z.md#slot),
the member directory, the [service record](/documentation/glossary/n_to_z.md#service-record), leave
requests, fire missions, and the [game runtime](/documentation/glossary/g_to_m.md#game-runtime)'s roster
and player [deployments](/documentation/glossary/a_to_f.md#deployment).

## Contents

```text
crates/api/api_operations/src/handlers/
├── ballistics_catalogs/            the game ballistics catalog upload and its public reads
├── event_access_administration.rs  an event's access view and evidence; its policies and pools
├── event_create_update.rs          create, update and delete an event under the event-scope locks
├── event_group_administration.rs   an event's roster and partner-guild groups and their members
├── event_hub.rs                    one event's dossier, projected for what the viewer may see
├── event_listing.rs                the calendar list, filtered to the events the viewer may see
├── event_mission_attachment.rs     attach a mission with a snapshot of its ORBAT, and detach it
├── fire_missions/                  re-solved fire-mission saves and an event's saved fire missions
├── game_runtime_deployments.rs     authorize and end player lives for a game runtime
├── game_runtime_roster.rs          the event roster a game runtime seats players from
├── leave_requests.rs               file and read the caller's leave requests; administrator review
├── member_service_record.rs        the caller's service record: combat figures and deployments
├── mod.rs                          the module tree
├── orbat_view.rs                   an event mission's ORBAT, and the member directory for seating
├── slot_assignment.rs              leaders seat and clear members and hold or release whole squads
├── slot_registration.rs            a member reserves a seat or a place, or withdraws
├── tests/                          unit tests for event writes, attachment, the roster and paging
└── waitlist_promotion.rs           a leader asks for the deterministic promotion of waiting members
```

## How it works

The extractor a handler takes sets its tier: `AuthUser` for members, `LeaderUser` for seat
assignment, squad holds, waitlist promotion and the member directory, `AdminUser` for event writes,
attachments, access administration and leave review, and `MachineCaller` with the `mod_runtime`
executor kind for the `/api/v1/game-runtime/*` routes, which act only within the credential's own
server.

- `event_listing.rs` counts and pages only the events the viewer may see, and `event_hub.rs`
  answers a hidden event like a missing one (404); both report the effective status of
  `services::event_status_rules`. The list's `scope` is `upcoming` (the default), `past` or `all`;
  any other word answers 400.
- Every reservation write, and every change to an existing event or its access, takes the event
  scope of `services::event_reservations` first and rechecks the actor's authority after the waits;
  refusals carry a stable `details.code`, and access changes also check the access revision their
  form was loaded at. In a test build a member's own claim in `slot_registration.rs` passes the
  failpoint `ReservationClaimBeforeCommit` after its audit row and `ReservationClaimAfterCommit`
  after its commit.
- `event_create_update.rs` creates an event through `services::event_authoring::event_creation`,
  whose field validators `PATCH` also applies, and `event_mission_attachment.rs` attaches through
  `services::event_authoring::mission_attachment`; the `staging-fixtures` host tool writes through
  the same services.
- `event_mission_attachment.rs` copies the mission's ORBAT into `orbat_slots` rows of the new event
  mission, the only ORBAT the [API](/documentation/glossary/a_to_f.md#api) writes, so it refuses an
  unreadable template, one that seats nobody, and a blank or padded faction, which no
  [armory](/documentation/glossary/a_to_f.md#armory) line could match.
- `game_runtime_roster.rs` reads the slot bindings of the deployment the server runs and compiles
  nothing; `game_runtime_deployments.rs` answers a refused player life with 200 and
  `decision = "denied"`.
- `fire_missions/` re-solves every save against its pinned ballistics catalog through
  `fire_mission_planning` and stores the server's solution with one row
  per gun; a client solution beyond the tolerance answers 422 `solution_mismatch`. A saved fire
  mission names an event the caller fully sees or none, so saving against, or listing those of,
  an event that is missing or hidden answers 404.
- `slot_registration.rs` answers 400 for a `slot_id` that is not a UUID and 404 for a well-formed
  one that names no seat; `event_group_administration.rs` answers 404 for a roster account that
  does not exist.

## Boundaries

- Depends on: the domain's `models` and `services`; `api_http_layer` for the extractors,
  `api_foundation` for the errors, pagination and wire formats, `http_url_guard` for the URL guard; the API crates for the audit rows (`api_audit_log`), `MachineCaller`
  (`api_caller_identity`), the statistics recompute (`api_member_activity`) and `TerrainType` and
  `GameMode` (`api_mission_vocabulary`); `api_identity_and_access` for user lookups and the Discord
  membership enrollment of partner guilds; `api_missions` for the mission title and terrain,
  `MissionArmory` and the deployment in effect; `api_match_telemetry::models` for the matches of the
  service record;
  `fleet_wire_contract` for `ExecutorKind`; `mission_model::orbat` for the faction join-key check;
  `fire_mission_planning` for the ballistics.
- Used by: the domain's `routes.rs`; over HTTP, the operations pages in
  `crates/frontend/pages/operations_pages/src/`, the
  [event manager](/documentation/glossary/a_to_f.md#event-manager) in
  `crates/frontend/pages/administration_pages/src/event_manager/`, the mortar calculator in
  `crates/frontend/pages/field_tools_pages/src/mortar/`, the endpoint helpers in
  `crates/frontend/foundation/frontend_transport/src/endpoints/`, and the game runtime's roster loader in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/` and deployment queues in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/`.
- Rules: every handler carries its `/// @route` tag (`cargo xtask verify route-tags`); no handler
  imports another domain's handlers (`crates/api/api_server/src/tests/architecture_rules.rs`); the
  roster never compiles or pairs at read time
  (`roster_reads_the_deployed_artifact_bindings_and_never_compiles` in
  `tests/game_runtime_roster.rs`).
- Body decoding: every JSON body is read through `ApiError::from_json_rejection`: 413 with
  `details.code = request_too_large` over the body limit, 415 without a JSON content type, and 400
  with the decoder's message (which names the failing field) otherwise.
- Path decoding: every path segment is read through `api_foundation::http::path_parameters::PathParams`: a
  segment that does not decode into its type answers 400 in the `{error}` envelope with a message
  naming the parameter, never axum's plain-text rejection.

## Related documentation

- [Event eligibility and allocation](/documentation/crates/api/api_server/verification_evidence/event_eligibility_allocation.md)
  — access, visibility, pools, promotion and re-evaluation.
- [Event hub page](/documentation/crates/frontend/pages/operations_pages/event_detail/event_hub_page.md)
  and [Event manager page](/documentation/crates/frontend/pages/administration_pages/event_manager/event_manager_page.md)
  — the pages over the event routes.
