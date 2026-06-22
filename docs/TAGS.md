# Documentation tag glossary

Use the correct prefix everywhere. **Do not reuse T-0xx for frontend deferred work.**

## Naming convention (T-045)

Every domain has exactly one **`ROADMAP.md`** — the AI/human entry point for planning and doc links.

| Domain | Path |
|--------|------|
| Platform hub | [`docs/README.md`](README.md) → points to domain ROADMAPs |
| Frontend | [`docs/frontend/ROADMAP.md`](frontend/ROADMAP.md) |
| Backend | [`docs/backend/ROADMAP.md`](backend/ROADMAP.md) |
| Mission Creator | [`Design_Docs/Mission_Creator_Architecture/ROADMAP.md`](../Design_Docs/Mission_Creator_Architecture/ROADMAP.md) |

Supporting docs use **descriptive snake_case** filenames (no numeric prefixes): `engineering_plan.md`, `ux_spec.md`, `agent_execution.md`, etc. Old numbered paths redirect via stubs.

## Tag prefixes

| Prefix | Meaning | Where |
|--------|---------|-------|
| **T-0xx** | Platform git milestones | [`CLAUDE.md`](../CLAUDE.md) §Status |
| **T-048** | Library create dialog (shipped) | [`t048_library_create_dialog.md`](../Design_Docs/Mission_Creator_Architecture/t048_library_create_dialog.md) |
| **T-049** | Track A quick P0 (shipped) | [`t049_track_a_quick_p0.md`](../Design_Docs/Mission_Creator_Architecture/t049_track_a_quick_p0.md) |
| **T-050** | Cursor Z readout (shipped) | [`t050_cursor_z_readout.md`](../Design_Docs/Mission_Creator_Architecture/t050_cursor_z_readout.md) |
| **T-052** | Eden P1 undo keyboard shortcuts (shipped) | [`t052_eden_p1_undo_shortcuts.md`](../Design_Docs/Mission_Creator_Architecture/t052_eden_p1_undo_shortcuts.md) |
| **T-053** | Eden P1 Ctrl/Cmd+LMB additive toggle select (shipped) | [`t053_eden_p1_additive_select.md`](../Design_Docs/Mission_Creator_Architecture/t053_eden_p1_additive_select.md) |
| **T-054** | Eden P1 Attributes entry points — map dbl-click + ORBAT tree (shipped) | [`t054_attributes_entry_points.md`](../Design_Docs/Mission_Creator_Architecture/t054_attributes_entry_points.md) |
| **T-055** | Eden P1 asset browser search (shipped) | [`t055_asset_browser_search.md`](../Design_Docs/Mission_Creator_Architecture/t055_asset_browser_search.md) |
| **T-056** | Eden P1 Ctrl+C/V copy-paste at cursor (slots) (shipped) | [`t056_eden_p1_copy_paste.md`](../Design_Docs/Mission_Creator_Architecture/t056_eden_p1_copy_paste.md) |
| **T-051** | Title PATCH sync (**deferred**, not started) | [`t049_track_a_quick_p0.md`](../Design_Docs/Mission_Creator_Architecture/t049_track_a_quick_p0.md) amendment |
| **FD-0xx** | Frontend deferred work | [`frontend/docs/TRACKING.md`](../frontend/docs/TRACKING.md) |
| **BE-0xx** | Backend deferred work | [`docs/backend/ROADMAP.md`](backend/ROADMAP.md) |
| **P0–P3** | Eden parity backlog | MC `eden/gap_analysis.md` |
| **A / B / C** | MC functional tracks | MC [`ROADMAP.md`](../Design_Docs/Mission_Creator_Architecture/ROADMAP.md) |

## T-0xx vs engineering phases

[`CLAUDE.md`](../CLAUDE.md) T-029–T-040 = git milestones.  
[`engineering_plan.md`](../Design_Docs/Mission_Creator_Architecture/engineering_plan.md) phases 0–9 = design doc — not 1:1.

## Historical commits

Frontend TRACKING once used T-0xx for deferred items — those are **FD-0xx** from T-043 onward.
