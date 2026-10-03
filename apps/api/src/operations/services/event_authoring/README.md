# Event authoring services

The write paths that bring an [event](/documentation/glossary/a_to_f.md#event) and its seats into
being: creating the event row, and attaching a [mission](/documentation/glossary/g_to_m.md#mission)
with a snapshot of its [ORBAT](/documentation/glossary/n_to_z.md#orbat). The administrator routes
and the `staging-fixtures` host tool write through the same code.

## Contents

```text
apps/api/src/operations/services/event_authoring/
├── event_creation.rs      the event field rules and the insert with its `event.created` audit row
├── mission_attachment.rs  the ORBAT an attachment seats, and the attach with its slots and audit row
├── mod.rs                 the module tree
└── tests/                 unit tests for the event rules and the attachment template checks
```

## How it works

`event_creation.rs` turns an `EventCreationRequest` into an `EventCreation` by the rules the
`events` table does not enforce, in the order the route reports them: a start time is required and
stored to the microsecond; `max_slots` is 0 to 256 (0 leaves the event uncapped); the status is a
pre-start one (`scheduled`, `open` or `locked`); a name override is empty or not blank; a banner is
empty or an absolute http(s) URL. `create_event` then checks that a named server and modpack exist,
inserts the row and appends `event.created` on the caller's transaction. The new row takes the
table's defaults: the access policy that admits verified TBD members and a member reservation pool
that is uncapped and open from creation. `PATCH /api/v1/events/:id` applies the same field
validators to the fields it changes.

`mission_attachment.rs` resolves an `AttachmentTemplate`: the caller's explicit `orbat`, or the
ORBAT of the mission's current published version. It refuses a template that seats nobody (400 for
a requested one, 409 for the mission's), an `orbat` the API cannot read, and a squad whose faction
is blank or padded, which no [armory](/documentation/glossary/a_to_f.md#armory) line could match.
`attach_mission` takes only such a template:

```text
event row (FOR NO KEY UPDATE) ─▶ mission row (FOR NO KEY UPDATE; deleted 404, archived 409)
  ─▶ event scope: an administrator session is rechecked; a host tool account is locked as a
     system transaction ─▶ due lifecycle moves stored
  ─▶ a removed attachment of the mission is restored (`event.mission_restored`)
     or a new `event_missions` row (duplicate 409) with one `orbat_slots` row per template slot
     (`event.mission_attached`)
```

Slot rows store `faction` byte for byte, because the Event Hub joins it to the armory's faction.

## Public surface

- `event_creation`: `EventCreationRequest`, `EventCreation`, `create_event`, and the validators
  `validated_banner_image_url`, `check_name_override`, `require_server` and
  `require_event_modpack`.
- `mission_attachment`: `AttachmentTemplate` (`resolve`, `requested`, `slot_count`),
  `AttachmentAuthority` (`AdministratorSession`, `HostToolAccount`) and `attach_mission`.

## Boundaries

- Depends on: `operations::models`; the domain's `event_reservations` (schedule normalisation, the
  event scope lock, mission restoration), `event_status_rules` and `event_lifecycle_transition`;
  `core` for errors, configuration, `AuthUser`, the URL guard and unique-violation detection;
  `administration` for the audit rows; `mission_model::orbat` for the faction
  join-key check.
- Used by: `apps/api/src/operations/handlers/event_create_update.rs` and
  `event_mission_attachment.rs`; the `staging-fixtures` host tool's
  `apps/api/src/bin/staging_fixtures/load_fixture_events/`; the API test
  `apps/api/tests/staging_fixtures_fixture_events.rs`.
- Rules: every write runs on the caller's transaction, and authority is settled by the caller or,
  for an attachment, by the `AttachmentAuthority` it passes; nothing else inserts `events` or
  `orbat_slots` rows.

## Related documentation

- [Operations services](/apps/api/src/operations/services/README.md) — the reservation
  writers and the status rules these services build on.
- [Staging fixtures host tool](/apps/api/src/bin/staging_fixtures/README.md) — the load
  fixture events written through these services.
