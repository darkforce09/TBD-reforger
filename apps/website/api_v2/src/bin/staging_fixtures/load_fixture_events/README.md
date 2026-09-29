# Load fixture events

The `seed-load-fixture-events` and `clean-load-fixture-events` subcommands of the `staging-fixtures`
host tool: the ten [events](/documentation_v2/glossary/a_to_f.md#event) the synthetic load population
registers on during a staging load run, and their removal afterwards.

## Contents

```text
apps/website/api_v2/src/bin/staging_fixtures/load_fixture_events/
├── fixture_cleaning.rs  `clean-load-fixture-events`: deletes the fixture events and their registrations
├── fixture_plan.rs      the fixture's shape: titles, start times, place cap and the 2 × 8 × 8 ORBAT
├── fixture_seeding.rs   `seed-load-fixture-events`: the preconditions, the plan and the writes
├── mod.rs               the two parsers the subcommand table in `main.rs` names
└── tests/               unit tests for the fixture's capacity, slot order and planned events
```

## How it works

The seeding creates the events `[Load fixture] 01` to `[Load fixture] 10`, open for registration,
the first starting 14 days after the seeding minute and each later one an hour after the one before.
Each is capped at 128 places and attaches the [mission](/documentation_v2/glossary/g_to_m.md#mission)
`--mission` names with the fixture [ORBAT](/documentation_v2/glossary/n_to_z.md#orbat): factions
`Load A` and `Load B`, each with squads `<faction> Squad 1` to `<faction> Squad 8` of eight
[slots](/documentation_v2/glossary/n_to_z.md#slot). Slot s of an event is faction s div 64, squad
(s mod 64) div 8 + 1 and slot index s mod 8, the order `(faction, squad, slot_index)` sorts the rows
into; the load workload sends account k of the 1,100-account population to event k mod 10 and slot
k div 10, so each event seats 110 accounts and keeps 18 slots free.

```text
seed:  DISCORD_BOT_TOKEN unset? ─▶ lowest reserved account = author (the population's first)
       ─▶ no fixture event of a reserved author yet ─▶ mission present, not archived ─▶ plan
       ─▶ with --apply, per event: event_creation::create_event ─▶ member-admitting policy and
          open, uncapped member pool confirmed ─▶ mission_attachment::attach_mission ─▶ commit
clean: [Load fixture] events of reserved authors (FOR UPDATE) ─▶ census ─▶ plan
       ─▶ with --apply: registration history ─▶ registrations ─▶ events (attachments, slots,
          pools, allocations and groups cascade) ─▶ one event.load_fixture_removed row each
```

The seeding writes through the services the administrator routes use, so each event carries its
`event.created` and `event.mission_attached` audit rows; the attachment runs for the author as a
host tool account, without a session. The cleaning deletes no account and keeps every audit row; a
deployment or a live slot occupancy of a fixture event restricts the delete and fails the run. Both
are one transaction, and neither writes without `--apply`.

## Boundaries

- Depends on: the tool's `argument_list`, `guarded_context`, `reserved_accounts` and
  `tool_failure`; the `website_api` library's
  `operations::services::event_authoring::{event_creation, mission_attachment}`,
  `operations::services::{OrbatSquadTemplate, OrbatSlotTemplate}` and
  `administration::services::required_audit::append_system_audit`.
- Used by: the subcommand table in `apps/website/api_v2/src/bin/staging_fixtures/main.rs`;
  `apps/website/api_v2/tests/staging_fixtures_fixture_events.rs`, which runs the built binary.
- Rules: the seeding needs the load population (`seed-load-population`) first and the cleaning runs
  before its removal; the cleaning deletes only `[Load fixture]` events whose author lies in the
  reserved range, and what hangs off them; test functions start with `staging_fixtures_`.

## Related documentation

- [Staging fixtures host tool](/apps/website/api_v2/src/bin/staging_fixtures/README.md) — the guards
  every subcommand passes.
- [Event authoring services](/apps/website/api_v2/src/operations/services/event_authoring/README.md)
  — the event rules and the attachment the seeding writes through.
