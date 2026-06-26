# Sidebar Navigation

## Status

`doc-complete`

## Summary

- **What:** Fixed left navigation panel listing all app sections and routes.
- **Why:** Primary way members navigate TBD Reforger; mirrors Stitch ops-center layout.
- **Route:** (shell component — no route)
- **Stitch reference:** `frontend/src/stitch-exports/restructured_operations_sidebar/code.html`
- **Min role:** `public-nav` (admin section hidden unless `admin`)
- **Blueprint ref:** —

## Element Inventory

| # | Element | Type | Text / Content | Purpose | Data source |
|---|---------|------|----------------|---------|-------------|
| 1 | Sidebar container | aside | — | 320px panel, gradient texture | Static |
| 2 | Top accent line | div | — | Primary gradient 2px line | Static |
| 3 | Brand primary | span | TBD | Brand word 1 | Static |
| 4 | Brand secondary | span | Reforger | Brand word 2 | Static |
| 5 | Section header | h3 | Command Center | Nav group label | Static |
| 6 | Nav item | link | Dashboard | Home | `grid_view` | `/` | enlisted |
| 7 | Nav item | link | Server Intel | Live server status | `dns` | `/server-intel` | enlisted |
| 8 | Nav item | link | Announcements | News feed | `campaign` | `/announcements` | enlisted |
| 9 | Section header | h3 | Operations | Nav group | Static |
| 10 | Nav item | link | Event Schedule | Upcoming ops | `calendar_month` | `/events` | enlisted |
| 11 | Nav item | link | My Deployments | Service record | `military_tech` | `/deployments` | enlisted |
| 12 | Nav item | link | Global Leaderboards | Stats | `leaderboard` | `/leaderboards` | enlisted |
| 13 | Section header | h3 | Mission Hub | Nav group | Static |
| 14 | Nav item | link | Mission Library | Mission DB + create (T-048 dialog) | `library_books` | `/missions` | enlisted |
| 15 | Section header | h3 | Field Tools | Nav group | Static |
| 16 | Nav item | link | Mortar Calculator | Ballistics tool | `calculate` | `/tools/mortar` | enlisted |
| 17 | Section header | h3 | Doctrine & Info | Nav group | Static |
| 18 | Nav item | link | SOPs & Manuals | Wiki | `menu_book` | `/wiki` | enlisted |
| 19 | Nav item | link | Modpacks | Dependencies | `extension` | `/modpacks` | enlisted |
| 20 | Admin container | div | — | Red-tinted border box | Static |
| 21 | Section header | h3 | Administration | Admin group | Static |
| 22 | Nav item | link | Event Manager | Schedule ops | `event_available` | `/admin/events` | admin |
| 23 | Nav item | link | Mission Approvals | QC queue | `fact_check` | `/admin/approvals` | admin |
| 24 | Nav item | link | Server Control | RCON panel | `settings_system_daydream` | `/admin/server` | admin |
| 25 | Nav item | link | Personnel Roster | User admin | `groups` | `/admin/personnel` | admin |
| 26 | Nav item | link | Content Manager | CMS | `folder_managed` | `/admin/content` | admin |
| 27 | Nav item | link | Audit Logs | System log | `receipt_long` | `/admin/audit` | admin |
| 28 | Active state | class | nav-item-active | Current route highlight | React Router |
| 29 | Mobile hamburger | button | `menu` | Opens drawer <1280px | Static |

## Behavior

### Primary flow
1. User sees sidebar on desktop (fixed 320px) or hamburger on tablet/mobile.
2. Clicking a link navigates; active item gets `nav-item-active` glow.
3. Mission creation is on Mission Library (**+ New Mission** dialog, T-048) — not a sidebar item.
4. Administration block hidden unless `hasMinRole('admin')`.

### States
- **Unauthenticated:** Full nav visible (browse all routes).
- **Mobile:** Sidebar off-canvas; overlay on open.

## API Dependencies

| Endpoint | Method | When called | Response shape |
|----------|--------|-------------|----------------|
| — | — | — | Nav is static; role from `useAuthStore` |

## Milestones

### M1 — Shell
- [x] Sidebar renders with all sections per inventory
- [x] NavLink active states work
- [x] Role filtering for Administration
- [x] T-048: Mission Creator nav item removed; create on Library

### M2 — Static Stitch
- [ ] Visual match to Stitch (texture, glow, scrollbar)
- [ ] TBD Reforger branding (no version line, no emojis)

### M3 — API wired
- [ ] Role from JWT/user store updates visibility

### M4 — Complete
- [ ] Mobile drawer tested at <1280px

## Test Plan

### Manual
1. Load app → sidebar shows 6 sections; brand reads "TBD Reforger".
2. Click Server Intel → item highlights with primary glow; route changes.
3. Log in as enlisted → Administration hidden; no **+ New Mission** on Library (T-048).

### Automated (future)
- `describe('Sidebar', () => { ... })`

## Open Questions / Blockers

- Server Control nav links to T-086 stub page.
