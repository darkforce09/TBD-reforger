# Vehicle Database

## Status

`doc-complete`

## Summary

- **What:** Split-pane vehicle reference with search, faction filters, and per-vehicle dossier.
- **Why:** IFF and armor intel separated from wiki SOPs for faster lookup during ops.
- **Route:** `/vehicles`
- **Live source:** `frontend/src/pages/doctrine.tsx` (`VehicleDatabasePage`)
- **Stitch reference:** `frontend/src/stitch-exports/sop_wiki_vehicle_database_iff/code.html` (archived — vehicles section split out)
- **Min role:** `public-nav`
- **Blueprint ref:** [docs/platform/context_handoff.md](../../../../../docs/platform/context_handoff.md) §4.6

## Element Inventory

| # | Element | Type | Text / Content | Purpose | Data source |
|---|---------|------|----------------|---------|-------------|
| 1 | Master list | `SplitPane` | Search + vehicle rows | Browse/filter | `GET /vehicle-database` |
| 2 | Search input | input | Filter by name… | Client filter | Local |
| 3 | Faction chips | buttons | US / USSR / FIA / … | Faction filter | Vehicle `faction` |
| 4 | Detail dossier | panel | Name, stats, threat, gallery | Selected vehicle | `Vehicle` row |
| 5 | Armor / amphibious | badges | Threat indicators | Quick scan | API fields |
| 6 | Empty state | `SplitPaneEmpty` | Select a vehicle | No selection | Static |

## Behavior

### Primary flow
1. User opens `/vehicles` (nav: Doctrine & Info → Vehicle Database).
2. `SplitPane` master lists vehicles from `useVehicleDatabase()`; detail shows dossier for selection.
3. Search and faction filters narrow the master list client-side.

### States
- **Loading:** `QueryState` skeleton in master column.
- **Empty list:** No vehicles in API → empty copy in master.
- **No selection:** `SplitPaneEmpty` in detail column.

## API Dependencies

| Endpoint | Method | When called | Response shape |
|----------|--------|-------------|----------------|
| `GET /vehicle-database` | GET | Page load | `Vehicle[]` |

## Milestones

### M1 — [x] Route `/vehicles` full-bleed `SplitPane`
### M2 — [x] Master/detail layout (Aegis glass)
### M3 — [x] `useVehicleDatabase()` wired
### M4 — [x] Search + faction filter

## Test Plan

1. Visit `/vehicles` → list populates from API.
2. Select row → detail dossier updates.
3. Faction chip → list filters; search narrows by name.

## Open Questions / Blockers

- None. Wiki vehicle table removed — see [wiki.md](wiki.md) scope note.
