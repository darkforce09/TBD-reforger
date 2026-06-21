# Mission Creator (Setup Wizard)

## Status

`doc-complete`

## Summary

- **What:** Wizard to initialize a new mission before 2D canvas editing.
- **Why:** Authors set terrain, time, weather, and max players before map work.
- **Route:** `/missions/create`
- **Live source:** `frontend/src/pages/missions.tsx` (`MissionCreatorPage`)
- **Stitch reference:** `frontend/src/stitch-exports/mission_creator_setup_wizard/code.html` (archived)
- **Min role:** `mission_maker`
- **Blueprint ref:** [docs/platform/context_handoff.md](../../../docs/platform/context_handoff.md) §3 Mission Library

## Element Inventory

| # | Element | Type | Text / Content | Purpose | Data source |
|---|---------|------|----------------|---------|-------------|
| 1 | Page H1 | h1 | Initialize New Mission | Title | Static |
| 2 | Subtitle | p | Define environment parameters… | Context | Static |
| 3 | Op name label | label | Operation Designation | Form | Static |
| 4 | Op name input | input | Enter operation designation… | Title field | Form state |
| 5 | Terrain label | label | Select Terrain | Form | Static |
| 6 | Terrain card | radio | Everon / Arland | Terrain option | `everon`, `arland` |
| 7 | Game mode | select | PvE Co-op, etc. | Mode | `game_mode` |
| 8 | Time label | label | Insertion Time | Form | Static |
| 9 | Time input | input | HH:MM | Time of day | Form state |
| 10 | Weather label | label | Weather Conditions | Form | Static |
| 11 | Weather select | select | Clear / Overcast / Rain / Fog | Weather | backend `WeatherType` enum |
| 12 | Max players | input | number | Capacity | `max_players` (T-040) |
| 13 | Submit btn | button | Initialize 2D Canvas | Create + open editor | `POST /missions` → `/missions/:id/edit` |

## Behavior

- ProtectedRoute `mission_maker`.
- Submit calls `useCreateMission` with `title`, `terrain`, `game_mode`, `weather`, `time_of_day`, `max_players`.
- On success navigates to `/missions/:id/edit` (T-040).

## API Dependencies

| Endpoint | Method | When | Response |
|----------|--------|------|----------|
| `POST /missions` | POST | Submit wizard | `Mission` |

## Milestones

### M1 — [x] Route protected
### M2 — [x] Wizard form + validation
### M3 — [x] POST /missions on submit
### M4 — [x] Navigate to editor on success (T-040)

## Test Plan

1. Enlisted user → cannot access route.
2. Mission maker → form renders all fields including max players.
3. Submit with valid data → mission created → redirect to `/missions/:id/edit`.

## Open Questions / Blockers

- None. Remaining editor work tracked under [mission-editor.md](mission-editor.md) / [FD-003](../TRACKING.md).
