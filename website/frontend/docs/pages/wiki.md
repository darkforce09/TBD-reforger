# Wiki (SOPs & Manuals)

## Status

`doc-complete`

## Summary

- **What:** Two-column wiki with category nav and markdown content.
- **Why:** Community doctrine, SOPs, IFF, medical flowcharts.
- **Scope note:** Vehicle reference moved to [/vehicles](vehicle-database.md) (`VehicleDatabasePage` in `doctrine.tsx`). This page covers wiki SOPs only — no vehicle table.
- **Route:** `/wiki` and `/wiki/:slug`
- **Stitch reference:** `frontend/src/stitch-exports/sop_wiki_vehicle_database_iff/code.html`
- **Min role:** `public-nav`
- **Blueprint ref:** [docs/platform/context_handoff.md](../../../docs/platform/context_handoff.md) §4.6 Wiki

## Element Inventory

| # | Element | Type | Text / Content | Purpose | Data source |
|---|---------|------|----------------|---------|-------------|
| 1 | Nav H2 | h2 | SOP Categories | Sidebar title | Static |
| 2 | Nav link | link | Rules of Engagement (ROE) | Category | `GET /wiki` slug `roe` |
| 3 | Nav link | link | Radio Protocol & Comms | Category | slug `radio` |
| 4 | Nav link | link | Medical Flowchart ACE | Category | slug `medical` |
| 5 | Nav link | link | (vehicle DB nav removed) | See `/vehicles` | — |
| 6 | Nav link | link | Infantry Spacing & Formations | Category | slug `formations` |
| 7 | Content H1 | h1 | {WikiPage.title} | Page title | `WikiPage.title` |
| 8 | Warning callout | div | CRITICAL IFF Directive | Safety | `WikiPage` body |
| 9 | — | — | Vehicles not on wiki | Moved to vehicle-database.md | — |
| 10 | Gallery section | h4 | Visual ID Guide Gallery | Images | Static/ future |
| 11 | Gallery btn | button | View Full Gallery | Expand | Client |

## API Dependencies

| Endpoint | Method | When | Response |
|----------|--------|------|----------|
| `GET /wiki` | GET | Auth | nav list |
| `GET /wiki/:slug` | GET | Auth | `WikiPage` |

## Milestones

### M1 — [x] Routes `/wiki`, `/wiki/:slug`
### M2 — [ ] Stitch layout static HTML content
### M3 — [ ] `useWiki(slug)`
### M4 — [ ] Markdown render (T-085)

## Test Plan

1. `/wiki` → nav + default content.
2. Click ROE → slug changes content area.
3. Vehicle lookup uses `/vehicles`, not wiki.

## Open Questions / Blockers

- **T-085** ([ticket registry](../../../docs/TICKET_REGISTRY.md)): react-markdown for M4.
