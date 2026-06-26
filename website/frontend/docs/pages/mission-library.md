# Mission Library

## Status

`doc-complete` — T-048 shipped: embedded create dialog + header button / empty-state CTA / Cmd-Ctrl+N; `/missions/create` route removed.

## Summary

- **What:** Searchable grid of community missions with filters, tabs, dossier Sheet, and **mission creation** (T-048).
- **Why:** Browse, create, and open missions before events or 2D editing — single Mission Hub surface.
- **Route:** `/missions`
- **Live source:** `frontend/src/pages/missions.tsx` (`MissionLibraryPage`); create dialog: `frontend/src/features/mission-creator/CreateMissionDialog.tsx` (T-048)
- **Stitch reference:** `frontend/src/stitch-exports/mission_library_tactical_browser/code.html` (archived)
- **Min role:** `enlisted` (browse); `mission_maker` (+ New Mission button)
- **Blueprint ref:** [docs/platform/context_handoff.md](../../../docs/platform/context_handoff.md) §4.5 Mission Library
- **Implementation spec:** [t048_library_create_dialog.md](../../../docs/specs/Mission_Creator_Architecture/t048_library_create_dialog.md)

## Element Inventory

| # | Element | Type | Text / Content | Purpose | Data source |
|---|---------|------|----------------|---------|-------------|
| 1 | Page H1 | h1 | Mission Library | Title | Static |
| 2 | New Mission btn | button | New Mission (+ add icon) | Open create dialog | Static; `mission_maker+` only (T-048) |
| 2b | Empty state CTA | button | Create mission (or similar) | My Missions tab when list empty | `mission_maker+` only (T-048) |
| 3 | Tab | button | Global Missions | Filter scope | `GET /missions` |
| 4 | Tab | button | My Missions | Author filter | `scope=mine` |
| 5 | Tab | button | Bookmarked | Bookmarks | `scope=bookmarked` |
| 6 | Search input | input | — | Text search | `?q=` |
| 7 | Terrain select | select | All / Everon / Arland | Filter | `terrain` |
| 8 | Mode select | select | All / PvE / PvP / Zeus | Filter | `game_mode` |
| 9 | Players select | select | Player count ranges | Filter | `player_count` |
| 10 | Featured hero | section | Newest global mission | Spotlight | `useMissions('global')` |
| 11 | Mission card | card | Thumb, title, author | Each mission | `Mission` |
| 12 | Mode badge | span | PvE/COOP etc. | Game mode | `mission.game_mode` |
| 13 | Status badge | Badge | Draft / Live / Open for review | Visibility | `mission.status` |
| 14 | Card click | — | — | Open dossier Sheet | `previewId` state |
| 15 | Dossier Sheet | Sheet | Mission dossier slide-over | Preview without route change | `useMission(id)` |
| 16 | Dossier CTA | button | OPEN IN MISSION CREATOR | Edit existing | `/missions/:id/edit` (owner/admin + maker) |

### Create Mission Dialog (T-048 — embedded modal)

| # | Element | Type | Text / Content | Purpose | Data source |
|---|---------|------|----------------|---------|-------------|
| D1 | Dialog title | h2 | New Mission | Modal header | Static |
| D2 | Description | p | Define terrain and environment… | Context | Static |
| D3 | Title input | input | Operation designation | Mission title | Form state |
| D4 | Terrain tiles | button×2 | Everon / Arland | Terrain pick | `everon`, `arland` |
| D5 | Game mode | select | Co-op PvE / PvP / Zeus | Mode | `game_mode` |
| D6 | Time | input time | HH:MM | Time of day | Form state |
| D7 | Weather | select | Clear / Overcast / Rain / Fog | Weather | backend enum |
| D8 | Max players | select | 16–128 | Capacity | `max_players` |
| D9 | Submit | button | Create Mission Draft | Create + open editor | `POST /missions` → `/missions/:id/edit` |

## Behavior

### Primary flow — browse
1. User opens `/missions` → scope tabs + filters + grid.
2. Click card → right Sheet dossier (no full-page navigation).
3. Maker with edit rights → **OPEN IN MISSION CREATOR** → `/missions/:id/edit`.

### Primary flow — create (T-048)
1. `mission_maker+` clicks **New Mission** in header **or** empty-state CTA on **My Missions** **or** **Cmd/Ctrl+N**.
2. Frosted `CreateMissionDialog` opens over library (macOS context retention).
3. Submit → `POST /missions` → toast → navigate to `/missions/:id/edit`.
4. Close without submit → form resets on next open.
5. Enlisted users: no create button, shortcut, or empty-state CTA.

### States
- **Unauthenticated:** AuthGate blocks page.
- **Loading / error:** QueryState on grid and dossier.
- **Create pending:** Submit disabled, "Creating…".

## API Dependencies

| Endpoint | Method | When | Response |
|----------|--------|------|----------|
| `GET /missions` | GET | Page load / filter change | `{ data, total, … }` |
| `GET /missions/:id` | GET | Dossier open | `MissionDetail` |
| `POST /missions` | POST | Create dialog submit | `{ id, … }` |

## Milestones

### M1 — [x] Route `/missions` + grid + Sheet dossier
### M2 — [x] Filters, tabs, featured hero (live API)
### M3 — [x] T-048 CreateMissionDialog + remove `/missions/create`
### M4 — [x] Surface spec `doc-complete` after T-048 ship

## Test Plan

1. `mission_maker` → `/missions` → **+ New Mission** → fill form → lands on editor.
2. `enlisted` → no create button.
3. `/missions/create` → 404 after T-048.
4. Sidebar Mission Hub → Mission Library only.
5. Dossier edit CTA still opens editor for owned missions.

## Open Questions / Blockers

- None for T-048. Mission Creator queue: [MC ROADMAP](../../../docs/specs/Mission_Creator_Architecture/ROADMAP.md) · [TICKET_LEAD.md](../../../docs/TICKET_LEAD.md).
