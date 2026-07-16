# T-159.23 — Claude Code handoff (Attributes modal)

**CWD:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-159`  
**Baseline:** `ce73c5bc` / tag `T-159.22.1`  
**Spec:** [`docs/platform/t159_23_attributes_modal.md`](../../docs/platform/t159_23_attributes_modal.md)

## Context

Outliner + palette + undo (product-correct) shipped. Next = **Attributes** modal (Transform +
Identity). Arsenal tab is a stub only — Forge/loadout is a later stream (or Fable bulk plan).

**T-159.22.1 corrigendum:** undo core was never broken; do not “fix” UndoManager. See
`.ai/artifacts/t159_22_1_verify_log.md`.

## Notes

- Soft-WebGPU smokes stay `?force=webgl` where required.
- Prefer existing Leptos Dialog patterns from suite pages.
- Clippy: stash-diff for zero new lints if full-crate counts stay red.

## Return

Tag **T-159.23** + verify log → Cursor (or Fable bulk plan per operator).
