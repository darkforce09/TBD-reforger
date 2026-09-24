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

<What the user or caller sees and does, as the code behaves now: the flows as numbered steps, what
moves the feature from one state to the next, the rules and limits, and the reasons behind them.
Interface text is quoted as the code writes it; for a page or an app, link the README's Routes and
States instead of repeating them.>

## Data

- <METHOD /api/v1/path, message or file>: <the handler or loader, the DTO or schema where no README
  lists it, and what the call means server-side: what it returns or changes, and the rules it
  applies>
- <the storage the feature reads and writes beyond what the README's Data lists, which is linked
  here, not repeated>

## Design

<The layout and visual design as built, and the design target: the visual_references/ sets and the
design system pages the feature follows, linked, and each difference between the built UI and the
target. No screenshots of the built UI; the only images are the visual_references sets.>

## Open work

- [<ticket id> — <ticket title>](<its spec, or `/.ai/tickets/T-<id>.toml` when it has none>)
  (<status>): <what changes for this feature when it ships>

## Decisions

- <the decision in one sentence>: <why, and what it rules out>; <a link to its decisions.md entry
  when the feature keeps a log>
````

Open work lists only tickets whose status is idea, queued, ready, running, review or deferred,
each checked in `.ai/tickets/`; a shipped ticket's lasting knowledge moves into Behaviour, Data or
Decisions. Open work says "None." when nothing is open. A design-target gap no open ticket covers
goes into Design as a difference and into the writer's report; Open work lists tickets only. The
feature doc and the README of the page or app it covers split the facts: the README holds what the
code declares (routes, calls with their DTOs, states with their exact text), and the feature doc
holds the flows, rules and reasons, what each call means server-side, the design, the open work
and the decisions, linking the README's Routes, Data and States. A feature doc stays within 500
lines and is split by topic into a folder with a README index when it grows past that.

## Worked sample

Written from `apps/website/frontend/src/v2/pages/operations/schedule/`, the API's event handlers
and the ticket registry. The sample sits in a fenced block, so no gate reads it; the page's own
feature doc is written from the same code and may differ.

````markdown
**Status:** live

# Event schedule page

The `/events` page: members browse the upcoming [events](/documentation_v2/glossary.md#event) in
a list and open any event's full hub beside it, briefing and ORBAT included, without leaving the
list.

## Where it lives

- Code: [`apps/website/frontend/src/v2/pages/operations/schedule/`](/apps/website/frontend/src/v2/pages/operations/schedule/):
  `page.rs`, the route component `EventSchedulePage`, and `upcoming_ops.rs`, one event card.
- Entry: the `/events` route, whose component, access and layout the page README's
  [Routes](/apps/website/frontend/src/v2/pages/operations/schedule/README.md#routes) gives.
- Related features: the [event hub page](/documentation_v2/website/frontend/pages/operations/event_detail/event_hub_page.md),
  whose view the schedule embeds.

## Behaviour

1. The list loads only for a signed-in viewer; anyone else gets `AuthGate`'s sign-in prompt.
2. The master column, headed "Upcoming Ops", shows each upcoming event the viewer may see as a
   card: local start time, status badge, title (`name_override`, else "Untitled Operation"),
   [mission](/documentation_v2/glossary.md#mission) and slot counts, a countdown or `LOCKED` when
   registration is locked, and a fill bar.
3. The first event is selected until the viewer picks another. The detail column fetches the
   selected event's hub and renders it with the view `/events/:id` uses, so the viewer reads the
   briefing and registers for a slot in place.
4. A hub fetched for one event never shows under another: the result carries its event id, and
   the column renders it only while that id is the selected one.

The text of each loading, empty and failed state is in the page README's
[States](/apps/website/frontend/src/v2/pages/operations/schedule/README.md#states).

## Data

The page README's [Data](/apps/website/frontend/src/v2/pages/operations/schedule/README.md#data)
lists each call with the DTO the page reads. Server-side:

- `GET /api/v1/events` (`list_events` in
  `apps/website/api_v2/src/operations/handlers/event_listing.rs`): the default scope `upcoming`
  returns the events that start ahead or are live now, in start order, filtered to those the
  viewer's access admits; each item adds `mission_count`, `registered`, `filled`, `total_slots` and
  `percent` to the event row.
- `GET /api/v1/events/{id}` (`get_event` in
  `apps/website/api_v2/src/operations/handlers/event_hub.rs`): once the viewer's access to the
  event is checked, the event and each attached mission's dossier in start order, read in one
  read-only snapshot.
- `POST` and `DELETE /api/v1/event-missions/{emid}/register`: registration and withdrawal, sent by
  the embedded hub view.

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
