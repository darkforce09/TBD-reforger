# Top Navigation Bar

## Status

`doc-complete`

## Summary

- **What:** Fixed 64px header with breadcrumbs and user profile controls.
- **Why:** Shows context (where am I) and account actions (settings, link Arma, sign out).
- **Route:** (shell component)
- **Stitch reference:** `frontend/src/stitch-exports/tbd_reforger_top_navigation_bar/code.html`
- **Min role:** `public-nav`
- **Blueprint ref:** `tbd_reforger_topbar/DESIGN.md`

## Element Inventory

| # | Element | Type | Text / Content | Purpose | Data source |
|---|---------|------|----------------|---------|-------------|
| 1 | Header bar | header | — | 64px fixed, bg-surface, border-b | Static |
| 2 | Breadcrumb parent | span | e.g. Mission Hub | Parent section | Route `handle.breadcrumb.parent` |
| 3 | Breadcrumb separator | span | / | Visual | Static |
| 4 | Breadcrumb current | span | e.g. Operation Enduring Freedom | Current page | Route `handle.breadcrumb.current` |
| 5 | Linked pill | div | Linked: {arma_id truncated} | Arma identity status | `user.arma_id` |
| 6 | Unlinked pill | div | Unlinked | When no Arma ID | Static fallback |
| 7 | Avatar | img | User avatar | Discord avatar | `user.avatar_url` |
| 8 | Username | span | e.g. Admin Dave | Display name | `user.username` |
| 9 | Dropdown chevron | icon | expand_more | Toggle affordance | Static |
| 10 | Dropdown menu | div | — | Profile actions card | Static |
| 11 | Menu item | link | Settings | Account settings | `/settings` |
| 12 | Menu item | link | Link Arma Identity | Start link flow | Modal or `/settings` |
| 13 | Menu divider | hr | — | Visual separation | Static |
| 14 | Menu item | link | Sign Out | Logout | `POST /auth/logout` |

## Behavior

### Primary flow
1. Breadcrumbs update on route change from router handle meta.
2. Profile dropdown toggles on click; closes on outside click.
3. Sign Out clears auth store and calls logout API.

### States
- **Unauthenticated:** Show "Sign in with Discord" button instead of profile group.
- **Linked:** Green muted pill with truncated Steam/Arma ID.
- **Unlinked:** Muted pill text "Unlinked".

## API Dependencies

| Endpoint | Method | When called | Response shape |
|----------|--------|-------------|----------------|
| `GET /me` | GET | On auth | `User` |
| `POST /auth/logout` | POST | Sign out | 204 |

## Milestones

### M1 — Shell
- [x] 64px bar renders; breadcrumbs from route meta
- [x] Dropdown structure present

### M2 — Static Stitch
- [ ] Placeholder user when unauthenticated
- [ ] Pill + avatar match Stitch styling

### M3 — API wired
- [ ] Live user data from `useAuthStore`
- [ ] Logout works

### M4 — Complete
- [ ] Link Arma flow documented in settings

## Test Plan

### Manual
1. Navigate to `/missions` → breadcrumb shows Mission Hub / Mission Library.
2. Open dropdown → Settings, Link Arma, Sign Out visible.
3. Unauthenticated → Discord sign-in CTA shown.

### Automated (future)
- `describe('TopNav', () => { ... })`

## Open Questions / Blockers

- None
