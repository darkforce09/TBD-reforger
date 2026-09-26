# Operations handlers

The HTTP handlers of the [operations](/documentation_v2/glossary/n_to_z.md#operations) domain, one module
per surface: the [event](/documentation_v2/glossary/a_to_f.md#event) calendar and its hub, event writes and
[mission](/documentation_v2/glossary/g_to_m.md#mission) attachments, access administration, the
[ORBAT](/documentation_v2/glossary/n_to_z.md#orbat) and its [slots](/documentation_v2/glossary/n_to_z.md#slot),
the member directory, the [service record](/documentation_v2/glossary/n_to_z.md#service-record), leave
requests, fire missions, and the [game runtime](/documentation_v2/glossary/g_to_m.md#game-runtime)'s roster
and player [deployments](/documentation_v2/glossary/a_to_f.md#deployment).

## Contents

```text
apps/website/api_v2/src/operations/handlers/
├── event_access_administration.rs  an event's access view and evidence; its policies and pools
├── event_create_update.rs          create, update and delete an event under the event-scope locks
├── event_group_administration.rs   an event's roster and partner-guild groups and their members
├── event_hub.rs                    one event's dossier, projected for what the viewer may see
├── event_listing.rs                the calendar list, filtered to the events the viewer may see
├── event_mission_attachment.rs     attach a mission with a snapshot of its ORBAT, and detach it
├── fire_missions.rs                the mortar firing solution, saved fire missions, an event's list
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
  `services::event_status_rules`.
- Every reservation write, and every change to an existing event or its access, takes the event
  scope of `services::event_reservations` first and rechecks the actor's authority after the waits;
  refusals carry a stable `details.code`, and access changes also check the access revision their
  form was loaded at.
- `event_mission_attachment.rs` copies the mission's ORBAT into `orbat_slots` rows of the new event
  mission, the only ORBAT the [API](/documentation_v2/glossary/a_to_f.md#api) writes, so it refuses an
  unreadable template, one that seats nobody, and a blank or padded faction, which no
  [armory](/documentation_v2/glossary/a_to_f.md#armory) line could match.
- `game_runtime_roster.rs` reads the slot bindings of the deployment the server runs and compiles
  nothing; `game_runtime_deployments.rs` answers a refused player life with 200 and
  `decision = "denied"`.
- `fire_missions.rs` solves through `website_map_engine::data::scenario::ballistics` and stores
  the whole solution; a target out of range answers 422.

## Boundaries

- Depends on: the domain's `models` and `services`; `core` for the extractors, errors, pagination,
  wire formats and the URL guard; `administration` for the audit rows; `identity_and_access` for
  user lookups and the Discord membership enrollment of partner guilds; `missions` for the mission
  title and terrain, `MissionArmory` and the deployment in effect; `match_telemetry::models` for
  the matches of the service record; `server_infrastructure` for `MachineCaller` and
  `ExecutorKind`; `website_map_engine::data::scenario` for the faction join-key check and the
  ballistics.
- Used by: the domain's `routes.rs`; over HTTP, the operations pages in
  `apps/website/frontend/src/v2/pages/operations/`, the
  [event manager](/documentation_v2/glossary/a_to_f.md#event-manager) in
  `apps/website/frontend/src/v2/pages/administration/event_manager/`, the mortar calculator in
  `apps/website/frontend/src/v2/pages/field_tools/mortar/`, the endpoint helpers in
  `apps/website/frontend/src/v2/core/api/endpoints/`, and the game runtime's roster loader in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/` and deployment queues in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/`.
- Rules: every handler carries its `/// @route` tag (`cargo xtask verify route-tags`); no handler
  imports another domain's handlers (`apps/website/api_v2/src/tests/architecture_rules.rs`); the
  roster never compiles or pairs at read time
  (`roster_reads_the_deployed_artifact_bindings_and_never_compiles` in
  `tests/game_runtime_roster.rs`).

## Related documentation

- [Event eligibility and allocation](/documentation_v2/website/api_v2/verification_evidence/event_eligibility_allocation.md)
  — access, visibility, pools, promotion and re-evaluation.
- [Event hub page](/documentation_v2/website/frontend/pages/operations/event_detail/event_hub_page.md)
  and [Event manager page](/documentation_v2/website/frontend/pages/administration/event_manager/event_manager_page.md)
  — the pages over the event routes.
