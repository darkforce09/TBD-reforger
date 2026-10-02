# Wire types

The data transfer objects: one Rust shape per JSON body the [API](/documentation/glossary/a_to_f.md#api)
sends or accepts, grouped by domain. Most files are re-exported flat from `mod.rs`, so a caller names
the type (`MissionDetail`) rather than the file it lives in; the administration, ballistics catalog,
vehicle, wiki and equipment data viewer types are named by their module (`dto::wiki::WikiArticle`).

## Contents

```text
apps/frontend/src/v2/core/api/dto/
├── administration.rs               the personnel roster page, audit lines, the audit stream's ready and reset
├── auth.rs                         the viewer's profile, the Arma link, member rows and the member search page
├── ballistics_catalogs.rs          the stored catalog versions, the map engine's catalog document, the upload report
├── common.rs                       the list envelopes and `absent_null_or_value`, the patch field's wire form
├── content.rs                      modpack rows with their mods, the current modpack, announcement rows
├── equipment_data_viewer/          the equipment data viewer's read-only pages, one module per endpoint family
├── event_access_administration.rs  an event's access policies, groups, pools and every change body
├── event_viewer_access.rs          what the viewer may see and reserve in one event
├── events.rs                       the event list, ORBAT and hub, the service record, leave requests
├── fire_missions.rs                the save body, the stored fire mission with its guns, the save answer; re-exports the engine solution
├── fleet_commands.rs               fleet command receipts, their list and the request constructors
├── fleet_scenarios.rs              the terrain-to-mission-header entries and the body that sets one
├── match_events.rs                 a page of a match's detailed events, each payload typed by its kind
├── mission_deployments.rs          a mission deployment, a server's page of them, the request body
├── mission_reviews.rs              review history and thread, decisions, artifacts, review workspace
├── missions.rs                     mission cards, rows, detail and versions; armory; approval rows
├── mod.rs                          the module tree; re-exports the DTOs flat, but for five modules
├── registry.rs                     registry items, compatibility edges, cargo defaults and factions
├── servers.rs                      server rows, registration and change bodies, the status frame, credentials
├── telemetry.rs                    the dashboard summary with its fleet, and the leaderboards
├── tests/                          unit tests for the golden round trips and the fixture-free shapes
├── vehicles.rs                     vehicle database rows, the create and replace body, the three-state patch
└── wiki.rs                         wiki summaries, the article and its typed blocks, saves, refusals, revisions
```

## How it works

The DTOs mirror the snake_case models of the API in `apps/api/src/<domain>/models/`,
and the API wins a disagreement. A DTO that projects a definition in `contracts/definitions/`
names it in an `@contract` line of its docs, which `cargo xtask schema citations` resolves. They
are plain `serde` data, and all of them compile into the native test build. `tests/r_api.rs` holds each DTO to a captured answer from
`contracts/fixtures/api_goldens/`: re-serialising reproduces the capture canonically,
byte for byte, and the keys no named field reads (the ones a `#[serde(flatten)]` catch-all sweeps
up, found by poisoning each value) are exactly the ones the test lists. The `tests/r_api_*.rs`
files hold one domain's goldens each; `tests/shapes.rs` checks the shapes that need no capture, and
`tests/administration.rs`, `tests/vehicles.rs` and `tests/wiki.rs`, the sibling tests of their
modules, check the stream events, write bodies and refusals no capture carries.

- A value set the API may extend (review states,
  [fleet command](/documentation/glossary/a_to_f.md#fleet-command) actions and states,
  [mission deployment](/documentation/glossary/g_to_m.md#mission-deployment) states, leave statuses,
  reservation values) travels as a string, so a new value lists instead of failing the read.
- A null the capture carries stays explicit when serialising: an unclaimed
  [ORBAT](/documentation/glossary/n_to_z.md#orbat) [slot](/documentation/glossary/n_to_z.md#slot), a
  finding with no subject, a pool with no limit, an unmeasured K/D ratio, a saved fire mission's
  unrecorded coordinates and solution figures.
- A text field the API skips when empty (a modpack's `workshop_url`, a mod's `workshop_id`,
  `mod_guid` and `version`, an announcement's `snippet`, `thumbnail_url` and
  `discord_message_id`) reads as an empty string and is left out again when serialising; an
  announcement's tag and status travel as strings, like the other extensible value sets.
- The [event](/documentation/glossary/a_to_f.md#event) access conditions and group sources are tagged
  by `kind` and refuse unknown fields, because the
  [event manager](/documentation/glossary/a_to_f.md#event-manager) sends them back and must not
  rewrite a shape it does not know; `MissionDetail` has no catch-all either.
- A match event's `kind` and `payload` decode together into one `MatchEventDetail` variant per
  kind, so a payload cannot be read under the wrong kind; an unknown kind fails the read, and a
  kill's `distance_m` keeps the number as sent. A status's `telemetry_queue` and a fleet server's
  `status` are absent keys, never placeholders, when there is no reading.
- The closed sets the contract enumerates and a page branches on are enums: the audit severity, the
  audit stream's reset reason, a wiki table column's alignment, a callout's kind, and a wiki save
  refusal's code and its findings' codes. A value outside the set fails the read.
- A wiki article carries its markdown and the typed tree the API parsed from it: `WikiBlock` and
  `WikiInline` are tagged by `type`, their optional fields are absent rather than `null`, and the
  formatting-guide capture shows every variant (`tests/wiki.rs` checks that it still does).
  `WikiSaveRequest` always sends `base_revision`, `null` when the save creates the page.
- `VehiclePatch` tells an absent key, `null` and a value apart for each optional field
  (`Option<Option<String>>` through `absent_null_or_value` in `common.rs`): absent leaves the
  field, `null` clears it, a value sets it. A required field is absent or a value, because the API
  refuses a cleared one. `ServerChange`, the server registry's `PATCH` body, reads its
  `required_modpack_id` the same way, and a server registration or change answers with the
  `ServerRowDto` the server list reads.
- A fire mission's solution (the battery, each gun's charge rows with their refusals and wind
  corrections, the dispersion, the time fuze with its burst-point aim, the crest clearance) and a
  ballistics catalog document are the map engine's own types, re-exported, so the mortar
  calculator renders, solves and posts exactly the shapes the engine produces and the API
  re-solves. A solution's `fuze` and `crest`, and the save body's `event_id`, `charge_rings`,
  `wind` and `burst_height_m`, are absent rather than `null` when unset; a stored mission whose
  dispersion claims `verified_in_engine: true` fails the read, refused by the engine's
  dispersion deserializer.
- The audit stream's `ready` and `reset` events carry publication sequences, never audit line ids;
  an audit line's `metadata` is carried as the writer recorded it.
- `IssuedMachineCredential`, the one answer that carries a
  [machine credential](/documentation/glossary/g_to_m.md#machine-credential)'s secret, derives no
  `Debug`, so the secret cannot reach a log line.
- `decode_server_status_frame` turns one [SSE](/documentation/glossary/n_to_z.md#sse) frame into a
  status or a named rejection, here rather than beside the browser-only stream reader so the
  native tests reach it; `compiled_meta` on `MissionDetail` and `ArtifactMetadata` gives the
  metadata the shared [mission](/documentation/glossary/g_to_m.md#mission) compiler reads.

## Boundaries

- Depends on: `serde` and `serde_json`; `User` and `Role` of `crate::v2::core::auth`;
  `map_engine::data`, for the compiler metadata and the re-exported `FactionDoc`,
  `FactionRole`, `FactionVehicle` and `MissionEnv`.
- Used by: the client, endpoint calls and live status stream in
  `apps/frontend/src/v2/core/api/`, the auth store in
  `apps/frontend/src/v2/core/auth/store.rs`, the pages under
  `apps/frontend/src/v2/pages/`, the
  [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) under
  `apps/frontend/src/v2/apps/editor/`, and the equipment data viewer under
  `apps/frontend/src/v2/apps/debug/data_viewer/`.
- Rules: the API model changes first and the DTO follows; a golden round-trips, and its unread
  keys match its list, so drift either way fails (`cargo test -p frontend`);
  `byte_equality_alone_cannot_see_a_dropped_field_under_flatten` in `tests/r_api.rs` shows why the
  key check sits beside the byte comparison.

## Related documentation

- [Documentation standards](/documentation/standards/documentation_standards.md#2-contracts-behind-the-tags)
  — how the API's models, the schemas and these DTOs stay one contract.
