# Frontend documentation — master index

**Status:** living  
**Audience:** frontend developers, UI agents  
**Authority:** Running code → [`CLAUDE.md`](../../CLAUDE.md) → [`frontend/docs/`](../../frontend/docs/)  
**Updated:** 2026-06-20

Single front door for all frontend documentation.

## Living

| Link | Purpose |
|------|---------|
| [`frontend/docs/INDEX.md`](../../frontend/docs/INDEX.md) | Per-route surface specs |
| [`frontend/docs/TRACKING.md`](../../frontend/docs/TRACKING.md) | FD-0xx deferred work |
| [`frontend/docs/THEME.md`](../../frontend/docs/THEME.md) | Aegis tokens in use |
| [`docs/platform/macos_ux_architecture.md`](../platform/macos_ux_architecture.md) | Split-pane / frictionlessness methodology |
| [Mission Creator hub](../../Design_Docs/Mission_Creator_Architecture/README.md) | 2D mission editor |
| [`frontend/src/router.tsx`](../../frontend/src/router.tsx) | Route truth |
| [`frontend/src/pages/`](../../frontend/src/pages/) | Page components |
| [`frontend/src/features/`](../../frontend/src/features/) | Feature modules (tactical-map, mission-creator, …) |
| [`frontend/README.md`](../../frontend/README.md) | Stack, npm commands |

Several operations pages (`/announcements`, `/deployments`, `/leaderboards`, `/events`) share one source file: [`frontend/src/pages/operations.tsx`](../../frontend/src/pages/operations.tsx). See individual page docs for **Live source** lines.

## Archive (reference only)

| Link | Note |
|------|------|
| [`frontend/src/stitch-exports/README.md`](../../frontend/src/stitch-exports/README.md) | Historical Stitch HTML |
| [`Design_Docs/macOS_Blueprints/README.md`](../../Design_Docs/macOS_Blueprints/README.md) | Design-phase blueprint HTML |
| [`Design_Docs/Mission_Creator_Mock_Up/README.md`](../../Design_Docs/Mission_Creator_Mock_Up/README.md) | Product vision mockups |
| [`docs/platform/context_handoff.md`](../platform/context_handoff.md) §4 | UI blueprint prose (partially historical) |

**Live alternative:** implement from `frontend/src/pages` and `frontend/src/features`, not archived HTML.

## Related

- [Platform doc hub](../README.md)
- [Backend master](../backend/README.md)
- [Archive master](../archive/README.md)
