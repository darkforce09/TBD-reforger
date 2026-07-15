# T-159.15.2 — Mission Creator camera pan + pointer foundation

**Parent:** [`t159_leptos_ui_program.md`](t159_leptos_ui_program.md) · **Executor:** claude-code ·
**Worktree only:** `.ai/artifacts/worktrees/TBD-T-159/` @ `t-159-leptos-ui` · **Baseline:**
**T-159.15.1** @ `a425936d` (tag **T-159.15.1**)

## Problem

Editor has damage-driven rAF + wheel-zoom + resize, but **no pan** and no pointer→world
seam. React Eden uses middle/right-drag pan (`useSelectTool`) and freezes a viewport at
gesture start; wheel mid-pan **rebases** the pan (T-151.11.6). Without pan, the map is not
operable. Slot pick / marquee / entity drag need `WasmMissionDoc` + spatial index → **T-159.16+**.

## Locked decisions

| # | Decision |
|---|----------|
| P1 | **MMB + RMB drag = pan** (match React: `button === 1 \|\| button === 2`). LMB left alone this slice (no marquee/move yet). |
| P2 | Prefer **`engine.pan(dx_px, dy_px)`** (already on `RenderEngine`) for content-follows-cursor pan; keep camera bounds from 15.0. |
| P3 | **`contextmenu` preventDefault** on container so RMB pan is not blocked by the browser menu. |
| P4 | **Pointer capture** on pan start; release on up/cancel. |
| P5 | **Wheel mid-pan rebase** (T-151.11.6): after `zoom_at`, refresh pan gesture start target + start px so pan continues without re-press. |
| P6 | **Do not** resurrect `RenderEngine::unproject_xy` (deleted T-151.11.2 / audit X-05 — live unproject feedback-loops mid-pan). Pan via `engine.pan`; smoke asserts `target_x`/`target_y`/`zoom` getters. Frozen viewport unproject for pick lands with **T-159.16** + gesture machine. |
| P7 | Keep **`disable_frame_timing()`** + per-frame **`poll()`** from 15.1. Do not re-enable GpuTimer (→ **T-160**). |
| P8 | Gates = **smoke / camera Class R**, not DOM V-suite. Self-check `?force=webgl` still green. |
| P9 | **Out of scope:** slot pick, marquee, entity drag-move, clusters, MissionDoc, basemap/world loaders, Eden chrome. |

## Do

1. Pointer listeners on editor container: down/move/up/cancel + contextmenu.
2. Pan gesture machine (minimal): start on MMB/RMB → track dx/dy → `engine.pan` (rAF-coalesce if needed, match T-057 spirit).
3. Wire wheel handler to rebase in-flight pan after zoom.
4. Smoke helper: read `engine.target_x()` / `target_y()` / `zoom()` (already getters) — **do not** re-add `unproject_xy` (X-05).
5. Extend / add smoke: scripted RMB drag → camera target changes (Class R vs pre-drag); no panic; existing wheel + selfcheck still pass.
6. Commit + tag **T-159.15.2**.

## Verify

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-159
test "$(basename "$(git rev-parse --show-toplevel)")" = "TBD-T-159"
# Existing:
#   smoke_editor.mjs — wheel still pass
#   selfcheck_editor.mjs — ?force=webgl still pass
# New: pan smoke (name in verify log) — RMB drag moves target; no "already mapped"
cargo check -p website-leptos
clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings   # if engine API changed
trunk build --release
```

## Claude Code prompt — T-159.15.2 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the WORKTREE at .ai/artifacts/worktrees/TBD-T-159/ (NOT main).

Implement **T-159.15.2** — Mission Creator camera pan + pointer foundation.

═══ PREFLIGHT ═══
  cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-159
  test "$(basename "$(git rev-parse --show-toplevel)")" = "TBD-T-159"
  git status --porcelain
  git branch --show-current   # expect t-159-leptos-ui
  git rev-parse --short HEAD  # expect a425936d (T-159.15.1) or later
  # Do NOT checkout other branches; do NOT nest ./scripts/ticket run

═══ READ (in order — spec wins on conflict) ═══
  1. .ai/artifacts/t159_15_2_claude_code_handoff.md
  2. docs/platform/t159_15_2_camera_pan.md
  3. docs/platform/t159_leptos_ui_program.md
  4. .ai/artifacts/t159_15_1_verify_log.md   # real 15.1 root cause (GpuTimer / disable_frame_timing)
  5. apps/website-leptos/src/mission_editor.rs
  6. crates/map-engine-render/src/engine.rs  # pan, zoom_at, set_view, poll, disable_frame_timing
  7. apps/website/frontend/src/features/tactical-map/tools/useSelectTool.ts  # MMB/RMB pan + rebasePan
  8. apps/website/frontend/src/features/tactical-map/WgpuTacticalMap.tsx     # wheel + rebase (~550–585)

═══ PROBLEM ═══
  15.1 shipped the render loop + wheel. Editor still cannot pan. Need React-parity MMB/RMB
  pan + mid-pan wheel rebase + unproject seam for later pick/CUR. Slot pick needs MissionDoc → .16.

═══ SHIPPED (do not reopen) ═══
  T-159.15.0 @ 3066f14c — boundary collapse
  T-159.15.1 @ a425936d — loop + wheel + disable_frame_timing + poll + selfcheck
  24 page routes byte-identical (untouched)

═══ LANGUAGE GATE ═══
  Camera math stays in map-engine-core / RenderEngine. Leptos only: DOM pointer events → engine.pan / zoom_at.
  Do not reimplement ortho unproject in ad-hoc JS/TS.

═══ LOCKED ═══
  - MMB+RMB pan; LMB deferred (no marquee/slot move)
  - engine.pan(dx,dy); contextmenu preventDefault; pointer capture
  - Wheel mid-pan rebase (T-151.11.6)
  - Keep disable_frame_timing + poll; no GpuTimer (T-160)
  - Do NOT resurrect engine.unproject_xy (X-05); smoke via target_x/y/zoom getters
  - Gates: pan smoke Class R + existing wheel/selfcheck; NOT DOM V-suite
  - No MissionDoc / slots / basemap / Eden chrome

═══ DO ═══
  1. Pan gesture + listeners on mission_editor container
  2. Rebase pan after wheel zoom
  3. Pan smoke driver under .ai/artifacts/t159_gates/driver/ (assert target moves)
  4. .ai/artifacts/t159_15_2_verify_log.md (gates table + HEAD SHA)
  5. Commit T-159.15.2: · tag T-159.15.2
     Co-Authored-By: Claude Code <noreply@anthropic.com>

═══ DO NOT ═══
  - Edit docs/**, .ai/tickets/registry.json, CLAUDE ticket-sync markers
  - Slot pick / marquee / entity drag / MissionDoc (.16)
  - Re-enable GpuTimer
  - Break 15.1 smoke_editor / selfcheck_editor
  - Nested worktrees / ./scripts/ticket run

═══ VERIFY ═══
  smoke_editor.mjs pass (wheel)
  selfcheck_editor.mjs pass (?force=webgl)
  new pan smoke: target moves after RMB drag; no "already mapped"
  cargo check / trunk build --release as needed

═══ RETURN ═══
  - Commit SHA + tag T-159.15.2
  - .ai/artifacts/t159_15_2_verify_log.md
  - Ready for Cursor doc sync → next T-159.16 WasmMissionDoc host
```
