# Content Manager / Comms Broadcaster (Admin CMS)

## Status

`doc-complete`

## Summary

- **What:** CMS for announcements and wiki pages with rich editor.
- **Why:** Admins publish news and update SOPs without code deploys.
- **Route:** `/admin/content` (nav label: **Comms Broadcaster**)
- **Stitch reference:** `frontend/src/stitch-exports/content_manager_admin_cms/code.html`
- **Min role:** `admin`
- **Blueprint ref:** [docs/platform/context_handoff.md](../../../docs/platform/context_handoff.md) §4.11 Content Manager

## Element Inventory

| # | Element | Type | Text / Content | Purpose | Data source |
|---|---------|------|----------------|---------|-------------|
| 1 | Page H1 | h1 | Content Manager | Title | Static |
| 2 | Mode tab | button | Publish News Announcement | Announcement mode | Static |
| 3 | Mode tab | button | Edit SOP Wiki Page | Wiki mode | Static |
| 4 | Title label | label | Post Title | Meta | Form |
| 5 | Title input | input | Operation Thunderstrike Briefing | Title | Form |
| 6 | Category select | select | Event / Modpack Update / Important | Tag | Form |
| 7 | Cover upload | zone | Drag & drop JPG/PNG/WEBP max 5MB | Thumbnail | `POST /cms/uploads` |
| 8 | Editor toolbar | buttons | bold, italic, link, lists, code | WYSIWYG | T-087 |
| 9 | Editor body | textarea | Draft your briefing here... | Content | Form |
| 10 | Discord toggle | switch | Push to Discord Webhook | Webhook | Form |
| 11 | Save draft | button | Save Draft | Draft | Future |
| 12 | Publish btn | button | Publish Content | Submit | `POST /cms/announcements` or `PUT /wiki/:slug` |

Zod schema: `schemas/content.ts`

## API Dependencies

| Endpoint | Method | When | Response |
|----------|--------|------|----------|
| `POST /cms/announcements` | POST | Publish news | `Announcement` |
| `POST /cms/uploads` | POST | Image upload | URL |
| `PUT /wiki/:slug` | PUT | Wiki update | `WikiPage` |

## Milestones

### M1 — [x] Admin route
### M2 — [ ] Form + textarea stub (no WYSIWYG)
### M3 — [ ] zod + announcement POST
### M4 — [ ] WYSIWYG (T-087)

## Test Plan

1. Toggle announcement vs wiki mode → fields change.
2. Publish announcement validates required fields.
3. Discord toggle included in payload.

## Open Questions / Blockers

- **T-087** ([ticket registry](../../../docs/TICKET_REGISTRY.md))
