# T-159.23 — Attributes modal

**Parent:** [`t159_leptos_ui_program.md`](t159_leptos_ui_program.md) · **Executor:** claude-code ·
**Worktree:** `.ai/artifacts/worktrees/TBD-T-159/` · **Baseline:** **T-159.22.1** @ `ce73c5bc`

## Problem

Slots can be selected, moved, placed, and listed in the outliner, but there is no **Attributes**
surface (React: double-click → `AttributesModal`) to edit transform / identity fields.

## Locked decisions

| # | Decision |
|---|----------|
| A1 | **Open:** double-click a slot on the map **or** activate from outliner (dbl-click row) → Attributes dialog. Multi-select (>1) suppresses open (match React). Esc / backdrop closes. |
| A2 | **Tabs this slice:** **Transform** (X/Y/Z/rotation NumberFields, blur/Enter commit via `update_slot_position`) + **Identity** (role/tag TextFields; stance select if cheap; squad name readonly). **States** tab may be a stub. |
| A3 | **Arsenal tab:** present in the tab strip as **disabled stub** or “later” placeholder — **do not** port Forge/loadout doll (`loadout/` ~2.8k). |
| A4 | Commits go through `MissionDocCore` + `after_local_edit` (rebind + persist + undo). One gesture commit = one undo step (core confirmed OK in .22.1). |
| A5 | Reuse existing Leptos Dialog patterns (`ui.rs` / page dialogs). Aegis glass tokens. |
| A6 | Class R smoke: open Attributes on a seed/placed slot → commit role or X → digest/OBJ fields change; undo restores; close works. |
| A7 | **Out of scope:** Arsenal/loadout, ORBAT tree, Faction Manager, Mission Settings, VirtualOutliner @ 367k. |
| A8 | Keep all **11** editor smokes green; marquee/undo `?force=webgl` as required. |

## Do

1. Attributes modal + open paths + Transform/Identity commits.
2. Smoke + `.ai/artifacts/t159_23_verify_log.md` · tag **T-159.23**.

## Claude Code prompt — T-159.23 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the WORKTREE at .ai/artifacts/worktrees/TBD-T-159/ (NOT main).

Implement **T-159.23** — Attributes modal (Transform + Identity).

═══ PREFLIGHT ═══
  cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-159
  test "$(basename "$(git rev-parse --show-toplevel)")" = "TBD-T-159"
  git status --porcelain
  git branch --show-current
  git rev-parse --short HEAD  # expect ce73c5bc (T-159.22.1) or later after main merge

═══ READ ═══
  1. .ai/artifacts/t159_23_claude_code_handoff.md
  2. docs/platform/t159_23_attributes_modal.md
  3. docs/platform/t159_leptos_ui_program.md
  4. .ai/artifacts/t159_22_1_verify_log.md  (undo OK — no core bug; gate was wrong)
  5. apps/website/frontend/src/features/mission-creator/layout/AttributesModal.tsx
  6. apps/website/frontend/src/features/mission-creator/layout/RightInspector/fields.tsx
  7. apps/website-leptos/src/mission_editor.rs
  8. apps/website-leptos/src/editor_ops.rs
  9. apps/website-leptos/src/mission_history.rs
  10. apps/website-leptos/src/ui.rs  (Dialog patterns)
  11. crates/map-engine-core/src/doc/store.rs  (update_slot / update_slot_position)

═══ PROBLEM ═══
  No Attributes UI. Need dbl-click modal with Transform + Identity edits wired to MissionDocCore.

═══ SHIPPED ═══
  T-159.22.1 @ ce73c5bc — undo step boundaries OK (gate driver fixed)
  T-159.22 outliner + palette place

═══ LOCKED ═══
  - Open: map dbl-click + outliner activate; multi-select suppresses
  - Tabs: Transform + Identity live; Arsenal stub only; States stub OK
  - Commits via update_slot* + after_local_edit
  - Smoke Class R; keep 11 prior editor smokes
  - No Arsenal/ORBAT/Faction Manager

═══ DO ═══
  1. Modal + open paths + field commits
  2. Smoke Class R
  3. .ai/artifacts/t159_23_verify_log.md
  4. Commit T-159.23: · tag T-159.23 (Opus harness trailer)

═══ DO NOT ═══
  - Edit docs/** or registry
  - Port Arsenal / loadout doll / ORBAT tree
  - Re-open the undo “core defect” (it was never real)
  - “Fix” unrelated clippy/fmt drift

═══ VERIFY ═══
  Prior 11 smokes pass
  New: Attributes open/commit/undo/close
  cargo check wasm32 + trunk build --release

═══ RETURN ═══
  - SHA + tag T-159.23
  - verify log
  - Ready for Cursor → T-159.24 or Fable bulk cutover plan (operator call)
```
