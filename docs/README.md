# TBD Reforger — documentation hub

**Status:** living  
**Audience:** all contributors and AI agents  
**Authority:** Running code → [`CLAUDE.md`](../CLAUDE.md) → this tree  
**Updated:** 2026-06-20

Central index for all human-oriented documentation. Start here.

## Quick links

| Hub | Purpose |
|-----|---------|
| [**Frontend master**](frontend/README.md) | Surface specs, theme, Mission Creator (FE), archived stitch/blueprints |
| [**Backend master**](backend/README.md) | Architecture, runbook, models, handlers, migrations, seeds |
| [**Archive master**](archive/README.md) | Historical HTML mockups and partially stale design docs |
| [**Mission Creator**](../Design_Docs/Mission_Creator_Architecture/README.md) | Editor engineering, Eden parity, agent execution |
| [**TAGS.md**](TAGS.md) | T-0xx vs FD-0xx vs P0–P3 glossary |
| [**CLAUDE.md**](../CLAUDE.md) | Agent runtime: status, commands, doc-on-commit rule |
| [**DEV_RUNBOOK.md**](../DEV_RUNBOOK.md) | Local stack: db-up, api, web, dev-login |

## Role-based entry paths

- **Frontend surface work** → [frontend master](frontend/README.md) → matching [`frontend/docs/pages/`](../frontend/docs/pages/) doc
- **Backend / API change** → [backend master](backend/README.md) → [`internal/models/`](../internal/models/) + handlers
- **Mission Creator agent** → [MC README](../Design_Docs/Mission_Creator_Architecture/README.md) → `05` Decisions log → `02` roadmap → `06` inventory
- **Historical reference only** → [archive master](archive/README.md)

## Authority ladder

1. **Running code** — `router.tsx`, handlers, features, `internal/models/`
2. **[`CLAUDE.md`](../CLAUDE.md)** — status, commands, T-0xx commit tags
3. **[Frontend master](frontend/README.md)** — surface specs (FD-0xx deferred work)
4. **[Backend master](backend/README.md)** — architecture + code links
5. **Mission Creator hub** — Decisions log, roadmap, feature inventory
6. **[Archive master](archive/README.md)** — blueprint HTML, stitch exports (reference only)

> **Live UI:** `frontend/src/pages` + `frontend/src/features`. Do not implement from archived `code.html`.

## Reorganization

See [`REORG_CHANGELOG.md`](REORG_CHANGELOG.md) for old → new path mapping (T-043).
