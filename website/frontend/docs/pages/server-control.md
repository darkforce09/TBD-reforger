# Server Control (Admin)

## Status

`doc-complete`

## Summary

- **What:** Planned admin panel for game server RCON: restart, change map, status.
- **Why:** [docs/platform/context_handoff.md](../../../docs/platform/context_handoff.md) §2.B requires basic server controls in admin panel.
- **Route:** `/admin/server`
- **Stitch reference:** none (spec from [context handoff](../../../docs/platform/context_handoff.md) §2)
- **Min role:** `admin`
- **Blueprint ref:** [docs/platform/context_handoff.md](../../../docs/platform/context_handoff.md) §2.B RCON

## Element Inventory

| # | Element | Type | Text / Content | Purpose | Data source | Blocked |
|---|---------|------|----------------|---------|-------------|---------|
| 1 | Page H1 | h1 | Server Control | Title | Static | — |
| 2 | Alert banner | div | This feature is not yet available. See T-086 in the ticket registry. | Blocker notice | Static | T-086 |
| 3 | Server name | h2 | TBD Main | Target server | Config | T-086 |
| 4 | Status card | card | Online / Offline | Health | `GET /servers/:id/status` | Partial |
| 5 | Restart btn | button | Restart Server | RCON restart | `POST /admin/server/restart` | T-086 |
| 6 | Change map btn | button | Change Map | RCON mission | `POST /admin/server/map` | T-086 |
| 7 | Map select | select | Mission list | Pick map | `GET /missions` | T-086 |
| 8 | Console log | pre | RCON output stream | Live log | WebSocket/SSE | T-086 |
| 9 | Link | a | View ticket registry | Documentation | [TICKET_LEAD.md](../../../docs/TICKET_LEAD.md) | — |

## Behavior

M1 stub page shows T-086 blocker message and intended layout wireframe. No API calls until backend exists.

## API Dependencies

| Endpoint | Method | When | Response |
|----------|--------|------|----------|
| `POST /admin/server/restart` | POST | Future | — |
| `POST /admin/server/map` | POST | Future | — |

All blocked — see [T-086](../../../docs/TICKET_REGISTRY.md) in the ticket registry.

## Milestones

### M1 — [x] Stub route with T-086 notice
### M2 — [ ] Wireframe UI per inventory
### M3 — [ ] API when backend ready
### M4 — [ ] RCON console stream

## Test Plan

1. Admin navigates to `/admin/server` → stub message visible.
2. Link references T-086 in the ticket registry.
3. Non-admin cannot access.

## Open Questions / Blockers

- **T-086** ([ticket registry](../../../docs/TICKET_REGISTRY.md)): No Stitch, no API, no RCON integration.
