# My Deployments

## Status

`doc-complete`

## Summary

- **What:** Personal service record: upcoming deployments and attendance history.
- **Why:** Players see ORBAT assignment and past operations.
- **Route:** `/deployments`
- **Live source:** `frontend/src/pages/operations.tsx` (`DeploymentsPage`)
- **Stitch reference:** `frontend/src/stitch-exports/my_deployments_service_record/code.html` (archived)
- **Min role:** `public-nav` (personal data requires auth)
- **Blueprint ref:** [docs/platform/context_handoff.md](../../../docs/platform/context_handoff.md) §4.3 Deployments

## Element Inventory

| # | Element | Type | Text / Content | Purpose | Data source |
|---|---------|------|----------------|---------|-------------|
| 1 | Page H1 | h1 | My Deployments | Title | Static |
| 2 | Stat | span | Total Operations {n} | Career stat | `user.total_deployments` |
| 3 | Stat | span | Attendance Rate {pct}% | Career stat | `user.attendance_rate` |
| 4 | Section H2 | h2 | Awaiting Deployment | Upcoming | Static |
| 5 | Deployment card | card | Op name, date, slot badge | Upcoming event | `GET /me/deployments` upcoming |
| 6 | Countdown | span | Time to event | Urgency | UTC `start_time` |
| 7 | ORBAT badge | span | ASSIGNED SLOT: {slot} | Assignment | `assignment` |
| 8 | LOA button | button | Submit Leave of Absence | LOA request | `POST /me/leave-requests` |
| 9 | Modify button | button | Modify Registration | Change registration | Event register API |
| 10 | Empty state | p | No Further Operations Scheduled | No upcoming | Static |
| 11 | Section H2 | h2 | Service Record | History | Static |
| 12 | Table col | th | Date | History | Static |
| 13 | Table col | th | Operation | History | Static |
| 14 | Table col | th | Role Played | History | Static |
| 15 | Table col | th | Outcome | History | Static |
| 16 | Table col | th | AAR | History | Static |
| 17 | Outcome badge | span | MISSION SUCCESS / FAILED | Result | `match.outcome` |
| 18 | AAR link | link | View Replay | Replay URL | `aar_replay_url` |

## API Dependencies

| Endpoint | Method | When | Response |
|----------|--------|------|----------|
| `GET /me/deployments` | GET | Auth | upcoming + history |
| `POST /me/leave-requests` | POST | LOA submit | `LeaveRequest` |

## Milestones

### M1 — [x] Route `/deployments`
### M2 — [ ] Stitch layout static
### M3 — [ ] `useDeployments()`
### M4 — [ ] LOA + replay links

## Test Plan

1. Guest → static layout + login CTA on actions.
2. Auth → upcoming cards and history table populate.
3. Submit LOA → form validation + API call.

## Open Questions / Blockers

- None
