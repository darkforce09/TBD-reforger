# Mission Approvals

## Status

`doc-complete`

## Summary

- **What:** Admin queue of missions pending approval.
- **Why:** Quality control before missions go live.
- **Route:** `/admin/approvals`
- **Stitch reference:** `frontend/src/stitch-exports/mission_approvals_queue_admin_dashboard/code.html`
- **Min role:** `admin`
- **Blueprint ref:** [docs/platform/context_handoff.md](../../../docs/platform/context_handoff.md) §4.9 Mission Approvals

## Element Inventory

| # | Element | Type | Text / Content | Purpose | Data source |
|---|---------|------|----------------|---------|-------------|
| 1 | Page H1 | h1 | Mission Approvals | Title | Static |
| 2 | Col | th | DATE SUBMITTED | Table | `created_at` local |
| 3 | Col | th | AUTHOR | Table | author name |
| 4 | Col | th | MISSION NAME | Table | title |
| 5 | Col | th | TERRAIN | Table | terrain |
| 6 | Col | th | ACTIONS | Table | Static |
| 7 | Review btn | button | Review File | View JSON | Modal `json_payload` |
| 8 | Approve btn | button | Approve | Approve live | `POST /approvals/:id/approve` |
| 9 | Reject btn | button | Reject | Reject | `POST /approvals/:id/reject` |
| 10 | Pagination | span | Showing x of y pending | Footer | `GET /approvals` |

## API Dependencies

| Endpoint | Method | When | Response |
|----------|--------|------|----------|
| `GET /approvals` | GET | Load queue | pending missions |
| `POST /approvals/:id/approve` | POST | Approve | — |
| `POST /approvals/:id/reject` | POST | Reject | — |

## Milestones

### M1 — [x] Admin route
### M2 — [ ] Table static
### M3 — [ ] `useApprovals()` + actions
### M4 — [ ] JSON review modal

## Test Plan

1. Admin sees pending rows.
2. Approve → row removed from queue.
3. Review opens JSON viewer.

## Open Questions / Blockers

- Stitch says "Review File"; Handoff says "Review JSON" — show JSON.
