# Modpacks

## Status

`doc-complete`

## Summary

- **What:** Server modpack dependencies and download/workshop links.
- **Why:** Players sync mods before joining.
- **Route:** `/modpacks`
- **Stitch reference:** `frontend/src/stitch-exports/server_modpacks_deployment_dependencies/code.html`
- **Min role:** `public-nav`
- **Blueprint ref:** [docs/platform/context_handoff.md](../../../docs/platform/context_handoff.md) §4.7 Modpacks

## Element Inventory

| # | Element | Type | Text / Content | Purpose | Data source |
|---|---------|------|----------------|---------|-------------|
| 1 | Page H1 | h1 | Server Modpacks | Title | Static |
| 2 | Card title | h2 | Core Modern Expansion (v2.1) | Modpack name | `Modpack` |
| 3 | Size badge | span | Total Size: 45.2 GB | Size | `modpack.size_bytes` |
| 4 | Mod count badge | span | Mods Included: 32 | Count | `modpack.mod_count` |
| 5 | Section H3 | h3 | Key Dependencies | List header | Static |
| 6 | Dependency row | li | Mod name + check | Each mod | `ModpackMod[]` |
| 7 | Launch btn | button | Launch Game & Auto-Download | Connect | External launcher |
| 8 | Workshop btn | button | View Collection in Reforger Workshop | Workshop | URL |

## API Dependencies

| Endpoint | Method | When | Response |
|----------|--------|------|----------|
| `GET /modpacks` | GET | Auth | `Modpack[]` |
| `GET /modpacks/current` | GET | Auth | current modpack |

## Milestones

### M1 — [x] Route `/modpacks`
### M2 — [ ] Card static
### M3 — [ ] `useModpacks()`
### M4 — [ ] Workshop links

## Test Plan

1. Page renders modpack card.
2. Auth → live mod list.
3. Workshop button opens URL in new tab.

## Open Questions / Blockers

- None
