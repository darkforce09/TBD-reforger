# T-159.20 — Claude Code handoff (save / export)

**CWD:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-159`  
**Baseline:** `f444b878` / tag `T-159.19`  
**Spec:** [`docs/platform/t159_20_save_export.md`](../../docs/platform/t159_20_save_export.md)

## Context

Marquee/move + edit-driven persist done. Next = compile MissionDoc → Export download + Save
Version POST (React `compile.ts` / `useMissionEditor.saveVersion` oracle).

## CI note from .19

Marquee GPU upload panics on software WebGPU — keep `?force=webgl` for that smoke.

## Return

Tag **T-159.20** + verify log → Cursor sets up **T-159.21**.
