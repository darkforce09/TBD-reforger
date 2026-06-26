# Global Leaderboards

## Status

`doc-complete`

## Summary

- **What:** Competitive stats rankings with search, category tabs, podium, and table.
- **Why:** Community engagement; telemetry-driven stats.
- **Route:** `/leaderboards`
- **Live source:** `frontend/src/pages/operations.tsx` (`LeaderboardsPage`)
- **Stitch reference:** `frontend/src/stitch-exports/global_leaderboards_player_rankings/code.html` (archived)
- **Min role:** `public-nav`
- **Blueprint ref:** [docs/platform/context_handoff.md](../../../docs/platform/context_handoff.md) §4.4 Leaderboards

## Element Inventory

| # | Element | Type | Text / Content | Purpose | Data source |
|---|---------|------|----------------|---------|-------------|
| 1 | Page H1 | h1 | Global Leaderboards | Title | Static |
| 2 | Search input | input | Search operatives... | Find player | `?q=` param |
| 3 | Tab | button | K/D Ratio | Category | `category=kd` |
| 4 | Tab | button | Command Win Rate | Category | `category=command_win` |
| 5 | Tab | button | Missions Played | Category | `category=missions` |
| 6 | Tab | button | Longest Kill | Category | `category=longest_kill` |
| 7 | Tab | button | Wall of Shame (Team Kills) | Category | `category=team_kills` |
| 8 | Podium slot 2 | card | #2 player | Silver | `data[1]` |
| 9 | Podium slot 1 | card | #1 player | Gold | `data[0]` |
| 10 | Podium slot 3 | card | #3 player | Bronze | `data[2]` |
| 11 | Table col | th | Rank | Rows 4+ | Static |
| 12 | Table col | th | Player | — | Static |
| 13 | Table col | th | Kills | — | Category-dependent |
| 14 | Table col | th | Deaths | — | — |
| 15 | Table col | th | K/D Ratio | — | — |
| 16 | Load more | button | Load More Intel | Pagination | `offset` |

## API Dependencies

| Endpoint | Method | When | Response |
|----------|--------|------|----------|
| `GET /leaderboards?category=&q=&limit=&offset=` | GET | Auth | `{ category, data[] }` |

## Milestones

### M1 — [x] Route `/leaderboards`
### M2 — [ ] Podium + table static
### M3 — [ ] `useLeaderboards(category)`
### M4 — [ ] Search + pagination

## Test Plan

1. Page loads with K/D tab active.
2. Switch tab → category param changes.
3. Search filters results.

## Open Questions / Blockers

- None
