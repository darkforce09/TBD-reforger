# T-151.7 — interaction rewire + parity suite (W7)

**Status:** **ready** (executor queue) · **Program:**
[`t151_wgpu_engine_program.md`](t151_wgpu_engine_program.md) · **Executor:** claude-code ·
**Worktree:** `tbd-reforger-wgpu-spike/` (absolute:
`/var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike`; do **not** touch `main`) ·
**Baseline:** `033ff715` (tag **T-151.6** — verify log
[`t151_6_verify_log.md`](../../../.ai/artifacts/t151_6_verify_log.md)).

## In one sentence

Wire **editor interaction** on `WgpuTacticalMap` to the ULP-0 `OrthoCamera` — same gesture
machine as Deck (`useSelectTool`), same picks (`SlotIndex` / `ClusterIndex`), same page
callbacks — so selection / drag / marquee / drop / Space / CUR work on `?engine=wgpu` without
redesigning UX.

## Problem

T-151.6 draws slot rings, but wgpu still cannot edit: `WgpuTacticalMap` ignores `onReady` /
`onCursorMove` / `onAssetDrop` / `onEntityActivate`; LMB is raw `engine.pan` (conflicts with
select/drag/marquee); all `view.makeViewport(...).unproject` call sites are Deck-only. Operator
must script Zustand for selection/drag. **W7** swaps camera math to Rust ULP-0 and wires the
existing gesture machine — does **not** invent a new interaction model.

## Goal

1. **Camera adapter:** every Deck `makeViewport` / `unproject` consumer can use
   `OrthoCamera` / `RenderEngine` (`unproject_xy`, `set_view`, `pan`, `zoom_at`,
   `visible_world_rect`) with Class R parity vs Deck oracle (`orthoCamera.parity` / ULP-0).
2. **Expose missing APIs** on the live engine if needed (`unproject_xy` on `RenderEngine` /
   wasm; `flyTo` via `set_view` for `TacticalMapApi`).
3. **`useSelectTool` on wgpu:** same SM (4 px threshold, rAF coalesce, pending-left / move /
   marquee / pan, Ctrl toggle, cluster drill) — only camera calls change. Replace raw LMB pan
   on `WgpuTacticalMap` with the tool (middle/right pan; left = select/drag/marquee).
4. **Wire props** on wgpu mount to match Deck: `onReady` (+ `flyTo`), `onCursorMove` (rAF +
   DEM `sampleElevation` z), `onAssetDrop`, `onEntityActivate` (dbl-click pick),
   `onEntitiesMove` / selection via existing store paths.
5. **Picks:** wasm `SlotIndex` (4 px) + `ClusterIndex` (48 px); world `PointIndex` unchanged.
6. **Keyboard / DnD contracts unchanged:** Ctrl+C/V (500 select cap), Space → `flyTo`, Delete,
   dbl-click ≤ 1 guard, `ASSET_DND_MIME`.
7. **Interaction parity suite:** scripted pointer/keyboard sequences on Deck vs wgpu →
   identical `selection.ids` + identical `encode_state` after each script (Class R); CUR/SEL Z
   == `sampleElevation` (Class R).

## Out of scope

- Culling / density ladder (W8).
- Deck retirement / default flip (W9).
- Slot GPU / forest / glyph retune (W5–W6 locked).
- New gesture UX (no redesign of thresholds, modifiers, or marquee rules).
- Markers / vehicles / ruler / LoS (W10).
- Registry/docs edits (Cursor-owned).

## Locked decisions

| # | Decision | Rationale |
|---|---|---|
| L1 | Gesture machine **unchanged** — only camera unproject / viewport calls swap to ULP-0 | Hub W7 |
| L2 | Camera SoT = Rust `OrthoCamera` (already ULP-0 vs Deck); FE must not keep a parallel Deck viewport for wgpu path | Spike parity |
| L3 | `RenderEngine` exposes `unproject_xy` (and `flyTo`/`set_view` already) for the mount | Investigator gap |
| L4 | Picks stay on wasm `SlotIndex` / `ClusterIndex` — no Deck GPU pick | T-063 |
| L5 | `MissionCreatorPage` passes the **same** interaction callbacks to wgpu as Deck | Dual-mount |
| L6 | CUR z = `sampleElevation` when DEM ready (same as Deck TacticalMap) | T-091 |
| L7 | Parity suite: selection + `encode_state` Class R after scripted sequences | Hub gate |
| L8 | W2–W6 regressions green; vitest ≥ **379** | Regression |
| L9 | Commit `T-151.7:` · tag **`T-151.7`** · verify log `.ai/artifacts/t151_7_verify_log.md` | House convention |

## Pinned numbers

| Quantity | Value | Source |
|---|---|---|
| Pick radius (slots) | **4 px** | `slotSpatialIndex` / useSelectTool |
| Cluster pick | **48 px** | cluster drill |
| Gesture threshold | **4 px** | useSelectTool |
| `ZOOM_CLUSTER_MAX` | **−4** | constants (unchanged) |
| Vitest baseline | **379** | T-151.6 |
| Wasm baseline | **4,063,618 B** | T-151.6 |

## Tasks

