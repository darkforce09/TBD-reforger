# Documentation tag glossary

Use the correct prefix everywhere. **Do not reuse T-0xx for frontend deferred work.**

| Prefix | Meaning | Where |
|--------|---------|-------|
| **T-0xx** | Platform git milestones (commit history) | [`CLAUDE.md`](../CLAUDE.md) §Status; commit messages e.g. `T-043 …` |
| **FD-0xx** | Frontend **deferred** work (backlog, not shipped) | [`frontend/docs/TRACKING.md`](../frontend/docs/TRACKING.md) |
| **P0–P3** | Eden parity backlog priority | MC `eden/gap_analysis.md` |
| **A / B / C** | Mission Creator functional tracks (map / assets / kits) | MC `02_roadmap.md` |

## T-0xx vs Ultra Plan phases

[`CLAUDE.md`](../CLAUDE.md) T-029–T-040 phase numbers describe **what shipped in git**.  
[`Design_Docs/Mission_Creator_Architecture/03_engineering_ultra_plan.md`](../Design_Docs/Mission_Creator_Architecture/03_engineering_ultra_plan.md) phase numbers describe **engineering design**. They are related but not 1:1 — do not renumber either system.

## Historical commits

Old git commits may say `T-001` in frontend TRACKING context. That meant **FD-001** (deferred). From T-043 onward, frontend backlog uses **FD-0xx** only.
