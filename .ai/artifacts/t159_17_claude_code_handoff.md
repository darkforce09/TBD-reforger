# T-159.17 — Claude Code handoff (yrsPersist + session)

**CWD:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-159`  
**Baseline:** `f2cd6178` / tag `T-159.16`  
**Spec:** [`docs/platform/t159_17_yrs_persist.md`](../../docs/platform/t159_17_yrs_persist.md)

## Context

MissionDocCore hosted in Leptos (.16). Next = durable IDB blob + warm sessionStorage, React
parity (`yrsPersist.ts` / `editorSession.ts`). Mutators still deferred.

## Notes from .16 return

- Prefer commit trailer matching `ebcabe1d` / Opus harness style (not bare noreply).
- If tagging before commit succeeds, delete stale tag and retag at real SHA.

## Return

Tag **T-159.17** + verify log + ready for Cursor doc sync.
