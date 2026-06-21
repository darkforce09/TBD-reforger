# Mission Library

## Status

`doc-complete`

## Summary

- **What:** Searchable grid of community missions with filters and tabs.
- **Why:** Browse and open missions before events or authoring.
- **Route:** `/missions`
- **Stitch reference:** `frontend/src/stitch-exports/mission_library_tactical_browser/code.html`
- **Min role:** `public-nav`
- **Blueprint ref:** §4.5

## Element Inventory

| # | Element | Type | Text / Content | Purpose | Data source |
|---|---------|------|----------------|---------|-------------|
| 1 | Page H1 | h1 | Mission Library | Title | Static |
| 2 | Tab | button | Global Missions | Filter scope | `GET /missions` |
| 3 | Tab | button | My Missions | Author filter | `author=me` |
| 4 | Tab | button | Bookmarked | Bookmarks | bookmark filter |
| 5 | Search label | label | Search Operations | Filter | `?q=` |
| 6 | Search input | input | — | Text search | API |
| 7 | Terrain select | select | All / Everon / Arland / Custom | Filter | `terrain` |
| 8 | Mode select | select | All / PvE / PvP / Zeus | Filter | `game_mode` |
| 9 | Players select | select | Player count ranges | Filter | `max_players` |
| 10 | Mission card | card | Thumb, title, author | Each mission | `Mission` |
| 11 | Mode badge | span | [PvE/COOP] etc. | Game mode | `mission.game_mode` |
| 12 | CTA | button | View Operation Intel | Open overview | `/missions/:id` |

## API Dependencies

| Endpoint | Method | When | Response |
|----------|--------|------|----------|
| `GET /missions` | GET | Auth | `Mission[]` |

## Milestones

### M1 — [x] Route `/missions`
### M2 — [ ] Filters + card grid static
### M3 — [ ] `useMissions(filters)`
### M4 — [ ] Tabs + bookmark filter

## Test Plan

1. Visit `/missions` → grid with 3 sample cards (static).
2. Change terrain filter → query updates.
3. Click View Operation Intel → `/missions/:id`.

## Open Questions / Blockers

- Handoff mentions Faction filter; Stitch uses Mode — use Stitch.
