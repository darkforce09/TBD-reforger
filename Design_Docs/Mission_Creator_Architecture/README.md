# Mission Creator Architecture — hub

**Status:** living  
**Audience:** Mission Creator engineers and agents  
**Authority:** [`05_agent_execution_plan.md`](05_agent_execution_plan.md) Decisions log → [`CLAUDE.md`](../../CLAUDE.md) §Status  
**Updated:** 2026-06-20

Engineering documentation for the 2D Deck.gl mission editor (`/missions/:id/edit`).

## Document map

| File | Role |
|------|------|
| [`05_agent_execution_plan.md`](05_agent_execution_plan.md) | **Execution authority** — phases, Decisions log, agent prompt |
| [`02_roadmap.md`](02_roadmap.md) | Master roadmap — Tracks A/B/C, DONE vs must-work |
| [`03_engineering_ultra_plan.md`](03_engineering_ultra_plan.md) | ADRs, Y.Doc schema, compiler, workers, phases 0–9 |
| [`04_eden_ux_spec.md`](04_eden_ux_spec.md) | Eden docked-shell UX contract |
| [`01_problem_statement.md`](01_problem_statement.md) | Four hard problems (200 slots, DEM, nesting, registry) |
| [`06_tbd_feature_inventory.md`](06_tbd_feature_inventory.md) | Code-evidenced TBD feature inventory |
| [`reference/feds_schema.md`](reference/feds_schema.md) | FEDS v2 feature-entry schema |
| [`eden/interactions.md`](eden/interactions.md) | Eden interaction reference (wiki-anchored) |
| [`eden/ui_anatomy.md`](eden/ui_anatomy.md) | Panel-by-panel Eden UI anatomy |
| [`eden/attributes.md`](eden/attributes.md) | Attribute catalog per entity type |
| [`eden/gap_analysis.md`](eden/gap_analysis.md) | Eden parity gap + P0–P3 backlog |
| [`eden/wiki_manifest.yaml`](eden/wiki_manifest.yaml) | Eden wiki scrape manifest |

Old numbered paths redirect via stubs at previous filenames.

## Code entrypoints

- [`frontend/src/features/mission-creator/`](../../frontend/src/features/mission-creator/)
- [`frontend/src/features/tactical-map/`](../../frontend/src/features/tactical-map/)
- Route: `/missions/:id/edit` in [`frontend/src/router.tsx`](../../frontend/src/router.tsx)

## Related hubs

- [Frontend master](../../docs/frontend/README.md)
- [Platform doc hub](../../docs/README.md)
- [Archive master](../../docs/archive/README.md) — historical mockups
