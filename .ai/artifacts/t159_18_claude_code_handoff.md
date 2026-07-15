# T-159.18 — Claude Code handoff (select / LMB)

**CWD:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-159`  
**Baseline:** `60c6e7ea` / tag `T-159.17`  
**Spec:** [`docs/platform/t159_18_select_tools.md`](../../docs/platform/t159_18_select_tools.md)

## Context

Persist shipped. Next = LMB click-select on seeded slots. Oracle: `useSelectTool` pending-left
+ frozen viewport + `pickNearest`. Rust: `PointIndex` + `OrthoCamera::unproject_xy` on a
**copied** camera at gesture start.

## .17 flag

Debounced IDB writer is not edit-driven yet — add change notify when mutators appear.

## Return

Tag **T-159.18** + verify log + ready for Cursor.
