# T-159 — Leptos UI rewrite program

**Status:** program hub · **ACTIVE:** **T-159.18** (select / LMB tools) · **Latest:**
**T-159.17** @ `60c6e7ea` (tag **T-159.17**) · **Worktree:**
`.ai/artifacts/worktrees/TBD-T-159/` @ `t-159-leptos-ui`

## Progress (tip `60c6e7ea`)

| Milestone | Status |
|-----------|--------|
| 24 pages + T-159.15.x camera | shipped |
| **T-159.16** MissionDocCore host | `f2cd6178` |
| **T-159.17** yrsPersist + warm session | `60c6e7ea` |
| **T-159.18** LMB select / pick foundation | **ACTIVE** |
| .19+ save / outliner / Arsenal / cutover | queued |

### Verify logs

- [`.ai/artifacts/t159_16_verify_log.md`](../../.ai/artifacts/t159_16_verify_log.md)
- [`.ai/artifacts/t159_17_verify_log.md`](../../.ai/artifacts/t159_17_verify_log.md) — semantic digest Class R (not raw encode bytes); **note:** debounced writer not yet edit-driven (no mutator change hook) → lands with mutators in .18+

### Locked carry-forward

MissionDoc = `MissionDocCore` same wasm. Persist DB `tbd-mission-yrs`. Pan = `engine.pan`. No
`unproject_xy` on RenderEngine (X-05) — LMB pick uses a **frozen** ortho viewport at gesture
start (React `useSelectTool` pattern). No GpuTimer (T-160).

## Slice index

| Slice | Status |
|-------|--------|
| **T-159.17** | shipped `60c6e7ea` |
| **T-159.18** | **ready** — `t159_18_select_tools.md` |
| **T-159.19+** | queued |

## Ops

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-159
# + smoke_persist_editor.mjs
```
