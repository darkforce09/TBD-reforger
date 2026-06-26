# Auth Callback

## Status

`doc-complete`

## Summary

- **What:** OAuth return URL that parses tokens and establishes session.
- **Why:** Completes Discord login flow in the SPA.
- **Route:** `/auth/callback`
- **Live source:** `frontend/src/pages/auth.tsx` (`AuthCallbackPage`)
- **Stitch reference:** none
- **Min role:** `public-nav`
- **Blueprint ref:** —

## Element Inventory

| # | Element | Type | Text / Content | Purpose | Data source |
|---|---------|------|----------------|---------|-------------|
| 1 | Loading state | div | Completing sign in... | Spinner while parsing | Static |
| 2 | Error title | h1 | Sign in failed | OAuth error | Query/hash params |
| 3 | Error message | p | Dynamic error text | User feedback | API error |
| 4 | Retry button | button | Try again | Re-login | Link `/login` |
| 5 | Success redirect | — | — | Navigate to `/` | After `setSession` |

## Behavior

### Primary flow
1. Backend redirects to `FRONTEND_URL/auth/callback#access_token=...&refresh_token=...`.
2. `AuthCallbackPage` parses hash → `useAuthStore.setSession()`.
3. Fetches `/me` to populate user; redirects to `/` or `returnUrl`.

### States
- **Invalid/missing tokens:** Error UI with retry link to `/login`.
- **Dev-login:** Same callback path — backend 302 with fragment tokens.

## API Dependencies

| Endpoint | Method | When called | Response shape |
|----------|--------|-------------|----------------|
| `GET /me` | GET | After token parse | `User` |
| `POST /auth/refresh` | POST | Later token refresh | tokens |

## Milestones

### M1 — Shell
- [x] Route exists outside AppLayout

### M2 — Static Stitch
- [x] Loading and error UI

### M3 — API wired
- [x] Parse tokens and set session (T-002 shipped)

### M4 — Complete
- [x] Full OAuth round trip + dev-login

## Test Plan

### Manual
1. Visit `/auth/callback` without hash → error + retry.
2. Dev-login redirect → session stored, lands on dashboard.
3. Invalid callback → error + retry link.

### Automated (future)
- `describe('AuthCallback', () => { ... })`

## Open Questions / Blockers

- None (T-002 shipped).