1. Engine/wasm: `unproject_xy` on live `RenderEngine` if missing; thin FE camera helper.
2. Abstract or dual-path `useSelectTool` camera (Deck viewport vs engine unproject).
3. Wire `WgpuTacticalMap` props + replace raw LMB pan; `onReady` → `flyTo` via `set_view`.
4. Cursor rAF + DEM z; asset-drop; dbl-click activate.
5. Interaction parity suite + verify log; tag **T-151.7**.

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

- **S1:** `?engine=wgpu` — click select, Ctrl toggle, empty-click clear (same as Deck).
- **S2:** Marquee multi-select; Delete removes; undo restores rings + `slot_len`.
- **S3:** Drag-move slots — overlay + commit; T-061 feel; no LMB-pan stealing drag.
- **S4:** Space centers selection; asset drop places slot; dbl-click opens Attributes (≤1).
- **S5:** CUR X/Y/Z tracks pointer (DEM z when ready); cluster drill at zoom ≤ −4.
- **S6:** Parity suite green (or documented scripted A/B) — selection + `encode_state` match Deck.

## Documentation sync (Cursor, after merge)

Registry `T-151.7 → shipped`; hub note; `./scripts/ticket sync`.

## Claude Code prompt — T-151.7 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the WORKTREE at tbd-reforger-wgpu-spike/ (NOT main).

Implement **T-151.7** — interaction rewire + parity suite (ULP-0 camera on wgpu mount).

═══ PREFLIGHT ═══
  cd /var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike
  test "$(git rev-parse --show-toplevel)" = "$(pwd)"
  git status --porcelain            # empty @ 033ff715+ (tag T-151.6)
  # Do NOT checkout branches; do NOT run ./scripts/ticket run
  git lfs pull && make map-assets-link
  make wasm

═══ READ ═══
  1. .ai/artifacts/t151_7_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t151_7_interaction_rewire.md
  3. docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md  (§T-151.7 / W7)
  4. apps/.../tools/useSelectTool.ts
  5. apps/.../TacticalMap.tsx          (Deck oracle — full interaction)
  6. apps/.../WgpuTacticalMap.tsx      (raw LMB pan today — replace)
  7. apps/.../mission-creator/MissionCreatorPage.tsx  (callback wiring)
  8. crates/map-engine-core/src/camera/ortho.rs
  9. crates/map-engine-render/src/engine.rs  (set_view / pan / zoom_at — add unproject_xy)
  10. crates/map-engine-wasm/src/lib.rs  (OrthoCameraJs / SlotIndex / ClusterIndex)
  11. apps/.../state/{slotSpatialIndex,slotClusterIndex}.ts
  12. apps/.../dem/*  (sampleElevation for CUR z)
  13. features/_wasm/orthoCamera.parity.test.ts

═══ PROBLEM ═══
  W6 draws slots but wgpu cannot edit: no useSelectTool, no onReady/cursor/drop/activate,
  LMB steals pan. Swap every makeViewport/unproject consumer to ULP-0 OrthoCamera; wire the
  SAME gesture machine + page callbacks. Do not redesign UX.

═══ SHIPPED (do not reopen) ═══
  T-151.6 @ 033ff715 — slot rings/selection/drag/clusters; vitest 379; wasm 4,063,618 B.
  T-151.5.1 @ a98fb421 — forest fidelity (good enough; T-149 deferred).

═══ LOCKED ═══
  - Gesture SM unchanged — only camera calls → ULP-0
  - RenderEngine.unproject_xy (+ flyTo via set_view) for wgpu mount
  - Picks: SlotIndex 4 px + ClusterIndex 48 px (wasm) — no Deck GPU pick
  - MissionCreatorPage: same callbacks on wgpu as Deck
  - CUR z = sampleElevation when DEM ready
  - Parity suite: selection + encode_state Class R vs Deck scripts
  - No W8 cull / W9 Deck delete / forest retune
  - Commit T-151.7: · tag T-151.7 · .ai/artifacts/t151_7_verify_log.md

═══ DO ═══
  1. Expose unproject_xy on RenderEngine/wasm if missing; thin FE camera helper
  2. Dual-path or abstract useSelectTool camera (Deck viewport vs engine)
  3. Wire WgpuTacticalMap: useSelectTool; onReady/flyTo; cursor+DEM; drop; dbl-click;
     remove conflicting raw LMB pan
  4. MissionCreatorPage: pass interaction props to wgpu path
  5. Interaction parity suite + verify log S1–S6; commit + tag T-151.7

═══ DO NOT ═══
  - Edit docs/registry/CLAUDE
  - Redesign gesture thresholds / modifiers / marquee rules
  - Start W8 culling or W9 Deck deletion
  - Change slot GPU pack / forest iso
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
  S1: click / Ctrl toggle / clear on wgpu
  S2: marquee + Delete + undo
  S3: drag-move (no LMB-pan steal)
  S4: Space flyTo + asset drop + dbl-click Attributes
  S5: CUR X/Y/Z + cluster drill
  S6: parity suite / A/B selection + encode_state

═══ RETURN ═══
  - SHA + tag T-151.7
  - .ai/artifacts/t151_7_verify_log.md
  - Ready for Cursor doc sync.
```
