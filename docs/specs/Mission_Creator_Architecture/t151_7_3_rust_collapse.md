# T-151.7.3 — Rust collapse: slot GPU bridge out of TypeScript

**Status:** **ready** (executor queue) · **Program:**
[`t151_wgpu_engine_program.md`](t151_wgpu_engine_program.md) · **Executor:** claude-code ·
**Worktree:** `tbd-reforger-wgpu-spike/` (absolute:
`/var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike`; do **not** touch `main`) ·
**Baseline:** `69ca1c08` (HEAD after **T-151.7.2** restore scroll-wheel; tags **T-151.7.1**
`fa6ad959`, **T-151.7.2** `64c64d98`).

## In one sentence

Move **slot / selection / drag / cluster GPU policy** from fat TypeScript (`wgpuSlots.ts`
~521 LOC) into **Rust** (`RenderEngine` + `slots_gpu` + `MissionDoc`), leaving TS as dumb
React subscribe + thin wasm calls — as much Rust as possible.

## Problem (operator / architecture)

W6–W7.x shipped a working wgpu editor path, but **orchestration lived in TypeScript**:
`WgpuSlotsController` owns SoA→GPU upload, selection tint, drag phases, cluster lane flips.
Hotfixes (7.1 / 7.2) grew that TS file further. Program north star is **Rust owns the engine**;
TS owns dumb UI (React, pointer events, Zustand). Current split is wrong.

## Goal

1. **`SlotGpuBridge` (or equivalent) in Rust** — single place that:
   - binds `MissionDoc`, `refresh()` → pack from `slot_xy` SoA
   - applies selection ids → tint/size (detail full-n; cluster selection-only policy)
   - drag: start once / delta-only / clear (T-061; no per-frame buffer recreate)
   - cluster gate + lane flip from camera zoom + `slot_len`
   - `px_to_m` from camera zoom
2. **Public wasm surface ≤ ~10 methods** (see Locked). Low-level
   `upload_slot_lane` / `patch_slot_lane` become **private** to the bridge (or stay but unused
   from TS).
3. **Collapse `wgpuSlots.ts` → ~40–60 LOC** — ctor, atlas bytes once, Zustand →
   `set_selection` / `set_drag` / `on_camera` / `set_cluster_markers`, dispose.
4. **Delete TS mirrors** of `slots_gpu` pack/gate/constants (keep canvas atlas generator only).
5. Behavior parity with post-7.2 operator fixes (tint correct, drag FPS, zoom-at-cursor) —
   **do not regress** 7.1/7.2; move the *logic*, don’t reintroduce bugs.
6. Verify log + tag **T-151.7.3**.

## Out of scope

- Porting `useSelectTool` gesture SM into Rust (stays TS — dumb UI / pointer).
- Porting supercluster / SlotIndex pick into a new home (already wasm; FE wrappers OK).
- Baking procedural ring atlas in Rust (canvas `slotAtlas.ts` draw stays OK this slice).
- W8 cull / W9 Deck delete / forest retune.
- Rewriting world glyph / basemap loaders (already mostly Rust + thin fetch).
- Registry/docs (Cursor-owned).

## Locked decisions

| # | Decision | Rationale |
|---|---|---|
| L1 | **Rust owns** slot GPU sync policy; TS must not reimplement pack/tint/drag/cluster | North star |
| L2 | Suggested wasm API (names flexible, semantics fixed): `bind_mission_doc`, `set_selection`, `set_drag` (ids+dx+dy; empty=clear), `on_camera_changed`, `set_cluster_markers`, `cluster_mode()`, `ensure_slot_atlas`, `slot_stats_json`, `clear_slots` | ≤10 surface |
| L3 | Reuse existing `slots_gpu::pack_*` / `cluster_mode` / `px_to_m_at_zoom` in-process — **delete** TS duplicates | One SoT |
| L4 | Drag phase machine in Rust (start / delta / clear) — same contracts as 7.1 `classifyDragTransition` | Perf |
| L5 | Selection: full re-materialize from SoA+mask (7.2 lesson) — no OOB patch-by-index into short lanes | Correctness |
| L6 | `wgpuSlots.ts` target **≤ 60 LOC**; prove with `wc -l` in verify log | Measurable |
| L7 | KEEP in TS: React effects, pointer SM, `mapCamera` CSS adapter, canvas atlas pixels, marquee store→`upload_marquee` | Dumb UI |
| L8 | W2–W7.2 regressions green; vitest ≥ **393** | Regression |
| L9 | Commit `T-151.7.3:` · tag **`T-151.7.3`** · verify log `.ai/artifacts/t151_7_3_verify_log.md` | House convention |

## Architecture note (binding for later slices)

After this slice, **new map-engine policy goes in Rust first**. TS may only:
- call wasm,
- handle DOM/React/pointer,
- hold Deck oracle until T-151.9.

Do **not** grow another `wgpu*Controller` with business logic.

## Pinned numbers

| Quantity | Before | After (target) |
|---|---|---|
| `wgpuSlots.ts` LOC | **~521** | **≤ 60** |
| Vitest baseline | **393** | ≥ 393 |
| Wasm baseline | **~4,063,911 B** | record delta |

## Tasks

