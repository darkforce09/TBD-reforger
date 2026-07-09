# T-151.7.1 — interaction hotfix (selection tint / drag FPS / zoom-at-cursor)

**Status:** **shipped** @ `fa6ad959` (tag **T-151.7.1**); follow-ups **T-151.7.2** @ `64c64d98` / `69ca1c08` · was ready · **Program:**
[`t151_wgpu_engine_program.md`](t151_wgpu_engine_program.md) · **Executor:** claude-code ·
**Worktree:** `tbd-reforger-wgpu-spike/` (absolute:
`/var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike`; do **not** touch `main`) ·
**Baseline:** `ab6bcb11` (tag **T-151.7** — verify log
[`t151_7_verify_log.md`](../../../.ai/artifacts/t151_7_verify_log.md)).

## In one sentence

Fix three operator regressions after W7: **stale selection tint**, **~40 FPS drop dragging
~1000 slots**, and **wheel zoom no longer anchored under the cursor** during RMB pan.

## Problem (operator @ T-151.7)

1. **Selection tint flaky** — select / deselect / change selection sometimes leaves rings
   yellow or primary wrong until another gesture. Intermittent.
2. **Drag perf** — moving ~1000 selected units drops FPS by ~40. Unacceptable vs Deck T-061.
3. **Zoom-at-cursor regression** — hold RMB (pan) + wheel zoom: world point under cursor
   drifts. Pre-W7 (raw canvas wheel) stayed anchored.

## Root causes (locked from code audit)

| Bug | Cause |
|-----|--------|
| **B1 tint** | Cluster mode (`applySelectionOnlyVisible`) shrinks Slots lane to **k selected** rows, but `syncSelection` still patches by **full-doc row index** into that short lane → `patch_slot_lane` **silent no-op** when `offset ≥ count*20`. Also `syncSelection` skipped while `dragActive`. |
| **B2 drag FPS** | Every `setDragPreviewDelta` frame runs full `syncDrag`: `refresh` + O(n) `slot_ids` Map + hide patches + **`upload_slot_drag_lane` → `create_buffer_init` every frame**. Only `set_slot_drag_delta` should be per-frame (T-061 contract). |
| **B3 zoom** | Pan/pick/resize use **container** CSS rect; wheel uses **`canvas.getBoundingClientRect()`**. Container lacks `position:relative` while canvas is `absolute; inset:0` → rects diverge → `zoom_at` cursor ≠ camera space. |

## Goal

1. **B1:** Selection tint always matches `selection.ids` in detail **and** cluster mode; no silent patch misses; restore tint after drag end.
2. **B2:** Drag start uploads overlay **once**; per-frame = **only** `set_slot_drag_delta` (16 B); reuse/grow drag buffer — no `create_buffer_init` per rAF; FPS drop ≪ 40 @ 1k (target: near Deck / within ~5–10 fps of idle pan).
3. **B3:** Single coordinate origin for pan + wheel + resize (container CSS + `position:relative`); zoom-at-cursor Class R vs OrthoCamera / pre-W7 feel.
4. Verify log + tag **T-151.7.1**.

## Out of scope

- W8 culling / density ladder.
- W9 Deck retirement.
- Redesign gesture SM / thresholds.
- Forest / world glyph / slot atlas art.
- Registry/docs (Cursor-owned).

## Locked decisions

| # | Decision | Rationale |
|---|---|---|
| L1 | Detail mode: full-n Slots lane; selection = size+tint patch only (never shrink lane for tint) | B1 |
| L2 | Cluster mode: either keep a stable id→GPU-row map for the short lane **or** re-upload selection-only bytes on every selection change (no index patch into short lane) | B1 |
| L3 | `patch_slot_lane` must not silently succeed-looking when OOB — log/assert in debug **or** FE never OOB | B1 |
| L4 | Drag: upload/hide once on start; per-frame **only** `set_slot_drag_delta`; buffer reuse (`write_buffer` / grow) | T-061 / B2 |
| L5 | Wheel + pan + `applySize` share **one** element rect (container); container `position:relative` | B3 |
| L6 | W2–W7 regressions green; vitest ≥ **391** | Regression |
| L7 | Commit `T-151.7.1:` · tag **`T-151.7.1`** · verify log `.ai/artifacts/t151_7_1_verify_log.md` | House convention |

## Pinned numbers

| Quantity | Value | Source |
|---|---|---|
| Drag delta uniform | **16 B** | T-151.6 |
| `uniform_bytes_last_frame` while drag | **80** (64+16) if unchanged | T-151.6 |
| Vitest baseline | **391** | T-151.7 |
| Wasm baseline | **4,063,911 B** | T-151.7 |

## Tasks

