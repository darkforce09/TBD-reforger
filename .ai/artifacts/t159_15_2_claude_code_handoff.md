# T-159.15.2 — Claude Code handoff (camera pan)

**CWD:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-159`  
**Branch:** `t-159-leptos-ui` · **Baseline tag:** `T-159.15.1` (`a425936d`)  
**Spec:** [`docs/platform/t159_15_2_camera_pan.md`](../../docs/platform/t159_15_2_camera_pan.md)  
**Hub:** [`docs/platform/t159_leptos_ui_program.md`](../../docs/platform/t159_leptos_ui_program.md)

## Context

Page V-suite done. MC editor: one-wasm `RenderEngine`, damage-driven loop, wheel, resize,
`disable_frame_timing` (Dawn GpuTimer fix). Operator next = **pan** then **T-159.16** doc host.

React oracle: `useSelectTool` MMB/RMB → pan with frozen start viewport; `rebasePan` after wheel
(T-151.11.6). Engine already has `pan(dx_px, dy_px)`.

## File map (expected)

| Path | Action |
|------|--------|
| `apps/website-leptos/src/mission_editor.rs` | Pan + contextmenu + rebase |
| `crates/map-engine-render/src/engine.rs` | Touch only if pan API broken — **no** `unproject_xy` resurrection |
| `.ai/artifacts/t159_gates/driver/*pan*` | New smoke |
| `.ai/artifacts/t159_15_2_verify_log.md` | Create |

## Return

Tag **T-159.15.2** + verify log + “Ready for Cursor doc sync.”
