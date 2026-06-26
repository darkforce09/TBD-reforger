# Settings

## Status

`doc-complete`

## Summary

- **What:** User account settings from TopNav dropdown.
- **Why:** Update profile, link Arma identity, manage preferences.
- **Route:** `/settings`
- **Stitch reference:** none
- **Min role:** `enlisted` (auth required for live data)
- **Blueprint ref:** —

## Element Inventory

| # | Element | Type | Text / Content | Purpose | Data source |
|---|---------|------|----------------|---------|-------------|
| 1 | Page H1 | h1 | Settings | Title | Static |
| 2 | Section | h2 | Profile | Section | Static |
| 3 | Avatar | img | Discord avatar | Identity | `user.avatar_url` |
| 4 | Username | p | {username} | Display | `user.username` |
| 5 | Discord handle | p | {handle} | Account | `user.discord_handle` |
| 6 | Role badge | span | {role} | Permission | `user.role` |
| 7 | Section | h2 | Arma Identity | Section | Static |
| 8 | Link status | p | Linked / Unlinked | Status | `user.arma_id` |
| 9 | Link btn | button | Generate Link Code | Start link | `POST /me/link` |
| 10 | Code display | code | 6-digit code | In-game entry | link code response |
| 11 | Unlink btn | button | Unlink Arma ID | Remove link | `DELETE /me/link` |
| 12 | Section | h2 | Service Stats | Section | Static |
| 13 | Deployments stat | p | Total Operations {n} | Stat | `user.total_deployments` |
| 14 | Attendance stat | p | Attendance {pct}% | Stat | `user.attendance_rate` |

## API Dependencies

| Endpoint | Method | When | Response |
|----------|--------|------|----------|
| `GET /me` | GET | Load | `User` |
| `PATCH /me` | PATCH | Update | `User` |
| `POST /me/link` | POST | Link code | code |
| `DELETE /me/link` | DELETE | Unlink | 204 |

## Milestones

### M1 — [x] Route `/settings`
### M2 — [ ] Static profile layout
### M3 — [ ] Live `/me` data
### M4 — [ ] Link code flow

## Test Plan

1. Unauthenticated → login CTA.
2. Authenticated → profile fields populate.
3. Generate link code → 6-digit display.

## Open Questions / Blockers

- None