1. Fix selection sync vs cluster short-lane + post-drag restore (B1).
2. Split drag start vs per-frame delta; stop per-frame buffer recreate (B2).
3. Unify wheel/pan/resize coordinate space (B3).
4. Automated tests where cheap + verify log S1–S3; tag **T-151.7.1**.

## Verify

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings
cargo test -p map-engine-core --all-features
cargo test -p map-engine-render
cargo build --workspace
make wasm
cd apps/website/frontend && npm test && npm run build && npm run lint
! grep -l map_engine_wasm_bg dist/assets/index-*.js
```

## Manual acceptance

- **S1:** Rapid select / Ctrl-toggle / deselect / empty-clear — rings always match selection (detail + after leaving cluster zoom).
- **S2:** Select ~1000, drag — FPS drop much smaller than ~40; overlay follows; commit OK; `uniform_bytes_last_frame` stays ~80 during drag (no full buffer recreate).
- **S3:** RMB-hold + wheel zoom — world point under cursor stays put (same as Deck / pre-W7).

## Documentation sync (Cursor, after merge)

Registry `T-151.7.1 → shipped`; hub note; `./scripts/ticket sync`.

## Claude Code prompt — T-151.7.1 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the WORKTREE at tbd-reforger-wgpu-spike/ (NOT main).

Implement **T-151.7.1** — interaction hotfix (selection tint / drag FPS / zoom-at-cursor).

═══ PREFLIGHT ═══
  cd /var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike
  test "$(git rev-parse --show-toplevel)" = "$(pwd)"
  git status --porcelain            # empty @ ab6bcb11+ (tag T-151.7)
  # Do NOT checkout branches; do NOT run ./scripts/ticket run
  git lfs pull && make map-assets-link
  make wasm

═══ READ ═══
  1. .ai/artifacts/t151_7_1_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t151_7_1_interaction_hotfix.md
  3. apps/.../wgpu/wgpuSlots.ts          (syncSelection / syncDrag / applySelectionOnlyVisible)
  4. apps/.../WgpuTacticalMap.tsx        (wheel canvas rect vs container pan)
  5. apps/.../tools/useSelectTool.ts     (setDragPreviewDelta every rAF)
  6. crates/map-engine-render/src/engine.rs  (patch_slot_lane / upload_slot_drag_lane)
  7. .ai/artifacts/t151_7_verify_log.md

═══ PROBLEM ═══
  Operator after T-151.7:
  B1 — selection tint sometimes stale (cluster short-lane + patch by full-doc index → silent OOB)
  B2 — drag ~1000 slots ≈ −40 FPS (full syncDrag + create_buffer_init every delta frame)
  B3 — RMB pan + wheel: cursor world point drifts (wheel uses canvas rect; pan uses container)

═══ SHIPPED (do not reopen) ═══
  T-151.7 @ ab6bcb11 — interaction rewire; vitest 391; wasm 4,063,911 B.
  Gesture SM / pick radii / page callbacks stay — only fix the three bugs.

═══ LOCKED ═══
  - B1: never patch short cluster lane with full-doc row indices; selection always visually correct
  - B2: drag start = one overlay upload; per-frame = set_slot_drag_delta only; reuse buffer
  - B3: one CSS origin (container + position:relative) for wheel + pan + resize
  - No W8/W9; no gesture redesign; no forest/slot atlas art changes
  - Commit T-151.7.1: · tag T-151.7.1 · .ai/artifacts/t151_7_1_verify_log.md

═══ DO ═══
  1. Fix syncSelection / cluster selection path (B1); restore tint after clearDragOverlay
  2. Split syncDrag: start once vs delta-only; stop per-frame create_buffer_init (B2)
  3. Unify wheel zoom coords with container (B3)
  4. Tests where cheap; verify log S1–S3; commit + tag T-151.7.1

═══ DO NOT ═══
  - Edit docs/registry/CLAUDE
  - Redesign useSelectTool thresholds/modifiers
  - Start W8 cull or W9 Deck delete
  - git checkout -b / ./scripts/ticket run

═══ VERIFY ═══
  cargo fmt --check
  cargo clippy --all-targets -- -D warnings
  cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings
  cargo test -p map-engine-core --all-features
  cargo test -p map-engine-render
  cargo build --workspace && make wasm
  cd apps/website/frontend && npm test && npm run build && npm run lint

═══ MANUAL ═══
  S1: select/deselect/Ctrl-toggle — tint always correct (detail + cluster)
  S2: drag ~1000 — FPS drop ≪ 40; delta-uniform only per frame
  S3: RMB + wheel — world point under cursor stays put

═══ RETURN ═══
  - SHA + tag T-151.7.1
  - .ai/artifacts/t151_7_1_verify_log.md
  - Ready for Cursor doc sync.
```
