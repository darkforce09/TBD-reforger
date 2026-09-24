# Server intel page

The `/server-intel` page in the
[command center](/documentation_v2/glossary.md#command-center): one game server's live state, its
connect address, population, theatre and environment, kept current by the server's
[SSE](/documentation_v2/glossary.md#sse) status stream.

## Contents

```text
apps/website/frontend/src/v2/pages/command_center/server_intel/
├── direct_connect.rs  the panel header: online indicator, name, address chip with copy, launch
├── mod.rs             the module tree; re-exports `ServerIntelPage`
├── page.rs            the route component: the server list fetch and the one stream subscription
├── player_census.rs   the telemetry grid: population and performance, theatre, environment and mods
├── server_list.rs     the server pick, the frosted panel shell and the intelligence strip
└── tests/             unit tests for the terrain read and the clipboard path of the copy button
```

## How it works

`ServerIntelPage` renders inside `AuthGate`. The signed-in half fetches the server list and
`pick_default` chooses one server: the first active row, else the first row; there is no grid of
servers. For that server's id the page opens the status stream once (the `subscribed` flag, read
untracked, guards it), and registers `abort_server_status_stream` on the page owner, so leaving
the route closes the stream while a re-run of the suspense fragment does not.

```text
GET /api/v1/servers ─► pick_default ─► panel(server, live)
                              │              ├─ panel_header    (direct_connect.rs)
                              │              ├─ telemetry_grid  (player_census.rs)
                              │              └─ intelligence strip
                              └─► stream_server_status(id) ─► live: Option<ServerStatusDto>
```

The panel reads the live frame and falls back to the status the list row carried until the first
frame arrives. A row without a status is a server that has never reported; a status that
`ServerStatusDto` cannot read is audited as a rejected frame. Every readout shows an em dash, not
a zero, when there is no status, and the theatre is named only when the row carries a `terrain`.
The census is an aggregate head count with uptime and frame rate; there is no per-player roster and
no per-faction split. The copy button copies `ip:port` through the crate's one clipboard helper,
which toasts only once the write resolved; the launch button raises the toast "Launch requires the
Reforger client". The intelligence strip at the foot of the panel is fixed placeholder text, not a
feed. The map backdrop and the theatre tile are images on `lh3.googleusercontent.com`.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/server-intel` | `ServerIntelPage` | route tier `none`; the data renders only for a signed-in viewer | full-bleed inside the navigation frame; breadcrumb Command Center › Server Intel |

## Data

- `GET /api/v1/servers`: the server list, read as `DataEnvelope<serde_json::Value>`; the panel
  reads `id`, `is_active`, `name`, `ip`, `port`, `terrain`, `required_modpack` and the cached
  `status`.
- `GET /api/v1/servers/{id}/status/stream`: the chosen server's status stream, opened by
  `stream_server_status` in `apps/website/frontend/src/v2/core/api/sse.rs` as a bearer-authenticated
  fetch; each frame is a `ServerStatusDto`.
- The page reads the session from the `AuthStore` context and writes nothing. The fetch and the
  stream run in the browser build only.

## States

| State | What the viewer sees |
|---|---|
| session restoring | "Loading session…" |
| signed out | "Sign in to load live data from the platform." and a "Sign in with Discord" link to `/login` |
| loading | "Loading…" |
| failed | "Failed to load data." |
| no servers | "No servers configured." |
| loaded | "Server Online" or "Server Offline", the name, the address chip, "LAUNCH & CONNECT", then "Active Personnel", "Uptime:", "Server FPS:" with "Optimal" or "Low", "Theater of Operations" with "Match <id>" or "No Active Mission", "Simulated Time", "Conditions", "Mod Configuration" and "Recent Intelligence" |
| no status yet | "—" in every readout, 0 players |
| no modpack | "No modpack required" |
| address copied | the toast "Server address copied" |

## Boundaries

- Depends on: `crate::v2::core::api` (the `api_get` client, `DataEnvelope`, `ServerStatusDto`, the
  rejected-frame audit, and `sse::stream_server_status` with `sse::abort_server_status_stream`),
  `crate::v2::core::ui` (`AuthGate`, `MaterialIcon`, `cn`, the toasts), `crate::v2::core::utils`
  (uptime formatting and `clipboard::write_clipboard`), the `AuthStore` context, and the images on
  `lh3.googleusercontent.com`.
- Used by: the `/server-intel` route in `apps/website/frontend/src/app_routes.rs`;
  `server_intel_source` in `apps/website/frontend/src/v2/core/test_support/pins.rs`, which the
  stream tests in `apps/website/frontend/src/v2/core/api/tests/sse.rs` also read.
- Rules: at most one stream subscription per mount; the panel reads the `terrain` key the list
  joins in (`server_panel_reads_terrain_key`); the copy button goes through the awaited clipboard
  helper (`class_r_copy_address_routes_through_the_awaited_clipboard_helper`).

## Related documentation

- [Server intel page](/documentation_v2/website/frontend/pages/command_center/server_intel/server_intel_page.md)
  — the page's behaviour and design.
- [Server infrastructure domain](/apps/website/api_v2/src/server_infrastructure/README.md) — the
  server list and status stream routes.
