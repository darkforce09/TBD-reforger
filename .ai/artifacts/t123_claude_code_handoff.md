# T-123 — Claude Code handoff (slices .1–.6)

**Status:** **shipped @ `169e47d`** (historical — superseded by the merged code + the T-123 doc-sync commit; original brief preserved below)  
**Spec:** [`docs/platform/t123_documentation_standards_rollout.md`](../docs/platform/t123_documentation_standards_rollout.md)  
**Authority:** [`docs/platform/DOCUMENTATION_STANDARDS.md`](../docs/platform/DOCUMENTATION_STANDARDS.md)

## Branch

`ticket/T-123` · `./scripts/ticket run` (executes claude-code slices only, in registry order)

## Slice order (all in one ticket — do not split)

| # | Slice | Focus |
|---|-------|-------|
| 1 | **T-123.1** | Go Godoc + `@contract` / `@route` |
| 2 | **T-123.2** | TS TSDoc + `tsdoc.json` + tags |
| 3 | **T-123.3** | Enfusion Backend/Gamemode (enfusion-mcp first) |
| 4 | **T-123.4** | Schema codegen → `internal/contract/` + `frontend/src/types/contract/` + regen script |
| 5 | **T-123.5** | Go `CreateVersion` JSON Schema validation (`mission-editor-payload.schema.json`) |
| 6 | **T-123.6** | CI: revive, eslint jsdoc, `verify-contract-citations.mjs`, schema workflow |

## Verify (replay after each slice)

```bash
make test-it
cd apps/website/frontend && npm run build && npm run lint
go build ./...
cd packages/tbd-schema && npm run validate
```

Mod: Workbench compile on touched `.c` files.

## Return to Cursor

Per-slice verify output → Cursor advances slice + syncs docs. **Do not** edit `docs/` or registry in Claude commits.
