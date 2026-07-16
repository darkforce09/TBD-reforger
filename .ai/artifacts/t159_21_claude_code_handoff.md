# T-159.21 — Claude Code handoff (Eden chrome + undo)

**CWD:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-159`  
**Baseline:** `c0e11d54` / tag `T-159.20`  
**Spec:** [`docs/platform/t159_21_eden_chrome_undo.md`](../../docs/platform/t159_21_eden_chrome_undo.md)

## Context

Save/Export shipped. Next = docked Eden chrome scaffold + MissionDoc undo/redo (React
`undo.ts` / TopCommandStrip). Outliner/palette data = .22.

## Notes

- Soft-WebGPU marquee smoke stays `?force=webgl`.
- Don’t chase pre-existing website-leptos clippy drift outside touched files.

## Return

Tag **T-159.21** + verify log → Cursor sets up **T-159.22**.
