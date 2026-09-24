# T-159.22.1 — Undo granularity (shipped) — gate driver, not core

**Parent:** [`t159_leptos_ui_program.md`](t159_leptos_ui_program.md) · **Executor:** claude-code ·
**Worktree:** `.ai/artifacts/worktrees/TBD-T-159/` · **Shipped:** **`ce73c5bc`** (tag **T-159.22.1**)

## Corrigendum (read this first)

The **pre-ship** version of this spec assumed a `map-engine-core` UndoManager defect (U1–U3 below as
originally written). **That premise was wrong.**

**Truth:** one LOCAL txn = one undo step always held. T-159.22’s “two moves → one Ctrl+Z reverts both”
was a **measurement bug**: `smoke_undo_editor.mjs` `keyChord()` sent CDP `rawKeyDown` **and**
`keyDown`, so Chrome delivered **two** `keydown`s per chord. A real operator keypress never
double-fired. No product re-check needed.

Authoritative write-up: [`.ai/artifacts/t159_22_1_verify_log.md`](../../.ai/artifacts/t159_22_1_verify_log.md).  
Supersedes: [`.ai/artifacts/t159_22_verify_log.md`](../../.ai/artifacts/t159_22_verify_log.md) §Pre-existing defect
(conclusion only — observations explained).

## What actually shipped

| Change | Role |
|--------|------|
| `keyChord()` → one `rawKeyDown` + `keyUp` | **The fix** |
| Undo smoke rebased to **two** drags; `undo_depth` + A7 keydown count | Step-boundary + regression guard |
| `MissionDocCore::undo_depth()` + bridge | Read-only; `can_undo` alone hid double-pop |
| Two in-crate boundary unit tests | Green on baseline (no red phase possible) |

**No behaviour change** in undo capture. Core comment corrected to cite both `extend` guards.

## Original locked decisions (historical — what the slice was asked to dig)

| # | Original ask | Outcome |
|---|--------------|---------|
| U1 | Invariant: one LOCAL gesture = one undo step | **Already true** |
| U2 | Red unit test then fix | Tests **green on baseline**; reported, not manufactured red |
| U3 | Fix in map-engine-core | **N/A** — fix in gate driver |
| U4 | Extend smoke for step boundaries | **Done** (21 checks) |
| U5 | React/yrs parity unchanged | **Held** |
| U6–U7 | Out of scope / keep smokes | **Held** |
