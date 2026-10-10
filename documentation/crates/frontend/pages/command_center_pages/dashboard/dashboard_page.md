**Status:** live

# Dashboard page

The `/` page, the landing screen of the [command center](/documentation/glossary/a_to_f.md#command-center):
a signed-in member sees the countdown to the next [event](/documentation/glossary/a_to_f.md#event), the
configured fleet of game servers with its totals, their own [slot](/documentation/glossary/n_to_z.md#slot) in an upcoming
[mission](/documentation/glossary/g_to_m.md#mission), the current modpack and the three newest
announcements, all read from one request.

## Where it lives

- Code: [`crates/frontend/pages/command_center_pages/src/dashboard/`](/crates/frontend/pages/command_center_pages/src/dashboard/):
  `page.rs` holds the route component `DashboardPage`, the fetch and the panel grid;
  `hero_banner.rs` the banner; `server_uplink.rs` (with its totals row in `fleet_totals.rs`),
  `deployment.rs` and `modpack.rs` the three cards; `recent_intel.rs` the announcement feed. The folder's
  [README](/crates/frontend/pages/command_center_pages/src/dashboard/README.md) describes each
  file.
- Entry: the route, its tier and its layout are in the README's
  [Routes](/crates/frontend/pages/command_center_pages/src/dashboard/README.md#routes). The
  sidebar lists the page as "Dashboard", first in the "Command Center" section.
- Related: the [event hub page](/documentation/crates/frontend/pages/operations_pages/event_detail/event_hub_page.md),
  which the banner opens; the [announcements page](/documentation/crates/frontend/pages/command_center_pages/announcements/announcements_page.md),
  which each feed row opens; the [server intel page](/documentation/crates/frontend/pages/command_center_pages/server_intel/server_intel_page.md),
  the live view of a server; the [API](/documentation/glossary/a_to_f.md#api)'s
  [command center domain](/crates/api/api_command_center/src/README.md), which composes the
  payload.

## Behaviour

### Loading

1. The page body sits in `AuthGate` (`crates/frontend/foundation/frontend_session/src/gates.rs`): until a
   session is restored and signed in, the viewer sees the session states of the README's
   [States](/crates/frontend/pages/command_center_pages/src/dashboard/README.md#states)
   instead of the panels. The route itself has the tier `none`, so the navigation frame and the
   sign-in prompt render for anyone.
2. The signed-in half sends one `GET /api/v1/dashboard` and shows "Loading…" until it answers,
   then either "Failed to load data." or the panels. The page never fetches again while it stays
   mounted: every panel shows the values of that one answer.
3. `page.rs` splits the answer and hands each panel its own slice, owned: the banner at full
   width, then the three cards in a row, then the feed, which grows to fill the column.

### The panels

1. The banner shows "T-MINUS " and the time left until the next event starts, as one rounded
   unit in capitals ("T-MINUS 3 HOURS", "T-MINUS 2 DAYS"; `countdown_label` in
   `crates/frontend/foundation/frontend_ui/src/countdown.rs`). The label is worked out once, when the
   panel renders, and does not tick. Once the start time has passed at render it reads
   `LIVE NOW` without the prefix. Under it, "OPERATION: <name> — <terrain>", and an
   "Open Operation Hub" link to `/events/{event_id}`.
2. With no next event the banner keeps its shape, reads "NO UPCOMING OPS" and "Check the event
   schedule for new operations.", and offers no link.
3. "Server Uplink" shows the configured fleet: a pill with "<online>/<configured> ONLINE", then
   one row per active server in the API's order (name, then id) with a status dot, the name,
   "ONLINE" or "OFFLINE", "<players>/<cap> PLAYERS" and "FPS: " with the frame rate as the wire
   sent it. A server without a status row reads "OFFLINE" with "—" for players and frame rate.
   With no active server the card reads "No servers configured". A totals row closes the card:
   "ONLINE: <online>/<configured>", "PLAYERS: <players>/<cap of the online servers>",
   "QUEUE BACKLOG: " with the summed backlog of every reported telemetry queue, and
   "DROPPED: " with the summed dropped total, in the error tone above zero. Every total is the
   API's figure as sent; the card never recounts its rows.
4. "Deployment" shows the viewer's faction, squad and "Role: <role>" for their soonest seat, or
   "No active assignment".
5. "Modpack" shows "<name> v<version>", "SIZE: " with the size in GB (one decimal) or MB, and a
   green "STATUS: SYNCED" chip; with no current modpack, "No modpack", "—" and a grey
   "STATUS: NONE". The chip reports that a current modpack exists; the page cannot see the
   viewer's installed mods.
6. "Recent Intelligence" lists the announcements as rows: a date pill, a pin icon for a pinned
   announcement, the title (or "Untitled Post") and a two-line preview (the `snippet`, else the
   first paragraph of the `body`). A row links to `/announcements/{id}`, or to `/announcements`
   when it carries no id. With none, "No announcements yet."

Every panel keeps its heading and shape when its slice is empty, so the card row always holds
three cards.

### Known discrepancies

- The banner reads as a countdown ("T-MINUS …") but is fixed at render time
  (`crates/frontend/pages/command_center_pages/src/dashboard/hero_banner.rs`); the API returns
  only events that start after the moment of the request (`get_dashboard` in
  `crates/api/api_command_center/src/handlers/live_dashboard.rs`), so a started event leaves
  the banner on the next load rather than showing `LIVE NOW`.
- The "Server Uplink" pill draws "0/<n> ONLINE" in the same green as a fleet with servers
  online; only the dot turns grey
  (`crates/frontend/pages/command_center_pages/src/dashboard/server_uplink.rs`).

## Data

The README's [Data](/crates/frontend/pages/command_center_pages/src/dashboard/README.md#data)
lists the call and the DTO the page reads. Server-side:

- `GET /api/v1/dashboard` (`get_dashboard` in
  `crates/api/api_command_center/src/handlers/live_dashboard.rs`), for any signed-in
  member, composes five independent reads:
  - `next_event`: the soonest event the viewer created that starts after now and is scheduled,
    open or live; when there is none, the soonest such event of anyone's. The name is the
    event's `name_override`, else the title of its first mission; the terrain is that mission's.
    The lookup applies no event access policy. The object also carries `registered`, `max_slots`
    and `status`, which the page does not show.
  - `my_assignment`: the viewer's seat, the `orbat_slots` row assigned to them in the soonest
    event mission that starts after now, as faction, squad and role (and the event's id and name,
    which the page does not show).
  - `fleet`: the configured fleet (`load_fleet_overview` in
    `crates/api/api_command_center/src/services/fleet_overview.rs`): every server with
    `is_active = true`, ordered by name then id, each with its `server_statuses` row when it has
    one (including its `telemetry_queue` reading when the server reported one), and `totals`
    (`configured`, `online`, `players`, `max_players`, `telemetry_backlog`,
    `telemetry_dropped_total`). `online`, `players` and `max_players` count online servers only;
    the telemetry figures sum every reported queue reading. The
    [telemetry specification](/documentation/crates/api/api_server/design_notes/telemetry.md#fleet)
    defines the shape.
  - `current_modpack`: the modpack flagged current (`load_current_modpack` in
    `crates/api/api_community_content/src/services/modpack_lookup.rs`).
  - `recent_announcements`: the three newest published announcements, newest first, pinned or
    not.

The page writes nothing and stores nothing in the browser.

## Design

- A single scrolling column: the banner over a fixed backdrop image (`HERO_IMAGE`, hosted on
  `lh3.googleusercontent.com`), a three-column card grid (one column on narrow screens) and the
  feed, all as frosted glass panels.
- Design target: the [command dashboard blueprint](/documentation/crates/frontend/pages/command_center_pages/dashboard/visual_references/command_dashboard_blueprint/README.md),
  a design-phase reference. The built page follows its layout and headings and differs:
  - the countdown reads one rounded unit ("T-MINUS 3 HOURS") rather than `04:12:30`, and does
    not tick;
  - the banner link reads "Open Operation Hub" rather than "Enter Intelligence Hub";
  - "Server Uplink" lists every active server with players and frame rate, and a totals row with
    the telemetry backlog and drops, where the blueprint shows one server's ping and location;
  - the "Modpack" card keeps the blueprint's sync chip, which states no real sync check;
  - "Recent Intelligence" lists announcements with a date pill and preview, where the blueprint
    shows timestamped operational messages.
- A design-phase layout sketch planned a banner pairing unit dispatches with the countdown, a
  "Personal Deployment" card with the viewer's next sign-up, rank and leave of absence, a modpack
  card with a hash and a sync link, a live area of operations on the server card, and a feed mixing
  announcements with operations. None of these is built: the cards show the assignment, the
  modpack's size and the fleet's figures, and the feed holds announcements only.

## Open work

None. No open ticket in `ttm` changes this page; the known discrepancies above have no
ticket yet.

## Decisions

- One request feeds the whole page: the dashboard is a summary, so the API composes it in one
  call and each panel reads only its own slice; a panel with nothing to show says so rather than
  hiding, so the layout never shifts.
- The page only reads: registering, slotting and reading in full happen on the event hub and
  announcement pages the panels link to.
- The next event prefers one the viewer created: the API comment in `get_dashboard` gives the
  reason, that an organiser's own upcoming event wins over older rows left in a test database;
  every other member sees the soonest event of anyone's.
