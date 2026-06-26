# App Layout

## Status

`doc-complete`

## Summary

- **What:** Root layout composing Sidebar + TopNav + main content outlet.
- **Why:** Consistent shell for every page; separates chrome from page bodies.
- **Route:** Wraps all routes except `/login` and `/auth/callback`
- **Stitch reference:** Composite of sidebar + topnav exports
- **Min role:** `public-nav`
- **Blueprint ref:** —

## Element Inventory

| # | Element | Type | Text / Content | Purpose | Data source |
|---|---------|------|----------------|---------|-------------|
| 1 | Root flex container | div | — | `min-h-screen bg-background flex` | Static |
| 2 | Sidebar slot | component | Sidebar | Left 320px (desktop) | — |
| 3 | Main column | div | — | `flex-1 flex flex-col min-w-0` | Static |
| 4 | TopNav slot | component | TopNav | Fixed height 64px | — |
| 5 | Main content | main | — | `flex-1 overflow-y-auto p-gutter pt-navbar-height` | Static |
| 6 | Outlet | outlet | — | Page body renders here | React Router |
| 7 | Mobile overlay | div | — | Dim background when drawer open | UI state |
| 8 | Toaster | component | Sonner | Global notifications | — |

## Behavior

### Layout diagram
```text
flex min-h-screen
├── Sidebar (w-80 fixed lg:static)
└── flex-1 flex flex-col
    ├── TopNav (h-16 shrink-0)
    └── main (flex-1 overflow-y-auto)
        └── <Outlet />
```

### States
- **≥1280px:** Sidebar always visible.
- **<1280px:** Sidebar in drawer; hamburger in TopNav or Sidebar.

## API Dependencies

None at layout level.

## Milestones

### M1 — Shell
- [x] Layout wraps router outlet
- [x] Login routes excluded from shell

### M2 — Static Stitch
- [ ] Spacing matches THEME.md (gutter, navbar-height)

### M3 — API wired
- [ ] ErrorBoundary wraps outlet

### M4 — Complete
- [ ] Scroll behavior: sidebar fixed, main scrolls

## Test Plan

### Manual
1. Any member route → sidebar + topnav + page body visible.
2. `/login` → no sidebar (full-page login).
3. Resize to mobile → drawer toggles.

### Automated (future)
- `describe('AppLayout', () => { ... })`

## Open Questions / Blockers

- None