1. Implement Rust `SlotGpuBridge` (or engine methods) + wasm exports.
2. Rewire `useWgpuSlots` / thin `wgpuSlots.ts` to the new API; delete TS pack/gate mirrors.
3. Ensure 7.1/7.2 behaviors preserved (tint, drag delta-only, camera SoT for zoom).
4. Unit tests in `map-engine-core` / `map-engine-render` for sync/drag/cluster; slim FE parity.
5. Verify log with before/after `wc -l`; tag **T-151.7.3**.

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
wc -l src/features/tactical-map/wgpu/wgpuSlots.ts   # must be ≤ 60
```

## Manual acceptance

- **S1:** Select / deselect / Ctrl-toggle — tint always matches (detail + cluster).
- **S2:** Drag ~1000 — FPS drop ≪ 40; per-frame = delta uniform only.
- **S3:** RMB + wheel — zoom-at-cursor stable; scroll-wheel zoom works.
- **S4:** `wgpuSlots.ts` ≤ 60 LOC; no TS reimplementation of `pack_slot_instances` / `cluster_mode`.

## Documentation sync (Cursor, after merge)

Registry `T-151.7.3 → shipped`; hub note; `./scripts/ticket sync`.

## Claude Code prompt — T-151.7.3 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the WORKTREE at tbd-reforger-wgpu-spike/ (NOT main).

Implement **T-151.7.3** — Rust collapse: move slot GPU bridge out of TypeScript.

═══ PREFLIGHT ═══
  cd /var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike
  test "$(git rev-parse --show-toplevel)" = "$(pwd)"
  git status --porcelain            # empty @ 69ca1c08+ (after T-151.7.2)
  # Do NOT checkout branches; do NOT run ./scripts/ticket run
  git lfs pull && make map-assets-link
  make wasm
  wc -l apps/website/frontend/src/features/tactical-map/wgpu/wgpuSlots.ts  # ~521 today

═══ READ ═══
  1. .ai/artifacts/t151_7_3_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t151_7_3_rust_collapse.md
  3. docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md  (D1 + architecture note)
  4. apps/.../wgpu/wgpuSlots.ts          (FAT — collapse this)
  5. apps/.../wgpu/{useWgpuSlots,slotAtlas}.ts
  6. crates/map-engine-core/src/slots_gpu.rs
  7. crates/map-engine-render/src/engine.rs  (slot lane APIs)
  8. crates/map-engine-wasm/src/lib.rs      (MissionDoc SoA)
  9. .ai/artifacts/t151_7_1_verify_log.md + t151_7_2_verify_log.md  (behaviors to preserve)

═══ PROBLEM ═══
  Slot/selection/drag/cluster GPU policy lives in ~521 LOC TypeScript. North star = Rust owns
  the engine; TS = dumb UI only. Collapse the bridge into Rust without regressing 7.1/7.2 fixes.

═══ SHIPPED (do not reopen as TS fixes) ═══
  T-151.7 @ ab6bcb11 — interaction rewire
  T-151.7.1 @ fa6ad959 — tint / drag FPS / zoom origin
  T-151.7.2 @ 64c64d98 (+ 69ca1c08 wheel restore) — selection rematerialize + camera SoT
  Do NOT keep growing wgpuSlots.ts — move logic to Rust.

═══ LANGUAGE GATE (MANDATORY — D5) ═══
  Rust OWNS: SoA→GPU sync, selection tint, drag phases, cluster lane flip, pack_*,
  px_to_m, camera math already in OrthoCamera.
  TypeScript ONLY: React subscribe, canvas atlas pixels once, thin wasm calls
  (bind_doc / set_selection / set_drag / on_camera / set_cluster_markers).
  STOP IF: about to add sync/pack/LOD/drag policy in .ts → put it in crates/map-engine-* .
  LOC budget: wgpuSlots.ts ≤ 60 (was ~521). Fail the slice if over budget.

═══ LOCKED ═══
  - Rust owns sync/selection/drag/cluster/px_to_m policy
  - wasm surface ≤ ~10 methods (bind_doc, set_selection, set_drag, on_camera, set_cluster_markers,
    cluster_mode, ensure_slot_atlas, slot_stats_json, clear_slots)
  - wgpuSlots.ts ≤ 60 LOC after; delete TS pack/gate mirrors
  - Preserve 7.1/7.2 behavior (full rematerialize selection; drag delta-only; camera SoT zoom)
  - KEEP TS: React, useSelectTool, mapCamera, canvas atlas pixels, marquee thin call
  - No W8/W9; no gesture SM port to Rust this slice
  - Commit T-151.7.3: · tag T-151.7.3 · .ai/artifacts/t151_7_3_verify_log.md

═══ DO ═══
  1. Implement Rust SlotGpuBridge (or engine methods) using slots_gpu + MissionDoc SoA
  2. Export thin wasm API; hide low-level lane uploads from TS
  3. Rewrite wgpuSlots.ts to ≤60 LOC thin adapter; delete TS pack duplicates
  4. Native + FE tests; verify log with wc -l before/after; commit + tag T-151.7.3

═══ DO NOT ═══
  - Edit docs/registry/CLAUDE
  - “Fix” by adding more TypeScript policy
  - Port useSelectTool into Rust
  - Start W8 cull / W9 Deck delete
  - git checkout -b / ./scripts/ticket run

═══ VERIFY ═══
  cargo fmt --check
  cargo clippy --all-targets -- -D warnings
  cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings
  cargo test -p map-engine-core --all-features
  cargo test -p map-engine-render
  cargo build --workspace && make wasm
  cd apps/website/frontend && npm test && npm run build && npm run lint
  wc -l src/features/tactical-map/wgpu/wgpuSlots.ts   # ≤ 60

═══ MANUAL ═══
  S1: selection tint always correct
  S2: drag ~1000 FPS OK (delta-only)
  S3: zoom-at-cursor + scroll wheel
  S4: wgpuSlots.ts ≤ 60; no TS pack_slot_instances

═══ RETURN ═══
  - SHA + tag T-151.7.3
  - .ai/artifacts/t151_7_3_verify_log.md (include wc -l before/after)
  - Ready for Cursor doc sync.
```
