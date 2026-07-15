# T-159.18 — Select / LMB tools (pick foundation)

**Parent:** [`t159_leptos_ui_program.md`](t159_leptos_ui_program.md) · **Executor:** claude-code ·
**Worktree:** `.ai/artifacts/worktrees/TBD-T-159/` · **Baseline:** **T-159.17** @ `60c6e7ea`

## Problem

Editor has pan (MMB/RMB) + MissionDoc + IDB persist, but **no LMB select**. React
`useSelectTool`: pending-left → click pick / Ctrl-toggle / empty deselect; over threshold →
move or marquee. Pick uses **frozen viewport** unproject + `slotSpatialIndex.pickNearest`
(not live engine unproject — X-05).

Also from .17: debounced persist is not edit-driven yet — needs a **doc change hook** when
mutators exist.

## Locked decisions

| # | Decision |
|---|----------|
| S1 | Port **LMB click-select** first: pending-left + `DRAG_THRESHOLD` (4px); sub-threshold release = pick. |
| S2 | **Frozen viewport** at pointer-down for unproject (copy ortho camera / view state once — do **not** resurrect `RenderEngine::unproject_xy`). |
| S3 | Pick via Rust **`PointIndex` / MissionDocCore SoA** (map-engine-core spatial) rebuilt or maintained from seeded slots — Class S vs brute-force nearest on same points. |
| S4 | Selection state in Leptos signals: `{ ids: Vec<String> }` (or slot ids as used by core). Plain click = replace; **Ctrl/Cmd** = toggle; empty click = clear. |
| S5 | Smoke: scripted click near known seed slot → selection contains that id; empty click clears; optional Ctrl toggle. Expose `window.__editorSelection` or extend `__missionDoc`. |
| S6 | **Optional this slice if cheap:** marquee rect visual + `pick_rect` (no entity move yet). |
| S7 | **Out of scope unless trivial:** entity drag-move commit (`moveEntities`), cluster drill, full Zustand icon cache, Attributes dblclick, Eden chrome. If move lands, must call MissionDocCore mutator + notify persist debounce. |
| S8 | Add **`on_change` / bump hook** on doc host so `yrs_persist` debounce runs after mutators (even if only selection-adjacent mutators this slice). Document if selection-only has no encode change. |
| S9 | Keep all prior smokes green (incl. `smoke_persist_editor`). |

## Do

1. Frozen-viewport helper + PointIndex (or equivalent) over MissionDoc slots.
2. LMB gesture machine (pending → click select) alongside existing pan.
3. Selection signal + smoke.
4. Wire persist notify if any mutator fires.
5. `.ai/artifacts/t159_18_verify_log.md` · tag **T-159.18**.

## Claude Code prompt — T-159.18 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the WORKTREE at .ai/artifacts/worktrees/TBD-T-159/ (NOT main).

Implement **T-159.18** — Select / LMB tools (pick foundation).

═══ PREFLIGHT ═══
  cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-159
  test "$(basename "$(git rev-parse --show-toplevel)")" = "TBD-T-159"
  git status --porcelain
  git branch --show-current
  git rev-parse --short HEAD  # expect 60c6e7ea (T-159.17) or later

═══ READ ═══
  1. .ai/artifacts/t159_18_claude_code_handoff.md
  2. docs/platform/t159_18_select_tools.md
  3. docs/platform/t159_leptos_ui_program.md
  4. .ai/artifacts/t159_17_verify_log.md
  5. apps/website-leptos/src/mission_editor.rs
  6. apps/website-leptos/src/mission_doc.rs
  7. apps/website/frontend/src/features/tactical-map/tools/useSelectTool.ts
  8. apps/website/frontend/src/features/tactical-map/state/slotSpatialIndex.ts
  9. crates/map-engine-core/src/spatial/point_index.rs
  10. crates/map-engine-core/src/camera/ortho.rs  # unproject_xy for frozen camera
  11. crates/map-engine-core/src/doc/store.rs     # slot SoA / add_slot

═══ PROBLEM ═══
  No LMB pick/select. Need React-parity click-select on seeded slots using frozen viewport
  + Rust PointIndex. Entity drag-move / full marquee optional. Mutator→persist hook if mutators land.

═══ SHIPPED ═══
  T-159.17 @ 60c6e7ea — IDB persist + warm session; semantic digest Class R
  T-159.16 MissionDoc host; 15.x camera/pan

═══ LOCKED ═══
  - LMB pending-left + 4px threshold; click pick / Ctrl toggle / empty clear
  - Frozen viewport unproject only — NO RenderEngine::unproject_xy
  - PointIndex / MissionDocCore SoA for pick
  - Selection signal + smoke bridge
  - Keep prior smokes; no GpuTimer
  - Drag-move / Attributes / Eden chrome deferred unless trivial

═══ DO ═══
  1. Frozen viewport + spatial pick + LMB select machine
  2. Selection smoke driver
  3. Persist notify hook if mutators added
  4. .ai/artifacts/t159_18_verify_log.md
  5. Commit T-159.18: · tag T-159.18 (Opus harness trailer)

═══ DO NOT ═══
  - Edit docs/** or registry
  - Resurrect live engine unproject_xy
  - Break persist/pan/doc/wheel/selfcheck smokes

═══ VERIFY ═══
  Prior smokes pass
  New select smoke: click seed slot → selection; empty clears
  cargo check wasm32 + trunk build --release

═══ RETURN ═══
  - SHA + tag T-159.18
  - verify log
  - Ready for Cursor → T-159.19 (as hub directs — save/export or marquee/move)
```
