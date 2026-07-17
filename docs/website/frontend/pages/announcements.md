# Announcements (News Feed)

## Status

`doc-complete`

## Summary

- **What:** Blog-style feed of community news and patch notes.
- **Why:** Central place for updates; complements Discord webhooks.
- **Route:** `/announcements`
- **Live source:** `apps/website-leptos/src/announcements.rs` (T-159 Leptos rewrite — React deleted at T-159.29.3)
- **Stitch reference:** `[git history — deleted with the React tree at T-159.29.3] src/stitch-exports/command_announcements_feed/code.html`
- **Min role:** `public-nav`
- **Blueprint ref:** [docs/platform/context_handoff.md](../../../website/platform/context_handoff.md) §4.2 Announcements

## Element Inventory

| # | Element | Type | Text / Content | Purpose | Data source |
|---|---------|------|----------------|---------|-------------|
| 1 | Page H1 | h1 | Command Announcements | Title | Static |
| 2 | News card | article | Thumbnail + meta | Each announcement | `GET /announcements` |
| 3 | Pinned badge | span | PINNED | Pinned post | `announcement.pinned` |
| 4 | Category pill | span | MODPACK UPDATE / EVENT | Tag | `announcement.tag` |
| 5 | Title | h2 | {title} | Headline | `announcement.title` |
| 6 | Meta line | span | {author} • {date} | Byline | `author`, `published_at` UTC→local |
| 7 | Snippet | p | Excerpt | Preview text | `announcement.summary` |
| 8 | Read button | button | Read Full Briefing | Detail view | Navigate `/announcements/:id` |
| 9 | Author icon | icon | account_circle | Visual | Static |

## Behavior

- Guest: static Stitch cards.
- Auth: `useAnnouncements()` loads list.
- Detail route optional future; button may expand or navigate.

## API Dependencies

| Endpoint | Method | When | Response |
|----------|--------|------|----------|
| `GET /announcements` | GET | Authenticated | `Announcement[]` |
| `GET /announcements/:id` | GET | Detail | `Announcement` |

## Milestones

### M1 — Shell — [x] Route `/announcements`
### M2 — Static Stitch — [ ] Card stack from Stitch
### M3 — API wired — [ ] Hook + loading
### M4 — Complete — [ ] Detail view

## Test Plan

1. Visit `/announcements` → cards render.
2. Authenticated → list from API.
3. Click Read Full Briefing → detail or expand.

## Open Questions / Blockers

- None
