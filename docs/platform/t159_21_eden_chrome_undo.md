# T-159.21 — Eden chrome scaffold + undo/redo

**Parent:** [`t159_leptos_ui_program.md`](t159_leptos_ui_program.md) · **Executor:** claude-code ·
**Worktree:** `.ai/artifacts/worktrees/TBD-T-159/` · **Baseline:** **T-159.20** @ `c0e11d54`

## Problem

Editor is chromeless canvas + a minimal Save/Export strip. React Eden has Top Command Strip
(undo/redo, title, settings stub, save/export), Bottom Toolbelt (tools + CUR/SEL/OBJ), and
docked left/right panels. Need the **chrome scaffold + working undo/redo** before outliner/palette
depth (.22+).

## Locked decisions

| # | Decision |
|---|----------|
| C1 | **Top strip (minimal Eden):** title (mission id or meta title), **Undo** / **Redo**, existing Save/Export (integrate, don’t duplicate), optional disabled Settings stub. Aegis glass / overlay recipe (`overlay.ts` tokens). |
| C2 | **Undo/redo:** drive `MissionDocCore` undo stack (LOCAL-origin only — match React `createUndoManager`). After undo/redo: re-bind slots glyphs / selection as needed; schedule persist. Expose `can_undo`/`can_redo` on smoke bridge. |
| C3 | **Bottom toolbelt (stub OK):** Select tool active indicator; CUR X/Y readout from engine camera/cursor if cheap; SEL count from selection; OBJ = `slot_count`. Ruler/LoS disabled stubs fine. |
| C4 | **Dock shells (empty):** left `w-64` + right `w-80` (or React widths) frosted panels — placeholders “ORBAT / Layers” and “Assets” — **no** tree/virtual outliner/palette data yet (.22). |
| C5 | Keyboard: **Ctrl/Cmd+Z** undo, **Ctrl/Cmd+Shift+Z** or **Ctrl+Y** redo (skip when focus in INPUT). |
| C6 | Class R smoke: move a slot → undo → positions digest restored; redo restores move; `can_undo` flips. |
| C7 | **Out of scope:** VirtualOutliner, AssetBrowser DnD, Attributes modal, Mission Settings dialog, Arsenal, cluster, full menu stubs. |
| C8 | Keep all **8** editor smokes green (incl. save-export); marquee `?force=webgl`. |

## Do

1. Chrome layout around canvas (top/bottom/docks) + integrate Save/Export.
2. Undo/redo API + buttons + shortcuts + post-undo rebind/persist.
3. Smoke + `.ai/artifacts/t159_21_verify_log.md` · tag **T-159.21**.

## Claude Code prompt — T-159.21 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the WORKTREE at .ai/artifacts/worktrees/TBD-T-159/ (NOT main).

Implement **T-159.21** — Eden chrome scaffold + undo/redo.

═══ PREFLIGHT ═══
  cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-159
  test "$(basename "$(git rev-parse --show-toplevel)")" = "TBD-T-159"
  git status --porcelain
  git branch --show-current
  git rev-parse --short HEAD  # expect c0e11d54 (T-159.20) or later

═══ READ ═══
  1. .ai/artifacts/t159_21_claude_code_handoff.md
  2. docs/platform/t159_21_eden_chrome_undo.md
  3. docs/platform/t159_leptos_ui_program.md
  4. .ai/artifacts/t159_20_verify_log.md
  5. apps/website-leptos/src/mission_editor.rs
  6. apps/website-leptos/src/mission_commands.rs
  7. apps/website-leptos/src/mission_doc.rs
  8. apps/website/frontend/src/features/tactical-map/state/undo.ts
  9. apps/website/frontend/src/features/mission-creator/layout/TopCommandStrip.tsx
  10. apps/website/frontend/src/features/mission-creator/layout/BottomToolbelt.tsx
  11. apps/website/frontend/src/features/mission-creator/layout/overlay.ts
  12. crates/map-engine-core/src/doc/ — undo/redo APIs on MissionDocCore

═══ PROBLEM ═══
  Canvas+Save/Export only. Need Eden top/bottom/dock chrome scaffold and working undo/redo
  on MissionDocCore before outliner/palette (.22).

═══ SHIPPED ═══
  T-159.20 @ c0e11d54 — compile + Save/Export + live POST
  T-159.19 marquee/move + edit persist

═══ LOCKED ═══
  - Top: title, Undo/Redo, Save/Export integrated; Settings stub OK
  - Undo/redo via MissionDocCore; rebind + persist after
  - Bottom: Select + CUR/SEL/OBJ stubs; docks empty placeholders
  - Ctrl/Cmd+Z / Shift+Z / Ctrl+Y
  - Keep 8 editor smokes; no full outliner/palette/Arsenal

═══ DO ═══
  1. Chrome layout + undo/redo + shortcuts
  2. Undo smoke Class R
  3. .ai/artifacts/t159_21_verify_log.md
  4. Commit T-159.21: · tag T-159.21 (Opus harness trailer)

═══ DO NOT ═══
  - Edit docs/** or registry
  - Port VirtualOutliner / AssetBrowser / Attributes
  - Break save-export or marquee smokes
  - “Fix” pre-existing crate-wide clippy drift outside your files

═══ VERIFY ═══
  Prior 8 smokes pass
  New: undo/redo restores move digest; can_undo/can_redo
  cargo check wasm32 + trunk build --release

═══ RETURN ═══
  - SHA + tag T-159.21
  - verify log
  - Ready for Cursor → T-159.22 (outliner / asset palette)
```
