# Dashboard (Home)

## Status

`doc-complete`

## Summary

- **What:** Home page with next operation hero, quick intel cards, and recent announcements.
- **Why:** Command center entry point; surfaces countdown, server status, assignment, and news.
- **Route:** `/`
- **Stitch reference:** `[git history — deleted with the React tree at T-159.29.3] src/stitch-exports/tbd_reforger_dashboard_content/code.html`
- **Min role:** `public-nav` (live data requires auth)
- **Blueprint ref:** —

## Element Inventory

| # | Element | Type | Text / Content | Purpose | Data source |
|---|---------|------|----------------|---------|-------------|
| 1 | Hero section | section | — | Atmospheric gradient banner | Static |
| 2 | Terrain pill | span | e.g. Everon | Next event terrain | `GET /dashboard` → `next_event.terrain` |
| 3 | Countdown | div | T-MINUS HH:MM:SS | Time to next op | UTC `next_event.start_time` → local countdown |
| 4 | Operation subtitle | p | {name} - {local datetime} | Event title + time | `next_event` |
| 5 | Register button | button | Register for Deployment | Register for event | `POST /events/:id/register` |
| 6 | Intel card 1 icon | icon | dns | Server status | Static |
| 7 | Intel card 1 title | h3 | Online / Offline | Server state | `dashboard.server_status` |
| 8 | Intel card 1 status | span | STATUS: ACTIVE | Health label | `server_status.is_online` |
| 9 | Intel card 1 detail | p | {count}/{max} Players ({map}) | Player count | `server_status` |
| 10 | Intel card 2 icon | icon | package_2 | Modpack | Static |
| 11 | Intel card 2 title | h3 | Core Modpack v{x} | Modpack name | `current_modpack` |
| 12 | Intel card 2 status | span | SYNCED | Sync state | Static |
| 13 | Intel card 2 detail | p | {size} GB Total | Size | `current_modpack` |
| 14 | Intel card 3 icon | icon | shield | Assignment | Static |
| 15 | Intel card 3 title | h3 | Deployed / Unassigned | Assignment state | `my_assignment` |
| 16 | Intel card 3 status | span | ASSIGNMENT | Label | Static |
| 17 | Intel card 3 detail | p | {faction} - {squad} - {role} | ORBAT slot | `my_assignment` |
| 18 | Section heading | h2 | Recent Announcements | News header | Static |
| 19 | Announcement row | div | Date pill + title + snippet | News preview | `recent_announcements[]` |
| 20 | Announcement link | — | Click row | Go to `/announcements/:id` | `announcement.id` |

## Behavior

### Primary flow
1. Guest sees static Stitch layout with placeholder data.
2. Authenticated user: `useDashboard()` populates hero, cards, announcements.
3. Register button: if not logged in → `/login`; else register for `next_event`.

### States
- **Unauthenticated:** Static placeholders; Register prompts login.
- **No next event:** Countdown shows "No upcoming operations".
- **Loading:** Skeleton cards.

## API Dependencies

| Endpoint | Method | When called | Response shape |
|----------|--------|-------------|----------------|
| `GET /dashboard` | GET | When authenticated | `DashboardResponse` |

## Milestones

### M1 — Shell
- [x] Route `/` in AppLayout

### M2 — Static Stitch
- [x] Full hero + 3 intel cards + 3 announcement rows from Stitch HTML

### M3 — API wired
- [ ] `useDashboard()` with auth gate

### M4 — Complete
- [ ] Live countdown from UTC; register flow works

## Test Plan

### Manual
1. Visit `/` unauthenticated → hero and cards visible with placeholder text.
2. Visit authenticated → data loads from API (when backend populated).
3. Click Register without login → redirected to login.

### Automated (future)
- `describe('Dashboard', () => { ... })`

## Open Questions / Blockers

- None
