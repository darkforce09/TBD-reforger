# Server Intel

## Status

`doc-complete`

## Summary

- **What:** Real-time server health, active mission context, and connection info for **TBD Main**.
- **Why:** Anti-ghosting safe telemetry; players join without tactical leaks.
- **Route:** `/server-intel`
- **Stitch reference:** `frontend/src/stitch-exports/server_intel_dashboard/code.html`
- **Min role:** `public-nav`
- **Blueprint ref:** [docs/platform/context_handoff.md](../../../docs/platform/context_handoff.md) §4.1 Server Intel

## Element Inventory

| # | Element | Type | Text / Content | Purpose | Data source |
|---|---------|------|----------------|---------|-------------|
| 1 | Page H1 | h1 | Server Intel | Title | Static |
| 2 | Subtitle | p | Real-time server health and deployment telemetry. | Context | Static |
| 3 | Card 1 header | h2 | Server Status | Column 1 | Static |
| 4 | Online indicator | dot + span | ONLINE / OFFLINE | Live state | `ServerStatus.is_online` |
| 5 | Uptime label | span | UPTIME | Field label | Static |
| 6 | Uptime value | code | HH:MM:SS | Server uptime | `uptime_seconds` via SSE |
| 7 | Player count label | span | PLAYER COUNT | Field label | Static |
| 8 | Player count | span | {n} / {max} | Population | `player_count`, `max_players` |
| 9 | Player bar | progress | — | Visual fill | Computed ratio |
| 10 | FPS label | span | SERVER PERFORMANCE | Field label | Static |
| 11 | FPS value | span | {fps} Server FPS (Healthy) | Tick health | `server_fps` |
| 12 | Card 2 header | h2 | Active Deployment | Column 2 | Static |
| 13 | Terrain image | img | — | Map thumb | Mission/terrain asset |
| 14 | Sector pill | span | Sector A-4 | Map sector | Static or mission meta |
| 15 | Mission row | li | Mission: {name} | Current mission | Status payload |
| 16 | Time row | li | In-Game Time: {time} | Sim time | `ingame_time` |
| 17 | Weather row | li | Weather: {conditions} | Sim weather | `ingame_weather` |
| 18 | Card 3 header | h2 | Connection Gateway | Column 3 | Static |
| 19 | IP label | span | IP / PORT | Field label | Static |
| 20 | IP value | code | {ip} : {port} | Direct connect | `Server` model |
| 21 | Copy button | button | content_copy | Clipboard | Client action |
| 22 | Modpack label | span | REQUIRED MODPACK | Field label | Static |
| 23 | Modpack value | span | Core v{x} | Version | `GET /modpacks/current` |
| 24 | Verified pill | span | Verified | Modpack OK | Static |
| 25 | Launch CTA | button | Launch Reforger & Connect | Join server | `steam://` or copy |

**Server assumption:** Display name **TBD Main**; ID from `VITE_DEFAULT_SERVER_ID` or first `GET /servers` result (multi-server picker deferred — **T-088**).

## Behavior

### Primary flow
1. Page loads for server "TBD Main".
2. Authenticated: `useServerTelemetry(serverId)` SSE + `useServerStatus` fallback.
3. Guest: static Stitch values.

## API Dependencies

| Endpoint | Method | When called | Response shape |
|----------|--------|-------------|----------------|
| `GET /servers` | GET | Resolve server ID | `Server[]` |
| `GET /servers/:id/status` | GET | Initial snapshot | Server intel DTO |
| `GET /servers/:id/status/stream` | GET (SSE) | Live updates | `ServerStatus` |
| `GET /modpacks/current` | GET | Modpack card | `Modpack` |

## Milestones

### M1 — Shell
- [x] Route `/server-intel`

### M2 — Static Stitch
- [ ] 3-column grid per inventory

### M3 — API wired
- [ ] SSE hook with Bearer auth

### M4 — Complete
- [ ] Copy IP + launch CTA work

## Test Plan

### Manual
1. Visit `/server-intel` → three cards visible.
2. Authenticated with backend → uptime/player count update via SSE.
3. Copy button copies IP:port to clipboard.

### Automated (future)
- `describe('ServerIntel', () => { ... })`

## Open Questions / Blockers

- **T-088** ([ticket registry](../../../docs/TICKET_REGISTRY.md)): Multi-server picker deferred.
