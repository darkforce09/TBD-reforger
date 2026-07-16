# T-159.22.1 — Claude Code handoff (undo granularity)

**CWD:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-159`  
**Baseline:** `0154b4e9` / tag `T-159.22`  
**Spec:** [`docs/platform/t159_22_1_undo_granularity.md`](../../docs/platform/t159_22_1_undo_granularity.md)

## Context

T-159.22 found (did not introduce) that consecutive LOCAL transactions merge into one undo step.
Proven on untouched `54c8a4bd`. Docs in `store.rs` claim `capture_timeout_millis: 0` + `ZeroClock`
prevents merge; yrs static reading also predicts no merge — observation disagrees. Dig + fix in
**map-engine-core**; add an in-crate unit test that fails before the fix.

Repro + notes: `.ai/artifacts/t159_22_verify_log.md` §Pre-existing defect.

## Notes

- Soft-WebGPU smokes stay `?force=webgl` where required.
- Do not paper over in Leptos UI alone.
- Clippy: prove zero new lints with stash-diff if full-crate counts stay red.

## Return

Tag **T-159.22.1** + verify log (root cause) → Cursor sets up **T-159.23** Attributes.
