# T-159.22.1 — Undo granularity (one LOCAL txn = one undo step)

**Parent:** [`t159_leptos_ui_program.md`](t159_leptos_ui_program.md) · **Executor:** claude-code ·
**Worktree:** `.ai/artifacts/worktrees/TBD-T-159/` · **Baseline:** **T-159.22** @ `0154b4e9`

## Problem

Documented invariant: with `capture_timeout_millis: 0` + `ZeroClock`, every LOCAL-origin transaction
is its own undo stack item. **Observed:** two consecutive LOCAL mutations (two drag-moves, or two
places) merge into **one** undo step — one Ctrl+Z reverts both; `can_undo` goes false.

Proven pre-existing on untouched `54c8a4bd` (T-159.21 merge tip). `smoke_undo_editor` cannot catch it
(only one mutation). Mechanism unresolved vs yrs 0.27.2 static reading — dig + fix in
`map-engine-core`, then tighten gates.

Full repro: [`.ai/artifacts/t159_22_verify_log.md`](../../.ai/artifacts/t159_22_verify_log.md) §Pre-existing defect.

## Locked decisions

| # | Decision |
|---|----------|
| U1 | **Invariant:** each LOCAL `MissionDocCore` mutation that today is one user gesture (one `move_entities`, one `add_slot`, …) must be **exactly one** undo step. Two moves → two undos to restore `d0`. |
| U2 | **Prove in-crate first:** native/wasm unit test in `map-engine-core` — two LOCAL ops → `undo()` restores only the second → second `undo()` restores the first. Fail the test on baseline before the fix. |
| U3 | **Fix in `map-engine-core`** (likely UndoManager options / clock / scope / origin plumbing — dig; do not paper over in Leptos by coalescing UI). Document the real root cause in the verify log. |
| U4 | **Smoke:** extend `smoke_undo_editor` (or sibling) — two moves → Ctrl+Z once → digest equals post-first-move (not `d0`); `can_undo` still true; second undo → `d0`. Same shape optional for two places. |
| U5 | **Parity:** if React `map-engine-wasm` / yrs host shares this core, keep behaviour consistent; do not silently change React Yjs `captureTimeout: 0` contract without noting it. |
| U6 | **Out of scope:** Attributes, ORBAT, Arsenal, seed_random filing, Unfiled redesign. |
| U7 | Keep all **11** editor smokes green; marquee/undo `?force=webgl` as required. |

## Do

1. Repro unit test (red) → dig yrs/UndoManager → fix → unit test green.
2. Extend undo smoke for two-step boundary.
3. `.ai/artifacts/t159_22_1_verify_log.md` · tag **T-159.22.1**.

## Claude Code prompt — T-159.22.1 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the WORKTREE at .ai/artifacts/worktrees/TBD-T-159/ (NOT main).

Implement **T-159.22.1** — Undo granularity hotfix (one LOCAL txn = one undo step).

═══ PREFLIGHT ═══
  cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-159
  test "$(basename "$(git rev-parse --show-toplevel)")" = "TBD-T-159"
  git status --porcelain
  git branch --show-current
  git rev-parse --short HEAD  # expect 0154b4e9 (T-159.22) or later after main merge

═══ READ ═══
  1. .ai/artifacts/t159_22_1_claude_code_handoff.md
  2. docs/platform/t159_22_1_undo_granularity.md
  3. docs/platform/t159_leptos_ui_program.md
  4. .ai/artifacts/t159_22_verify_log.md  §Pre-existing defect (repro + analysis)
  5. crates/map-engine-core/src/doc/store.rs  (UndoManager opts, ZeroClock, LOCAL_ORIGIN, undo_*)
  6. apps/website-leptos/src/mission_history.rs
  7. .ai/artifacts/t159_gates/driver/smoke_undo_editor.mjs
  8. yrs 0.27.x undo capture/extend path (crate source / docs) — dig the contradiction

═══ PROBLEM ═══
  Two LOCAL mutations merge into one undo step. Docs say capture_timeout_millis=0 prevents that.
  User Ctrl+Z discards more work than one gesture. Smoke only tests one mutation.

═══ SHIPPED ═══
  T-159.22 @ 0154b4e9 — outliner + palette; documented this defect, left untouched
  T-159.21 @ f02fed5a — undo UI (defect already present)

═══ LOCKED ═══
  - Fix in map-engine-core; in-crate unit test first (red→green)
  - Two moves: one undo → mid digest; two undos → d0; can_undo flips correctly
  - Extend smoke_undo_editor (or sibling) for step boundaries
  - Document real root cause in verify log
  - Keep 11 editor smokes green

═══ DO ═══
  1. Red unit test → dig → fix → green
  2. Smoke step-boundary assertions
  3. .ai/artifacts/t159_22_1_verify_log.md
  4. Commit T-159.22.1: · tag T-159.22.1 (Opus harness trailer)

═══ DO NOT ═══
  - Edit docs/** or registry
  - Paper over in Leptos only (e.g. fake multi-undo UI)
  - Port Attributes / ORBAT / Arsenal
  - “Fix” unrelated clippy drift

═══ VERIFY ═══
  map-engine-core unit test for two LOCAL undos
  Prior 11 smokes + extended undo smoke
  cargo check wasm32 + trunk build --release

═══ RETURN ═══
  - SHA + tag T-159.22.1
  - verify log with root-cause write-up
  - Ready for Cursor → T-159.23 (Attributes modal)
```
