# Event Schedule

## Status

`doc-complete`

## Summary

- **What:** Split-pane upcoming operations list with embedded Event Hub in the detail column.
- **Why:** Players browse Tuesday ops and register without a full-page navigation away from the schedule.
- **Route:** `/events`
- **Live source:** `apps/website/frontend/src/events.rs` (T-159 Leptos rewrite — React deleted at T-159.29.3)
- **Stitch reference:** `[git history — deleted with the React tree at T-159.29.3] src/stitch-exports/upcoming_operations_event_schedule/code.html` (archived — layout replaced by `SplitPane`)
- **Min role:** `public-nav`
- **Blueprint ref:** —

## Element Inventory

| # | Element | Type | Text / Content | Purpose | Data source |
|---|---------|------|----------------|---------|-------------|
| 1 | Master header | h2 | Upcoming Ops | List title | Static |
| 2 | Op card | button | Date, status badge, name, mission count, fill bar | Select operation | `GET /events?status=upcoming` |
| 3 | Status badge | `Badge` | OPEN / LOCKED / LIVE | Registration state | `event.status`, `registration_locked` |
| 4 | Fill bar | progress | filled/total_slots | Capacity | `EventListItem` |
| 5 | Detail column | panel | Embedded hub | Mission dossiers + ORBAT | `useEvent(selectedId)` → `EventHubView` |
| 6 | Empty master | p | No upcoming operations | No events | Static |
| 7 | Empty detail | `SplitPaneEmpty` | Select an operation… | No selection | Static |

## Behavior

### Primary flow
1. `EventSchedulePage` loads upcoming events via `useEvents('upcoming')`.
2. `SplitPane` master (`masterWidth="24rem"`) lists op cards; first item auto-selected.
3. Clicking a card sets `selectedId`; detail column fetches `useEvent(id)` and renders `EventHubView` (same body as [/events/:id](event-hub.md)).
4. User registers inline via mission dossier ORBAT — no separate "Open ORBAT" step.
5. Full-page hub still available at `/events/:id` for deep links and back-navigation UX.

### States
- **Loading:** `QueryState` wraps entire split-pane.
- **Empty schedule:** Master shows "No upcoming operations scheduled."
- **No selection:** Detail shows calendar `SplitPaneEmpty`.

## API Dependencies

| Endpoint | Method | When | Response |
|----------|--------|------|----------|
| `GET /events` | GET | Auth (`status=upcoming`) | `Event[]` list items |
| `GET /events/:id` | GET | Detail selection | `EventHub` |
| `POST /event-missions/:emid/register` | POST | Inline register | registration |

## Milestones

### M1 — [x] Route `/events`
### M2 — [x] `SplitPane` master list (replaces table/list toggle)
### M3 — [x] `useEvents('upcoming')` + selection state
### M4 — [x] Embedded `EventHubView` + inline ORBAT register flow

## Test Plan

1. Page shows split-pane; upcoming ops in master column.
2. Select op → detail shows hub hero + mission dossiers (not a redirect).
3. Register on a mission dossier → fill bar updates on master card.
4. Navigate to `/events/:id` → same hub content with back link.

## Open Questions / Blockers

- Calendar view deferred; master list is the only view. See [event-hub.md](event-hub.md) for standalone hub and ORBAT deep-link.
