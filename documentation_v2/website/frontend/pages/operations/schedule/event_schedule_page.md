**Status:** live

# Event schedule page

The `/events` page in the [operations](/documentation_v2/glossary/n_to_z.md#operations) section: a
signed-in member browses the upcoming [events](/documentation_v2/glossary/a_to_f.md#event) in a list and
reads any event's full hub beside it, briefing, places and
[ORBAT](/documentation_v2/glossary/n_to_z.md#orbat) slotting included, without leaving the list.

## Where it lives

- Code: [`apps/website/frontend/src/v2/pages/operations/schedule/`](/apps/website/frontend/src/v2/pages/operations/schedule/):
  `page.rs` holds the route component `EventSchedulePage`, the list fetch, the selection, the hub
  fetch and the split pane; `upcoming_ops.rs` one event card. The folder's
  [README](/apps/website/frontend/src/v2/pages/operations/schedule/README.md) describes each file.
- Entry: the route, its tier and its layout are in the README's
  [Routes](/apps/website/frontend/src/v2/pages/operations/schedule/README.md#routes). The sidebar
  lists the page as "Event Schedule", first in the "Operations" section.
- Related: the [event hub page](/documentation_v2/website/frontend/pages/operations/event_detail/event_hub_page.md),
  whose hub view (`event_hub_view`) the detail column renders; the
  [event manager page](/documentation_v2/website/frontend/pages/administration/event_manager/event_manager_page.md),
  where administrators create the events listed here; the
  [API](/documentation_v2/glossary/a_to_f.md#api)'s
  [operations domain](/apps/website/api_v2/src/operations/README.md), which serves the list and
  the hub.

## Behaviour

### The list

1. The page body sits in `AuthGate` (`apps/website/frontend/src/v2/core/ui/gates.rs`), which shows
   the session states of the README's
   [States](/apps/website/frontend/src/v2/pages/operations/schedule/README.md#states) until the
   viewer is signed in.
2. The signed-in half fetches the event list once, with no query parameters, and shows
   "Loading…", then "Failed to load data." or the split pane. The list is not fetched again while
   the page stays mounted.
3. The master column, headed "Upcoming Ops", shows one card per event in the order the API sent
   them, soonest first: the local start time, a status badge, the title (the event's
   `name_override`, else "Untitled Operation"), "<n> missions · <filled>/<total> slots", the time
   until the start as one rounded unit ("3 HOURS"), or `LOCKED` in yellow when registration is
   locked, and a fill bar.
4. The status badge shows the wire value: `open` green, `locked` yellow, `live` in the primary
   colour, `completed` tertiary, `cancelled` red, any other value (such as `scheduled`) neutral;
   the admin event manager uses the same colours.
5. The fill bar follows the server's `percent`, clamped to 0 to 100; `total_slots` is zero until
   [missions](/documentation_v2/glossary/g_to_m.md#mission) are attached, and the bar then stays empty.
6. With no events the list reads "No upcoming operations scheduled." beside the detail's
   "Select an operation to view its hub."

### The detail column

1. The first event is selected until the viewer clicks another card; the selected card is
   highlighted. A click writes the pick, and the selection falls back to the first event while
   nothing is picked.
2. The detail column fetches the selected event's hub and renders it with `event_hub_view`, the
   body `/events/:id` also shows, without that route's back link and backdrop. The viewer reads
   the briefing, the places and each mission's dossier, and registers for a
   [slot](/documentation_v2/glossary/n_to_z.md#slot) in place, as the
   [event hub page](/documentation_v2/website/frontend/pages/operations/event_detail/event_hub_page.md)
   describes.
3. A hub fetched for one event never shows under another: the result carries the id it was
   fetched for, and the column renders it only while that id is still the selected one. Until
   then it reads "Loading operation…", also while the previous event's hub is held. A failed
   fetch reads "Could not load this operation's hub."
4. After any registration, withdrawal or squad change in the embedded hub, the detail column
   fetches the hub again; the list and its cards keep the values of their one fetch.
5. The detail pane scrolls on its own, so the lower squads, the footer buttons and "Withdraw" are
   reached by scrolling the detail pane while the list stays in place.

### Known discrepancies

- A card's fill counts and bar do not follow a registration made in the detail column: the
  hub's change callback refetches only the hub (`board` in
  `apps/website/frontend/src/v2/pages/operations/schedule/page.rs`), while the counts come from
  the list fetch (`list_events` in
  `apps/website/api_v2/src/operations/handlers/event_listing.rs`).
- A card without a `name_override` reads "Untitled Operation" although its missions have titles;
  the dashboard names the same event after its first mission (`get_dashboard` in
  `apps/website/api_v2/src/command_center/handlers/live_dashboard.rs`).
- The list shows the first 20 events the viewer may see: the page sends no `limit` or `offset`
  and has no pager, and the API pages by 20 by default (`list_events`).

## Data

The README's [Data](/apps/website/frontend/src/v2/pages/operations/schedule/README.md#data)
lists each call with the fields the page reads. Server-side:

- `GET /api/v1/events` (`list_events` in
  `apps/website/api_v2/src/operations/handlers/event_listing.rs`), for any signed-in member:
  - The `scope` parameter picks the set; the page sends none, so the default `upcoming` applies:
    events that start after now, or whose effective status is `live`, soonest first. `past` and
    `all` exist and the page does not use them.
  - The status in each row is the effective status, derived in Postgres from the stored status
    and the clock (`EFFECTIVE_STATUS_SQL` in
    `apps/website/api_v2/src/operations/services/event_status_rules.rs`): an event turns `live` at
    its start time and `completed` six hours after its last mission starts, whether or not the
    lifecycle sweep has run. A cancelled event keeps its status and still lists while its start
    lies ahead.
  - A non-administrator sees only the events their access admits, and `total` and the page count
    only those; an event visible through squad or slot policies alone is counted from the seats
    the viewer may see. An administrator sees every event.
  - Each row adds `mission_count`, `registered`, `filled`, `total_slots` and `percent`
    (`filled * 100 / total_slots`, 0 when there are no slots).
  - The page is `limit` rows (20 by default, at most 100) from `offset`.
- `GET /api/v1/events/{id}` (`get_event` in
  `apps/website/api_v2/src/operations/handlers/event_hub.rs`): the selected event's hub; the
  [event hub page](/documentation_v2/website/frontend/pages/operations/event_detail/event_hub_page.md)
  gives its server-side meaning and the slotting calls the embedded view makes.

The page itself writes nothing and stores nothing in the browser.

## Design

- A `SplitPane` with a 24rem master column of cards and the hub as its detail; the empty detail
  shows the `calendar_month` icon.
- Design target: the [operations schedule blueprint](/documentation_v2/website/frontend/pages/operations/schedule/visual_references/operations_schedule_blueprint/README.md),
  a design-phase reference. The built page keeps its master-detail layout and differs:
  - a card shows missions and slots, a countdown and a fill bar, where the blueprint shows a
    terrain and "REGISTRATION 40/60";
  - the list holds upcoming events only, where the blueprint also shows a closed one;
  - the detail column is the event hub: a hero without a banner image, the Places panel, then
    every mission as a card of its own, where the blueprint shows a banner, one briefing and tabs
    for the missions;
  - the ORBAT sits inside each mission card, with the faction tabs above the squad list, where
    the blueprint shows faction tabs beside the "ORDER OF BATTLE" heading.
- A design-phase layout sketch planned a week-by-week date strip with previous and next buttons,
  an RSVP state on each row, and a section of historical after-action reports. None of these is
  built: the list has no date window and no archive, and the viewer's own sign-up shows inside
  the hub.

## Open work

None. No open ticket in `.ai/tickets/` changes this page; the known discrepancies above have no
ticket yet.

## Decisions

- The schedule embeds the hub rather than linking to it: one click shows an event's briefing and
  ORBAT, and registration happens without leaving the list; `/events/:id` keeps the standalone hub
  for links.
- The list is the page's only view: no table toggle, no calendar and no date window; the API's
  `upcoming` scope decides what it holds.
- The hub has one renderer: the schedule and `/events/:id` both call `event_hub_view`, so the two
  cannot drift apart (`schedule_briefing_empty_check_stays_trim_aligned` in
  `apps/website/frontend/src/v2/pages/operations/schedule/tests/schedule.rs`).
