**Status:** live

# Template: feature doc

**When to use:** the documentation of one feature or page area, the layer below the code README:
how it behaves, the data it uses, its design, its open work and the decisions behind it. A feature
doc sits in the feature's folder under `documentation_v2/`, beside that folder's README index, and
is named after what it covers: `<page component>_page.md` in a page folder
(`event_schedule_page.md`), `<screen>_specification.md` for a mod screen, a subject name
elsewhere. The feature's `visual_references/`, research and evidence sit in the same folder. The
[README standard](/documentation_v2/standards/readme_standard.md) holds the writing rules a feature
doc shares with READMEs.

## Skeleton

Copy the block and replace every `<…>` placeholder; each one says what goes there. The sections come
in this order, spelled this way; finer structure goes into `###` headings inside them.

````markdown
**Status:** live

# <Feature name, in plain words>

<One to three sentences: what the feature does and for whom.>

## Where it lives

- Code: <repository-root links to the code folders, and the files that matter most>
- Entry: <the route and its component, the command, or the class that starts the feature, with
  the file that declares it>
- <Related features: links to the feature docs this one embeds or hands off to>

## Behaviour

<What the user or caller sees and does, as the code behaves now: the flows as numbered steps, the
states and what moves between them, the rules and limits. Interface text is quoted as the code
writes it.>

## Data

- <METHOD /api/v1/path, message or file>: <the handler or loader, the DTO or schema, and what the
  feature does with it>
- <the storage the feature reads and writes: context, browser storage, database tables>

## Design

<The layout and visual design as built, and the design target: the visual_references/ sets and the
design system pages the feature follows, linked.>

## Open work

- [<ticket id> — <ticket title>](/documentation_v2/tickets/specs/<spec file>.md) (<status>): <what
  changes for this feature when it ships>

## Decisions

- <the decision in one sentence>: <why, and what it rules out>; <a link to its decisions.md entry
  when the feature keeps a log>
````

Open work lists only tickets whose status is idea, queued, ready, running, review or deferred,
each checked in `.ai/tickets/`; a shipped ticket's lasting knowledge moves into Behaviour, Data or
Decisions. Open work says "None." when nothing is open. A feature doc stays within 500 lines and is
split by topic into a folder with a README index when it grows past that.

## Worked sample

Written from `apps/website/frontend/src/v2/pages/operations/schedule/`, the API's event handlers
and the ticket registry. The sample sits in a fenced block, so no gate reads it; the page's own
feature doc is written from the same code and may differ.

````markdown
**Status:** live

# Event schedule page

The `/events` page: members browse the upcoming events in a list and open any event's full hub
beside it, briefing and ORBAT included, without leaving the list.

## Where it lives

- Code: [schedule page](/apps/website/frontend/src/v2/pages/operations/schedule/): `page.rs`, the
  route component `EventSchedulePage`, and `upcoming_ops.rs`, one event card.
- Entry: the `/events` route in `apps/website/frontend/src/app_routes.rs`; route tier `none`,
  full-bleed inside the navigation frame (`apps/website/frontend/src/router.rs`).
- Related features: the [event hub page](/documentation_v2/website/frontend/pages/operations/event_detail/event_hub_page.md),
  whose view the schedule embeds.

## Behaviour

1. A viewer who is not signed in sees "Sign in to load live data from the platform." from
   `AuthGate`; the list loads only for a signed-in viewer.
2. The master column, headed "Upcoming Ops", shows each upcoming event the viewer may see as a
   card: local start time, status badge, title (`name_override`, else "Untitled Operation"),
   mission and slot counts, a countdown or `LOCKED` when registration is locked, and a fill bar.
3. The first event is selected until the viewer picks another. The detail column fetches the
   selected event's hub and renders it with the view `/events/:id` uses, so the viewer reads the
   briefing and registers for a slot in place.
4. A hub fetched for one event never shows under another: the result carries its event id, and
   the column renders it only while that id is the selected one.
5. States: "Loading…" and "Failed to load data." for the list; "No upcoming operations scheduled."
   beside "Select an operation to view its hub." when the list is empty; "Loading operation…" and
   "Could not load this operation's hub." for the hub.

## Data

- `GET /api/v1/events` (`list_events` in
  `apps/website/api_v2/src/operations/handlers/event_listing.rs`): the default scope `upcoming`
  returns the events that start ahead or are live now, in start order, filtered to those the
  viewer's access admits; each item adds `mission_count`, `registered`, `filled`, `total_slots` and
  `percent` to the event row. The page reads it as `Paginated<serde_json::Value>`.
- `GET /api/v1/events/{id}` (`get_event` in `event_hub.rs`): the `EventHub` the detail column
  renders.
- `POST` and `DELETE /api/v1/event-missions/{emid}/register`: registration and withdrawal, sent by
  the embedded hub view.
- The session comes from the `AuthStore` context; the page stores nothing.

## Design

- A `SplitPane` with a 24rem master column of cards and the hub as its detail; the empty detail
  shows the `calendar_month` icon.
- The fill bar follows the server's `percent`, clamped to 0 to 100, because `total_slots` is zero
  until missions are attached.
- Design target: the [operations schedule blueprint](/documentation_v2/website/frontend/pages/operations/schedule/visual_references/operations_schedule_blueprint/operations_schedule_blueprint.png).

## Open work

- [T-940.3 — Reschedule cascades to event missions; delete hides schedule](/documentation_v2/tickets/specs/t940_website_platform.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-940_3_plan.md)): rescheduling an event shifts
  every event mission's start time with it, and deleting a mission hides its schedule entries and
  withdraws their registrations.

## Decisions

- The schedule embeds the hub rather than linking to it: one click shows an event's briefing and
  ORBAT, and registration happens without leaving the list; `/events/:id` keeps the standalone hub
  for deep links.
- The list is the page's only view: no table toggle and no calendar.
````
