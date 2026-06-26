# Not Found (404)

## Status

`doc-complete`

## Summary

- **What:** Fallback page for unknown routes.
- **Why:** Clear UX when URL does not exist.
- **Route:** `*` (catch-all)
- **Stitch reference:** none
- **Min role:** `public-nav`
- **Blueprint ref:** —

## Element Inventory

| # | Element | Type | Text / Content | Purpose | Data source |
|---|---------|------|----------------|---------|-------------|
| 1 | Error code | span | 404 | HTTP status | Static |
| 2 | Title | h1 | Sector Not Found | Headline | Static |
| 3 | Message | p | The requested route does not exist in this AO. | Explanation | Static |
| 4 | Home link | link | Return to Dashboard | Navigation | `/` |

## Behavior

Router catch-all renders inside AppLayout (guests can see shell).

## API Dependencies

None.

## Milestones

### M1 — [x] Catch-all route
### M2 — [ ] Styled per THEME.md
### M3 — [ ] N/A
### M4 — [ ] N/A

## Test Plan

1. Visit `/nonexistent` → 404 page inside shell.
2. Click Return to Dashboard → `/`.
3. No console errors.

## Open Questions / Blockers

- None
