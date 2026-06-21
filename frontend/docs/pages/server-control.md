# Server Control (Admin)

## Status

`doc-complete`

## Summary

- **What:** Planned admin panel for game server RCON: restart, change map, status.
- **Why:** Handoff §2.B requires basic server controls in admin panel.
- **Route:** `/admin/server`
- **Stitch reference:** none (spec from Handoff §2)
- **Min role:** `admin`
- **Blueprint ref:** Handoff §2.B RCON

## Element Inventory

| # | Element | Type | Text / Content | Purpose | Data source | Blocked |
|---|---------|------|----------------|---------|-------------|---------|
| 1 | Page H1 | h1 | Server Control | Title | Static | — |
| 2 | Alert banner | div | This feature is not yet available. See TRACKING FD-001. | Blocker notice | Static | FD-001 |
| 3 | Server name | h2 | TBD Main | Target server | Config | FD-001 |
| 4 | Status card | card | Online / Offline | Health | `GET /servers/:id/status` | Partial |
| 5 | Restart btn | button | Restart Server | RCON restart | `POST /admin/server/restart` | FD-001 |
| 6 | Change map btn | button | Change Map | RCON mission | `POST /admin/server/map` | FD-001 |
| 7 | Map select | select | Mission list | Pick map | `GET /missions` | FD-001 |
| 8 | Console log | pre | RCON output stream | Live log | WebSocket/SSE | FD-001 |
| 9 | Link | a | View TRACKING.md | Documentation | `/docs` ref | — |

## Behavior

M1 stub page shows FD-001 message and intended layout wireframe. No API calls until backend exists.

## API Dependencies

| Endpoint | Method | When | Response |
|----------|--------|------|----------|
| `POST /admin/server/restart` | POST | Future | — |
| `POST /admin/server/map` | POST | Future | — |

All blocked — see [TRACKING.md](../TRACKING.md) FD-001.

## Milestones

### M1 — [x] Stub route with FD-001 notice
### M2 — [ ] Wireframe UI per inventory
### M3 — [ ] API when backend ready
### M4 — [ ] RCON console stream

## Test Plan

1. Admin navigates to `/admin/server` → stub message visible.
2. Link references FD-001 in TRACKING.
3. Non-admin cannot access.

## Open Questions / Blockers

- [FD-001](../TRACKING.md): No Stitch, no API, no RCON integration.
