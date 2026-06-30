# TBD Reforger — Frontend

React 19 + TypeScript + Vite SPA for the TBD Arma Reforger platform. Talks to the
Go API (see [`CLAUDE.md`](../../CLAUDE.md) and [`docs/website/backend/README.md`](../../../docs/website/backend/README.md)).

**Documentation:** per-route specs live under [`docs/website/frontend/`](../../../docs/website/frontend/) — start at [`ROADMAP.md`](../../../docs/website/frontend/ROADMAP.md) and [`INDEX.md`](../../../docs/website/frontend/INDEX.md). Ticket queue: [`docs/TICKET_LEAD.md`](../../../docs/TICKET_LEAD.md).

## Stack
- **React 19** + **Vite** (HMR, `tsc -b && vite build`)
- **TanStack Query** for server state (`src/hooks/queries.ts`, `src/hooks/mutations.ts`)
- **Zustand** for auth/session (`src/store/useAuthStore.ts`, persisted to localStorage)
- **axios** client with a Bearer interceptor + single-flight token refresh (`src/api/`)
- **Tailwind** styling

## Develop
Requires the backend running (see root `Makefile`: `make db-up && make api`).

```bash
npm install
npm run dev      # Vite on http://localhost:5173, proxies /api -> http://localhost:8080
npm run build    # type-check (tsc -b) + production build
npm run lint     # eslint (keep at 0 errors)
```

Config: `VITE_API_URL` (defaults to `/api/v1`), `VITE_DEFAULT_SERVER_NAME`. See
`.env.example`. The dev proxy is in `vite.config.ts`.

### Logging in during development
With the backend in `APP_ENV=development`, open
`http://localhost:5173/api/v1/auth/dev-login?role=admin` — it mints a session and
lands you on the dashboard (no Discord needed). `role` can be `admin`,
`mission_maker`, or `enlisted`.

## Layout
- `src/api/` — axios instance (`client.ts`) and shared `refreshSession` (`refresh.ts`).
- `src/hooks/` — `queries.ts` (reads), `mutations.ts` (writes), `useAuthBootstrap.ts`
  (restores the session from the persisted refresh token on load), `useServerTelemetry.ts` (SSE).
- `src/store/useAuthStore.ts` — tokens + user; persists `refreshToken`, `user`, `expiresAt` only.
- `src/components/` — `AuthGate`/`AdminGate` route guards, `QueryState` (loading/error/empty), layout.
- `src/pages/` — one per route (see `src/router.tsx`).
- `src/types/` — **hand-written** API types; must match the Go JSON (snake_case). Update
  them alongside any backend model change.
