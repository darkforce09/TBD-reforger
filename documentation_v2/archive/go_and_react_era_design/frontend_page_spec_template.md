**Status:** archived — see [feature doc template](/documentation_v2/standards/templates/feature_doc.md)

# [Surface Name]

## Status

`doc-pending` | `doc-complete` | `shell-only` | `static-stitch` | `api-wired` | `complete`

## Summary

- **What:** [One sentence]
- **Why:** [Business reason]
- **Route:** `/path` or `(shell component)`
- **Live source:** `apps/website/frontend/src/<module>.rs`
- **Stitch reference:** `[git history] src/stitch-exports/<name>/code.html` — archive tier
- **Min role:** `enlisted` | `mission_maker` | `admin` | `public-nav`
- **Blueprint ref:** [docs/platform/context_handoff.md](/documentation_v2/archive/go_and_react_era_design/platform_context_handoff.md) §4.x (if applicable)

**Doc hub:** [docs/website/frontend/README.md](/documentation_v2/website/frontend/README.md)

## Element Inventory

| # | Element | Type | Text / Content | Purpose | Data source |
|---|---------|------|----------------|---------|-------------|
| 1 | | | | | |

## Behavior

### Primary flow
1.

### States
- **Unauthenticated:** Static/Stitch layout; live data shows login CTA; mutations disabled
- **Loading:** Skeleton or spinner per THEME.md
- **Empty:** Copy and optional action
- **Error:** Toast + retry
- **Role insufficient:** Redirect or toast per auth rules

## API Dependencies

| Endpoint | Method | When called | Response shape |
|----------|--------|-------------|----------------|
| | | | |

## Milestones

### M1 — Shell
- [ ] Route resolves
- [ ] Placeholder or layout shell renders inside AppLayout
- [ ] Breadcrumb meta set

### M2 — Static Stitch
- [ ] All inventory elements rendered with placeholder data
- [ ] Matches THEME.md tokens (primary `#3b82f6`)

### M3 — API wired
- [ ] React Query hook connected (`enabled: isAuthenticated`)
- [ ] Loading/error states
- [ ] Auth gate on mutations

### M4 — Complete
- [ ] All interactions work
- [ ] Edge cases handled
- [ ] Test plan passes

## Test Plan

### Manual
1.
2.
3.

### Automated (future)
- `describe('SurfaceName', () => { ... })`

## Open Questions / Blockers

- None, or link to [TICKET_LEAD.md](https://github.com/darkforce09/TBD-reforger/blob/2574b0ed2f76edb131447eb10e3d55446404d785/docs/TICKET_LEAD.md) / [TICKET_REGISTRY.md](https://github.com/darkforce09/TBD-reforger/blob/2574b0ed2f76edb131447eb10e3d55446404d785/docs/TICKET_REGISTRY.md) (T-0xx only)
