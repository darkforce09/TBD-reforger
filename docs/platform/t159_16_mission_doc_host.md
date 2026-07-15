# T-159.16 — MissionDoc host in Leptos editor

**Parent:** [`t159_leptos_ui_program.md`](t159_leptos_ui_program.md) · **Executor:** claude-code ·
**Worktree:** `.ai/artifacts/worktrees/TBD-T-159/` @ `t-159-leptos-ui` · **Baseline:**
**T-159.15.2** @ `ebcabe1d` (tag **T-159.15.2**)

## Problem

Editor owns `RenderEngine` (camera pan/zoom/loop) but has **no mission document**. React’s
authoritative state is wasm `yrs` behind `WasmMissionDoc` (`state/wasmDoc.ts` + `ydoc.ts`
mutators). Without a doc host, slot pick, undo, IDB, and save cannot land.

## Locked decisions

| # | Decision |
|---|----------|
| D1 | Own a **MissionDoc** lifecycle in the Leptos editor: create on mount, free on dispose (StrictMode-safe — no double-free). Match React shell semantics (`attach`/`detach`/`alive`). |
| D2 | Prefer **same wasm module** as the engine (boundary collapse). Use `MissionDocCore` / existing crate APIs — do **not** reintroduce a second `map_engine_wasm` JS shim if avoidable. If linking requires a thin re-export from `map-engine-render` or workspace dep on `map-engine-core`, document it. |
| D3 | **Seed:** empty doc OR apply a tiny golden `apply_update` fixture (record bytes in verify log). |
| D4 | **Class R gate:** after seed + one documented mutator (e.g. ensure default layer/squad **or** `apply_update` round-trip), `encode_state()` (or equivalent) matches oracle / is stable across encode→apply→encode. |
| D5 | Optional: `bind_mission_doc` / slots GPU bridge if API exists and is cheap — empty slots OK. Not required if bind needs more world stack. |
| D6 | Expose a smoke bridge (e.g. `window.__missionDoc`) with slot_count / change_version / encode hex-or-len — enough for headless Class R. |
| D7 | **Out of scope:** full `ydoc.ts` mutator port, Zustand mirror, IDB `yrsPersist` (→ **.17**), select/marquee/entity drag, outliner, save/export, Arsenal, Eden chrome. |
| D8 | Keep 15.x camera: `disable_frame_timing`, `poll`, pan, wheel; do not regress smokes. |

## Do

1. Wire MissionDoc into `mission_editor` (or `mission_doc.rs` module) with safe dispose.
2. Seed + Class R encode round-trip test/smoke.
3. Verify log `.ai/artifacts/t159_16_verify_log.md`.
4. Commit + tag **T-159.16**.

## Verify

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-159
# Regression:
#   smoke_editor.mjs · selfcheck_editor.mjs · smoke_pan_editor.mjs
# New:
#   smoke_doc_editor.mjs (or vitest/cargo equivalent) — encode_state Class R
cargo check -p website-leptos --target wasm32-unknown-unknown
trunk build --release
```

## Claude Code prompt — T-159.16 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the WORKTREE at .ai/artifacts/worktrees/TBD-T-159/ (NOT main).

Implement **T-159.16** — MissionDoc host in Leptos editor.

═══ PREFLIGHT ═══
  cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-159
  test "$(basename "$(git rev-parse --show-toplevel)")" = "TBD-T-159"
  git status --porcelain
  git branch --show-current   # t-159-leptos-ui
  git rev-parse --short HEAD  # expect ebcabe1d (T-159.15.2) or later
  # Do NOT nest ./scripts/ticket run

═══ READ (in order — spec wins) ═══
  1. .ai/artifacts/t159_16_claude_code_handoff.md
  2. docs/platform/t159_16_mission_doc_host.md
  3. docs/platform/t159_leptos_ui_program.md
  4. .ai/artifacts/t159_15_2_verify_log.md
  5. apps/website-leptos/src/mission_editor.rs
  6. apps/website/frontend/src/features/tactical-map/state/wasmDoc.ts
  7. apps/website/frontend/src/features/mission-creator/hooks/useMissionDoc.ts
  8. crates/map-engine-core/src/doc/ (MissionDocCore)
  9. crates/map-engine-wasm/src/lib.rs MissionDoc wasm API (reference)
  10. crates/map-engine-render — any bind_mission_doc / slots hooks

═══ PROBLEM ═══
  Engine + camera shipped; no mission yrs doc in Leptos. Need lifecycle + Class R encode
  round-trip so .17+ persist/tools/save can land. Not a full mutator/UI port.

═══ SHIPPED (do not reopen) ═══
  T-159.15.0–.15.2 @ 3066f14c / a425936d / ebcabe1d
  24 page routes byte-identical

═══ LANGUAGE GATE ═══
  Doc ops in Rust (core/engine). Leptos = lifecycle + thin smoke bridge only.

═══ LOCKED ═══
  - Attach/detach/free once; no double-free
  - Same-wasm preference; no second JS wasm shim if avoidable
  - Seed empty or tiny golden apply_update
  - Class R encode_state round-trip
  - No full ydoc port, IDB, pick/marquee, Eden chrome
  - Keep pan/wheel/disable_frame_timing/poll; no GpuTimer (T-160)

═══ DO ═══
  1. MissionDoc host module + editor mount/dispose
  2. Seed + Class R smoke/driver
  3. .ai/artifacts/t159_16_verify_log.md
  4. Commit T-159.16: · tag T-159.16
     Co-Authored-By: Claude Code <noreply@anthropic.com>

═══ DO NOT ═══
  - Edit docs/** or .ai/tickets/registry.json
  - Port all of ydoc.ts / outliner / save
  - Break 15.x smokes
  - Resurrect unproject_xy / re-enable GpuTimer

═══ VERIFY ═══
  smoke_editor + selfcheck_editor + smoke_pan_editor still pass
  new doc Class R gate green
  cargo check wasm32 + trunk build --release

═══ RETURN ═══
  - Commit SHA + tag T-159.16
  - .ai/artifacts/t159_16_verify_log.md
  - Ready for Cursor doc sync → next T-159.17 (yrsPersist / editor session)
```
