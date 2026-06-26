# Event Manager (Admin)

## Status

`doc-complete`

## Summary

- **What:** Admin calendar + form to schedule Tuesday operations.
- **Why:** Admins publish events linked to mission library.
- **Route:** `/admin/events`
- **Stitch reference:** `frontend/src/stitch-exports/event_manager_admin_scheduling/code.html`
- **Min role:** `admin`
- **Blueprint ref:** [docs/platform/context_handoff.md](../../../docs/platform/context_handoff.md) §4.8 Event Manager

## Element Inventory

| # | Element | Type | Text / Content | Purpose | Data source |
|---|---------|------|----------------|---------|-------------|
| 1 | Page H1 | h1 | Event Manager | Title | Static |
| 2 | Calendar H2 | h2 | {Month Year} | Calendar header | Client date |
| 3 | Calendar nav | button | chevron prev/next | Month change | Client |
| 4 | Day cells | button | 1–31 | Pick date | Client |
| 5 | Event dots | dot | — | Days with events | `GET /events` |
| 6 | Form H3 | h3 | Schedule Operation | Form title | Static |
| 7 | Date field | read-only | Tuesday, October 28 | Selected day | Form |
| 8 | Time field | input | 20:00 | Start time (local display) | Form → UTC |
| 9 | Mission select | select | Operation names | Link mission | `GET /missions` |
| 10 | Registration toggle | toggle | Open / Locked | Registration state | Form |
| 11 | Publish btn | button | Publish Event | Create/update | `POST /events` |
| 12 | Delete btn | button | Delete Event | Remove | `DELETE /events/:id` |

## API Dependencies

| Endpoint | Method | When | Response |
|----------|--------|------|----------|
| `GET /events` | GET | Calendar dots | `Event[]` |
| `POST /events` | POST | Publish | `Event` |
| `PATCH /events/:id` | PATCH | Update | `Event` |
| `DELETE /events/:id` | DELETE | Delete | 204 |

Zod schema: `schemas/event.ts` — `scheduled_at`, `mission_id`, `title`, `registration_open`.

## Milestones

### M1 — [x] Protected admin route
### M2 — [ ] Calendar + form static
### M3 — [ ] react-hook-form + zod + API
### M4 — [ ] Delete + edit existing

## Test Plan

1. Non-admin → redirected.
2. Select calendar day → form date updates.
3. Publish → POST with UTC timestamp.

## Open Questions / Blockers

- None
