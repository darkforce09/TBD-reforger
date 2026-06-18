# Frontend Deferred Work — TRACKING

Open this file anytime to see what is blocked and what remains. Update when blockers are resolved.

| ID | Item | Blocked by | Owner | Doc link | Notes |
|----|------|------------|-------|----------|-------|
| T-001 | **Server Control** (`/admin/server`) | No Stitch export; no admin server/RCON API | Backend + Frontend | [pages/server-control.md](pages/server-control.md) | Sidebar link exists; stub page references this ID |
| ~~T-002~~ | ~~**Discord OAuth redirect**~~ | RESOLVED | — | [auth/auth-callback.md](auth/auth-callback.md) | Backend redirects to `/auth/callback#tokens`; `AuthCallbackPage` now parses the fragment, fetches `/me`, and stores the session |
| T-003 | **2D Mission Editor canvas** | No Stitch export; large feature | Frontend | [pages/mission-creator.md](pages/mission-creator.md), [pages/mission-overview.md](pages/mission-overview.md) | "Initialize 2D Canvas" / "Launch 2D Mission Editor" stubbed |
| T-004 | **Personnel Roster API** | RESOLVED (backend) — `GET /admin/users` exists; frontend hook still stubbed | Frontend | [pages/personnel-roster.md](pages/personnel-roster.md) | Wire `usePersonnel` to the live endpoint |
| T-005 | **Audit Logs API** | RESOLVED (backend) — `GET /admin/audit-logs` (+ `/stream`, `/export.csv`) exist; frontend hook stubbed | Frontend | [pages/audit-logs.md](pages/audit-logs.md) | Wire `useAuditLogs` to the live endpoint |
| ~~T-006~~ | ~~**Backend CORS**~~ | RESOLVED | — | — | `middleware.CORS` wired in `cmd/api/main.go` |
| T-007 | **CMS rich text editor** | WYSIWYG not chosen (Tiptap/Lexical) | Frontend | [pages/content-manager.md](pages/content-manager.md) | Stitch shows toolbar; textarea stub in M2 |
| T-008 | **Wiki markdown renderer** | `react-markdown` not integrated | Frontend | [pages/wiki.md](pages/wiki.md) | Static HTML in M2; markdown in M4 |
| T-009 | **Multi-server picker** | Single server `TBD Main` assumed | Frontend | [pages/server-intel.md](pages/server-intel.md) | `GET /servers` returns many; UI picks first |

## Resolution log

| Date | ID | Resolution |
|------|-----|------------|
| 2026-06-18 | T-002 | `AuthCallbackPage` parses the token fragment, fetches `/me`, stores the session, and redirects to `/`. |
| 2026-06-18 | T-006 | Confirmed `middleware.CORS` is wired in `cmd/api/main.go`. |
| 2026-06-18 | T-004/T-005 | Backend endpoints confirmed present; reassigned to Frontend (wire the stubbed hooks). |
