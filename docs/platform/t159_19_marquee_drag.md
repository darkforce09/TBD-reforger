# T-159.19 — Marquee select + entity drag-move

**Parent:** [`t159_leptos_ui_program.md`](t159_leptos_ui_program.md) · **Executor:** claude-code ·
**Worktree:** `.ai/artifacts/worktrees/TBD-T-159/` · **Baseline:** **T-159.18** @ `eb30ebea`

## Problem

LMB click-select shipped. Still missing React `useSelectTool` over-threshold paths: **marquee**
(empty drag → `pickRect`) and **entity drag-move** (icon drag → preview → `move_entities` commit).
Also: first real mutator should fire **persist debounce** (.17 S8 / .18 S8).

## Locked decisions

| # | Decision |
|---|----------|
| M1 | Promote pending-left past **4px** → `move` if pick hit at down, else `marquee`. |
| M2 | **Frozen viewport** for all unprojects in the gesture (same as .18). |
| M3 | Marquee: on release, `PointIndex::pick_rect` (world AABB from start→end); set selection (replace). Min size ≥1 CSS px both axes (React). Optional transient marquee rect for smoke only — full HUD chrome later. |
| M4 | Move: track world delta via frozen unproject; on release if delta ≠ 0 call **`MissionDocCore::move_entities`** (or equivalent) then refresh spatial index / SoA bind if needed; update `engine.set_selection` as today. |
| M5 | **Persist notify:** after successful move mutator, call yrs_persist debounce/`schedule_save` so edit-driven IDB write exists. Smoke may assert digest/slot positions change + optional persist flush. |
| M6 | rAF-coalesce move preview if engine exposes a cheap selection/drag preview; otherwise commit-only on up is OK for .19 if documented (prefer live pan-like preview via `set_selection` + temporary positions only if safe). |
| M7 | **Out of scope:** cluster drill, Attributes dblclick, undo stack UI, compiler save/export (.20), Eden docked shell, GpuTimer. |
| M8 | Keep all prior smokes green (select/persist/pan/doc/wheel/selfcheck). |

## Do

1. Extend `select_tool` / `mission_editor` LMB machine: marquee + move.
2. Wire `move_entities` + spatial refresh + persist schedule.
3. Smokes: marquee selects ≥1 seed slots; drag moves slot Class R (position digest); persist optional.
4. `.ai/artifacts/t159_19_verify_log.md` · tag **T-159.19**.

## Claude Code prompt — T-159.19 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the WORKTREE at .ai/artifacts/worktrees/TBD-T-159/ (NOT main).

Implement **T-159.19** — Marquee select + entity drag-move.

═══ PREFLIGHT ═══
  cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-159
  test "$(basename "$(git rev-parse --show-toplevel)")" = "TBD-T-159"
  git status --porcelain
  git branch --show-current
  git rev-parse --short HEAD  # expect eb30ebea (T-159.18) or later

═══ READ ═══
  1. .ai/artifacts/t159_19_claude_code_handoff.md
  2. docs/platform/t159_19_marquee_drag.md
  3. docs/platform/t159_leptos_ui_program.md
  4. .ai/artifacts/t159_18_verify_log.md
  5. apps/website-leptos/src/select_tool.rs
  6. apps/website-leptos/src/mission_editor.rs
  7. apps/website-leptos/src/yrs_persist.rs
  8. apps/website/frontend/src/features/tactical-map/tools/useSelectTool.ts
  9. crates/map-engine-core/src/doc/store.rs  # move_entities
  10. crates/map-engine-core/src/spatial/point_index.rs

═══ PROBLEM ═══
  Click-select only. Need over-threshold marquee + drag-move with MissionDocCore mutator
  and first edit-driven persist notify.

═══ SHIPPED ═══
  T-159.18 @ eb30ebea — LMB click-select; frozen cam; pick_rect+argmin
  T-159.17 persist (not yet edit-driven)

═══ LOCKED ═══
  - 4px promote → move|marquee; frozen viewport only
  - marquee pick_rect → selection; move → move_entities + persist schedule
  - Keep prior smokes; no GpuTimer / live unproject_xy
  - No save/export / Eden chrome / clusters

═══ DO ═══
  1. Marquee + drag-move in select_tool / mission_editor
  2. move_entities + spatial refresh + yrs_persist notify
  3. New smoke(s) + .ai/artifacts/t159_19_verify_log.md
  4. Commit T-159.19: · tag T-159.19 (Opus harness trailer)

═══ DO NOT ═══
  - Edit docs/** or registry
  - Break editor-select-smoke or persist/pan/doc smokes
  - Resurrect RenderEngine::unproject_xy

═══ VERIFY ═══
  Prior 6 editor smokes pass
  New: marquee selects; drag moves Class R; persist notify documented/tested
  cargo check wasm32 + trunk build --release

═══ RETURN ═══
  - SHA + tag T-159.19
  - verify log
  - Ready for Cursor → T-159.20 (save/export)
```
