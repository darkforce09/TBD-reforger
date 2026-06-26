# Personnel Roster (Admin)

## Status

`doc-complete`

## Summary

- **What:** Admin user management table with search, edit, ban.
- **Why:** Manage Discord/Arma linked accounts, roles, warnings.
- **Route:** `/admin/personnel`
- **Live source:** `frontend/src/pages/admin.tsx` (`PersonnelPage`); data via `usePersonnel`
- **Stitch reference:** `frontend/src/stitch-exports/personnel_roster_admin_management/code.html` (archived)
- **Min role:** `admin`
- **Blueprint ref:** [docs/platform/context_handoff.md](../../../docs/platform/context_handoff.md) §4.10

## Element Inventory

| # | Element | Type | Text / Content | Purpose | Data source |
|---|---------|------|----------------|---------|-------------|
| 1 | Page H1 | h1 | Personnel Roster | Title | Static |
| 2 | Search input | input | Search Discord ID or Arma Name... | Filter | `?q=` |
| 3 | Col | th | Discord Account | Table | `discord_handle` |
| 4 | Col | th | Arma Character | Table | `arma_character` or Unlinked |
| 5 | Col | th | Rank/Role | Table | `role` badge |
| 6 | Col | th | Warnings | Table | warning count |
| 7 | Col | th | Actions | Table | Static |
| 8 | Edit btn | button | Edit User | Edit modal | Future PATCH |
| 9 | Ban btn | button | Ban | Ban user | Future POST ban |
| 10 | Role badge | span | Platform Admin / Enlisted | Visual | `user.role` |
| 11 | Pagination | span | Showing x of y entries | Footer | offset |

## API Dependencies

| Endpoint | Method | When | Response |
|----------|--------|------|----------|
| `GET /admin/users` | GET | Load roster | `User[]` via `usePersonnel(q)` |

## Milestones

### M1 — [x] Admin route
### M2 — [x] Table layout (wide roster + slide-over dossier)
### M3 — [x] `usePersonnel` live (shipped)
### M4 — [ ] Edit/ban when API exists

## Test Plan

1. Admin page renders table from API.
2. Search input passes `q` to `usePersonnel`.
3. Row selection opens dossier slide-over.

## Open Questions / Blockers

- Edit/ban mutations not yet implemented. API read path shipped (T-004 admin wiring).
