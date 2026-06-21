# Event Hub

## Status

`doc-complete`

## Summary

- **What:** Operation container page: hero briefing, countdown, modpack link, and per-mission dossiers with inline ORBAT registration.
- **Why:** Players review multi-mission events and claim slots without leaving the hub.
- **Route:** `/events/:id`
- **Live source:** `frontend/src/pages/events.tsx` (`EventHubPage`, `EventHubView`)
- **Stitch reference:** none (composed from campaign refactor UX)
- **Min role:** `public-nav` (registration requires auth)
- **Blueprint ref:** —

## Element Inventory

| # | Element | Type | Text / Content | Purpose | Data source |
|---|---------|------|----------------|---------|-------------|
| 1 | Back link | link | All Operations | Return to schedule | `/events` |
| 2 | Hero | section | Op name, T-MINUS, briefing, banner | Operation context | `GET /events/:id` (`EventHub`) |
| 3 | TS3 chip | span | ts.tbdevent.eu | Comms | Static |
| 4 | Modpack chip | link | Current modpack name/version | Workshop link | `useCurrentModpack()` |
| 5 | Mission dossier | card | Intel, objectives, armory, ORBAT | Per attached mission | `event.missions[]` |
| 6 | Inline ORBAT | widget | Faction → squad → slot selector | Register / claim | `GET /event-missions/:emid/orbat` |
| 7 | Register btn | button | Register for mission | Per-mission registration | `POST /event-missions/:emid/register` |
| 8 | Squad reserve | buttons | Reserve / release squad | Leader tier | `POST …/squads/reserve` |

## Behavior

### Primary flow
1. User lands on `/events/:id` (from schedule detail, dashboard, or deep link).
2. `useEvent(id)` loads hub with nested mission dossiers.
3. Each dossier renders inline `OrbatSelector` — faction tabs, squad list, slot pick, Register.
4. Leaders+ can reserve squads and assign members via directory search.

### ORBAT deep-link subsection

Standalone split-pane ORBAT selector for bookmarking a single mission:

- **Route:** `/events/:id/missions/:emid/orbat`
- **Live source:** same `OrbatSelector` in `frontend/src/pages/events.tsx`
- **Use when:** Direct link to one mission's slot picker (e.g. Discord pin). Full hub context is optional; page focuses on ORBAT column layout.

### States
- **No missions:** "No missions have been added" under dossiers heading.
- **Registration locked:** Badges and actions reflect `registration_locked` / event status.
- **Embedded:** `EventHubView` also renders inside [event-schedule.md](event-schedule.md) split-pane detail (no back link in embed).

## API Dependencies

| Endpoint | Method | When called | Response shape |
|----------|--------|-------------|----------------|
| `GET /events/:id` | GET | Page load | `EventHub` |
| `GET /event-missions/:emid/orbat` | GET | Per dossier | `OrbatSquad[]` |
| `POST /event-missions/:emid/register` | POST | Register | registration |
| `POST /event-missions/:emid/squads/reserve` | POST | Leader hold | reservation |
| `POST /event-missions/:emid/slots/:slotId/assign` | POST | Assign member | slot |

## Milestones

### M1 — [x] Route `/events/:id`
### M2 — [x] Hero + mission dossiers
### M3 — [x] Inline ORBAT + per-mission register
### M4 — [x] ORBAT deep-link route + leader squad reserve

## Test Plan

1. Open `/events/:id` → hero and dossiers render.
2. Select slot → Register → registration reflected in fill counts.
3. Open `/events/:id/missions/:emid/orbat` → ORBAT selector full-page.
4. Leader reserves squad → others cannot claim held slots.

## Open Questions / Blockers

- Placeholder mission intel (maker, duration, structured objectives) still mocked in dossier UI until API fields exist.
