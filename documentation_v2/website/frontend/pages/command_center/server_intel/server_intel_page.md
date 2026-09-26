**Status:** live

# Server intel page

The `/server-intel` page in the [command center](/documentation_v2/glossary/a_to_f.md#command-center): a
signed-in member sees one game server's live state, its connect address, its population and
frame rate, the theatre of its current match, the in-game time and weather, and the modpack it
requires, kept current by the server's [SSE](/documentation_v2/glossary/n_to_z.md#sse) status stream.

## Where it lives

- Code: [`apps/website/frontend/src/v2/pages/command_center/server_intel/`](/apps/website/frontend/src/v2/pages/command_center/server_intel/):
  `page.rs` holds the route component `ServerIntelPage`, the server list fetch and the one stream
  subscription; `server_list.rs` the server pick, the panel shell and the intelligence strip;
  `direct_connect.rs` the header with the address and the launch button; `player_census.rs` the
  telemetry grid. The folder's
  [README](/apps/website/frontend/src/v2/pages/command_center/server_intel/README.md) describes
  each file.
- Entry: the route, its tier and its layout are in the README's
  [Routes](/apps/website/frontend/src/v2/pages/command_center/server_intel/README.md#routes). The
  sidebar lists the page as "Server Intel" in the "Command Center" section.
- Related: the [server control page](/documentation_v2/website/frontend/pages/administration/server_control/server_control_page.md),
  where administrators run the servers this page reports on; the
  [dashboard page](/documentation_v2/website/frontend/pages/command_center/dashboard/dashboard_page.md),
  whose "Server Uplink" card summarises one server; the
  [API](/documentation_v2/glossary/a_to_f.md#api)'s
  [server infrastructure domain](/apps/website/api_v2/src/server_infrastructure/README.md), which
  serves the server list and the stream.

## Behaviour

### Choosing the server

1. The page body sits in `AuthGate` (`apps/website/frontend/src/v2/core/ui/gates.rs`), which shows
   the session states of the README's
   [States](/apps/website/frontend/src/v2/pages/command_center/server_intel/README.md#states)
   until the viewer is signed in.
2. The signed-in half fetches the server list and shows "Loading…", then "Failed to load data."
   or the panel. An empty list shows "No servers configured."
3. `pick_default` chooses one server: the first active row in the list, else the first row. The
   page has no picker and no grid of servers, so every other server stays out of view.

### Keeping it live

1. As soon as the list names a server with an id, the page opens that server's status stream,
   once per mount (the `subscribed` flag, read untracked, guards it).
2. Until the first frame arrives, the panel reads the status the list row carried; after that,
   each frame replaces it. A server that has never reported has no status, and every readout then
   shows "—" and the population 0.
3. A status the page cannot read as `ServerStatusDto`, in the row or in a frame, is reported to
   the rejected-frame audit and treated as absent.
4. Leaving the route closes the stream: the abort is registered on the page owner, so a re-run
   of the suspense fragment does not close a stream that is still on screen.

### The panel

1. The header shows a dot, green when the server reports online and yellow otherwise, with the
   tooltip "Server Online" or "Server Offline"; the server's name; the address chip `ip : port`;
   and "LAUNCH & CONNECT".
2. The copy button copies `ip:port` through the crate's clipboard helper, which shows the toast
   "Server address copied" only once the browser confirmed the write.
3. "LAUNCH & CONNECT" launches nothing: it shows the toast "Launch requires the Reforger client".
4. The grid's first column, "Active Personnel", shows the player count over the cap, "Uptime:"
   and "Server FPS:" with "Optimal" at 30 frames per second or more and "Low" below
   (`FPS_OPTIMAL_FLOOR` in the same folder's `player_census.rs`). The count is a total: the page
   has no per-player roster and no split by faction.
5. "Theater of Operations" shows a fixed terrain image, the terrain's name when the list row
   carries one, and "Match <first 8 characters of the match id>" or "No Active Mission". The tile
   links to `/events`.
6. The last column shows "Simulated Time", "Conditions" and "Mod Configuration": the required
   modpack as "<name> v<version>", with "(Synced)" when it is the current modpack, or
   "No modpack required".
7. "Recent Intelligence", at the foot of the panel, shows two fixed lines written into the code,
   "[14:02:00Z] New hostile movement detected in Sector 4" and "[13:45:12Z] Server Uplink
   maintenance completed". No feed supplies them.

### Known discrepancies

- The "Recent Intelligence" strip reads as live reports but is fixed text copied from the
  blueprint (`server_panel` in
  `apps/website/frontend/src/v2/pages/command_center/server_intel/server_list.rs`); the API has no
  intelligence feed, and the list row carries nothing the strip could show
  (`list_servers` in `apps/website/api_v2/src/server_infrastructure/handlers/server_intel.rs`).
- "LAUNCH & CONNECT" is styled as the page's main action and answers with a success toast, but
  connects nothing (`launch_stub` in
  `apps/website/frontend/src/v2/pages/command_center/server_intel/direct_connect.rs`).

## Data

The README's [Data](/apps/website/frontend/src/v2/pages/command_center/server_intel/README.md#data)
lists each call with the DTO the page reads. Server-side:

- `GET /api/v1/servers` (`list_servers` in
  `apps/website/api_v2/src/server_infrastructure/handlers/server_intel.rs`), for any signed-in
  member: every registered server, inactive ones included, ordered by name, unpaged, as intel
  cards. A card is the server row (`id`, `name`, `ip`, `port`, `required_modpack_id`,
  `is_active`) with its `server_statuses` row as `status` (null when the server never reported),
  its required modpack when it names one, and `terrain`, the terrain of the match the status
  names as current (null when there is none).
- `GET /api/v1/servers/{id}/status/stream` (`stream_server_status` in
  `apps/website/api_v2/src/server_infrastructure/handlers/server_status_stream.rs`), for any
  signed-in member: opens with the current status, then relays every frame the realtime hub
  publishes on `server:{id}`. Frames come from the game server's heartbeat
  (`apps/website/api_v2/src/match_telemetry/handlers/server_heartbeat.rs`) and from the
  `server_status_publisher` worker, which republishes every server's status on an interval
  (`SERVER_STATUS_PUBLISH_INTERVAL_SECS`, 10 seconds by default). The page opens it with
  `stream_server_status` in `apps/website/frontend/src/v2/core/api/sse.rs`, a fetch that sends
  the bearer token.
- `GET /api/v1/servers/{id}/status`, one server's card, exists and is not called by this page.

The page writes nothing and stores nothing in the browser.

## Design

- One frosted panel over a darkened map backdrop (`COMMAND_MAP_IMAGE`): the header band, a
  three-column telemetry grid (one column on narrow screens) and the intelligence strip. The
  backdrop and the theatre tile are fixed images hosted on `lh3.googleusercontent.com`.
- Design target: the [server intel blueprint](/documentation_v2/website/frontend/pages/command_center/server_intel/visual_references/server_intel_blueprint/README.md),
  a design-phase reference, and the archived platform spec's
  [Server Intel section](/documentation_v2/archive/go_and_react_era_design/platform_context_handoff.md#1-server-intel).
  The built page follows the blueprint closely and differs:
  - the theatre tile's second line is "Match <id>" or "No Active Mission" rather than an
    operation's name, and its image does not change with the terrain;
  - "Uptime:" shows the formatted uptime and "Simulated Time" and "Conditions" show the game's
    values as the heartbeat reported them, or "—";
  - the intelligence strip is the blueprint's two lines, copied as fixed text.
- The archived spec asked for a 3-column grid of health, current operation and connection cards
  with a "Launch via Steam / Join Server" button. A design-phase layout sketch asked for a list
  of server cards, a per-faction player census and a `steam://connect` link. None of these is
  built: the page shows one server, a total head count and a copy button for the address.

## Open work

- [T-088 — Multi-server picker](/.ai/tickets/T-088.toml) (deferred, no plan): the viewer chooses
  which server the page reports on instead of always seeing the first active one.

## Decisions

- One server, chosen by rule: the page reports the first active server rather than asking the
  viewer, and a picker waits for T-088.
- The status stream is the only live connection in the command center: the list gives the first
  paint, and the stream keeps it current without polling.
- No readout invents a value: a missing status shows "—" and a missing terrain shows no name;
  the tests in
  `apps/website/frontend/src/v2/pages/command_center/server_intel/tests/server_intel_t385.rs` keep
  the theatre wired to the `terrain` key.
- The copy button reports a copy only after the clipboard write resolved
  (`class_r_copy_address_routes_through_the_awaited_clipboard_helper` in
  `apps/website/frontend/src/v2/pages/command_center/server_intel/tests/server_intel_t773.rs`),
  because an unconfirmed toast claimed copies that never happened
  on insecure origins.
