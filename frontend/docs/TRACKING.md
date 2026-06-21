# Frontend Deferred Work — TRACKING

Open this file anytime to see what is blocked and what remains. Update when blockers are resolved.

| ID | Item | Blocked by | Owner | Doc link | Notes |
|----|------|------------|-------|----------|-------|
| FD-001 | **Server Control** (`/admin/server`) | No Stitch export; no admin server/RCON API | Backend + Frontend | [pages/server-control.md](pages/server-control.md) | Sidebar link exists; stub page references this ID |
| ~~FD-002~~ | ~~**Discord OAuth redirect**~~ | RESOLVED | — | [auth/auth-callback.md](auth/auth-callback.md) | Backend redirects to `/auth/callback#tokens`; `AuthCallbackPage` parses the fragment, fetches `/me`, and stores the session |
| FD-003 | **2D Mission Editor** (`/missions/:id/edit`) | Phases 2/5/6/8 blocked on DEM, asset registry API, tools | Frontend | [pages/mission-editor.md](pages/mission-editor.md), [pages/mission-creator.md](pages/mission-creator.md) | Eden docked shell shipped (T-029..T-040); DEM, registry worker, ruler/LoS remain |
| ~~FD-004~~ | ~~**Personnel Roster API**~~ | RESOLVED | — | [pages/personnel-roster.md](pages/personnel-roster.md) | `usePersonnel` wired to `GET /admin/users` in `admin.tsx` |
| ~~FD-005~~ | ~~**Audit Logs API**~~ | RESOLVED | — | [pages/audit-logs.md](pages/audit-logs.md) | `useAuditLogs` wired to `GET /admin/audit-logs` in `admin.tsx` |
| ~~FD-006~~ | ~~**Backend CORS**~~ | RESOLVED | — | — | `middleware.CORS` wired in `cmd/api/main.go` |
| FD-007 | **CMS rich text editor** | WYSIWYG not chosen (Tiptap/Lexical) | Frontend | [pages/content-manager.md](pages/content-manager.md) | Stitch shows toolbar; textarea stub in M2 |
| FD-008 | **Wiki markdown renderer** | `react-markdown` not integrated | Frontend | [pages/wiki.md](pages/wiki.md) | Static HTML in M2; markdown in M4 |
| FD-009 | **Multi-server picker** | Single server `TBD Main` assumed | Frontend | [pages/server-intel.md](pages/server-intel.md) | `GET /servers` returns many; UI picks first |

## Resolution log

| Date | ID | Resolution |
|------|-----|------------|
| 2026-06-18 | FD-002 | `AuthCallbackPage` parses the token fragment, fetches `/me`, stores the session, and redirects to `/`. |
| 2026-06-18 | FD-006 | Confirmed `middleware.CORS` is wired in `cmd/api/main.go`. |
| 2026-06-18 | FD-004/FD-005 | Backend endpoints confirmed present; reassigned to Frontend (wire the stubbed hooks). |
| 2026-06-21 | FD-004 | `usePersonnel` live in `admin.tsx` Personnel roster split-pane. |
| 2026-06-21 | FD-005 | `useAuditLogs` live in `admin.tsx` Audit Logs slide-over. |
