# T-151.6 — mission entities zero-copy: slots, selection, drag, clusters (W6)

**Status:** **shipped** @ `033ff715` (tag **T-151.6**, 2026-07-09) · verify log
[`t151_6_verify_log.md`](../../../.ai/artifacts/t151_6_verify_log.md) · **Program:**
[`t151_wgpu_engine_program.md`](t151_wgpu_engine_program.md) · **Executor:** claude-code ·
**Worktree:** `tbd-reforger-wgpu-spike/` (absolute:
`/run/media/system/Disk_2/Projects/TBD-Reforger/tbd-reforger-wgpu-spike`; do **not** touch `main`) ·
**Baseline:** `a98fb421` (tag **T-151.5.1** — verify log
[`t151_5_1_verify_log.md`](../../../.ai/artifacts/t151_5_1_verify_log.md)).

**Shipped notes:** Dedicated ring+disc atlas; SoA→GPU; selection tint; T-061 drag delta
uniform; T-065 clusters; vitest **379**; wasm **4,063,618 B**. Interaction = **T-151.7**.

## In one sentence

Draw **mission slots** (ring markers), **selection tint**, **T-061 drag overlay**, and **T-065
cluster discs** on `WgpuTacticalMap` from the Rust `MissionDoc` SoA (`slot_xy_ptr` / `slot_len`)
with dirty-range uploads — zero-copy render path; gesture rewire stays **T-151.7**.

## Problem

W5 draws world glyphs; the editor’s authored entities still only appear on the Deck mount.
`?engine=wgpu` shows basemap + world vectors + trees but **no ORBAT slots**, so operators cannot
edit missions on the wgpu path. Rust already materializes slot SoA (`refresh` / `slot_xy_ptr` /
`slot_len`); `ClusterIndex` / `SlotIndex` exist in wasm. Missing: GPU lanes + FE bridge that
pushes SoA → IconInstanced (or equivalent) with T-061/T-065 contracts.

## Goal

1. **Slot ring lane** on wgpu — Class R visual vs Deck `useIconLayer` (primary ring 20 px /
   selected 28 px / Aegis primary + tactical yellow).
2. **Zero-copy / dirty-range uploads** from `MissionDoc` SoA after `refresh()` — O(edited) on
   add / bulk-add / remove / move; full re-upload only on `_applySnapshot` (undo/boot).
3. **Selection tint** — per-instance flag (or dual base/selected upload) matching
   `selection.ids`; O(selection), not O(n) rebuild of positions.
4. **Drag overlay** — T-061 dual-layer: base excludes dragged ids; small overlay + **delta
   uniform** (no per-frame full re-upload); commit restores.
5. **Cluster discs** — existing Rust `ClusterIndex` under exact gates (`> 500` slots,
   `zoom ≤ ZOOM_CLUSTER_MAX` (−4), drill +1). Detail mode otherwise.
6. **`slotIconCache`** may slim to index maintenance for Deck oracle; wgpu must not depend on
   Deck packing the full icon array every frame.
7. **Gates** from program hub W6 (instance count == `slot_len`, selection flags, drag math,
   undo SoA sample, optional 500k stress note).

## Out of scope

- Gesture / camera rewire (`useSelectTool` → ULP-0) — **T-151.7**.
- World glyph / forest / building / road changes (T-151.5 / 5.1 locked).
- Retiring Deck slot layers (T-151.9).
- Markers / vehicles (T-069 / T-070).
- Registry/docs edits (Cursor-owned).
- Forest Path B / T-149 polish (deferred until Fable 5).

## Locked decisions

| # | Decision | Rationale |
|---|---|---|
| L1 | Source of truth for positions = Rust `MissionDoc` SoA after `refresh()` (`slot_xy_ptr` / `slot_len`) — **not** re-deriving from Zustand `slotsById` for the GPU buffer | Program hub D1 / W6 |
| L2 | New lane roles: **Slots** + **SlotDrag** (+ **Clusters**); draw after world badges, before grid/marquee | Layer stack |
| L3 | Reuse `PipelineKind::IconInstanced` (≤ 20 B). Slot/cluster art = **small dedicated atlas** (procedural ring + solid disc) — **not** the 28-key world-glyphs atlas | Deck uses canvas ring/disc, not world glyphs |
| L4 | Dirty-range uploads mirror `_patch*` classes; full upload on `_applySnapshot` only | Hub W6 |
| L5 | Selection = per-instance flag / tint column; drag = exclude + overlay + **delta uniform** | T-061 |
| L6 | Cluster gates verbatim: `CLUSTER_SLOT_THRESHOLD=500`, `ZOOM_CLUSTER_MAX=-4` (`state/constants.ts`) | T-065 |
| L7 | Interaction callbacks / pick rewire stay W7 — W6 may read Zustand `selection` / `dragPreview*` for tint/overlay when those are set (Deck dual-mount or future W7) | Scope |
| L8 | Thin TS glue only (subscribe + upload); geometry/cluster math stays Rust | Rust-first |
| L9 | W2–W5.1 regressions green; vitest ≥ **374** | Regression |
| L10 | Commit `T-151.6:` · tag **`T-151.6`** · verify log `.ai/artifacts/t151_6_verify_log.md` | House convention |

## Pinned numbers

| Quantity | Value | Source |
|---|---|---|
| Slot ring size (px) | **20** base / **28** selected | `useIconLayer.ts` |
| Colors | primary `[173,198,255]` / selected `[250,204,21]` | Aegis |
| `ZOOM_CLUSTER_MAX` | **−4** | `constants.ts` |
| `CLUSTER_SLOT_THRESHOLD` | **500** | `constants.ts` |
| Icon instance layout | ≤ **20 B** | W5 |
| Vitest baseline | **374** | T-151.5.1 |
| Wasm baseline | **4,055,075 B** | T-151.5.1 |

## Tasks

1. Slot + cluster atlas (1–2 glyphs) + `LaneRole::{Slots,SlotDrag,Clusters}`.
2. Engine APIs: upload/patch slot instances from SoA bytes; selection flags; drag delta uniform;
   cluster disc upload from `ClusterIndex` query.
3. FE `useWgpuSlots` (or controller): on doc change → `md.wasm.refresh()` → dirty upload; honor
   cluster mode from zoom + slot count.
4. Wire into `WgpuTacticalMap` draw order.
5. Automated gates + verify log; tag **T-151.6**.

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

- **S1:** `?engine=wgpu` on a mission with slots — rings visible at default zoom (−2); count
  matches Deck dual-mount / OBJ readout.
- **S2:** Select slots (Deck mount or store) → yellow tint on wgpu when selection mirrored;
  clear → primary.
- **S3:** Drag (Deck) / scripted `dragPreview*` → overlay follows delta; base excludes ids; no
  full-buffer re-upload every frame (`uniform_bytes` / stats note).
- **S4:** ≥ 500 slots, zoom ≤ −4 → cluster discs; zoom in → detail rings; drill behavior
  unchanged until W7 if pick not wired.
- **S5:** Undo/redo → instance count == `slot_len`; sampled SoA matches.

## Documentation sync (Cursor, after merge)

Registry `T-151.6 → shipped`; hub note; `./scripts/ticket sync`.

## Claude Code prompt — T-151.6 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the WORKTREE at tbd-reforger-wgpu-spike/ (NOT main).

Implement **T-151.6** — mission entities zero-copy (slots, selection tint, drag overlay, clusters).

═══ PREFLIGHT ═══
  cd /run/media/system/Disk_2/Projects/TBD-Reforger/tbd-reforger-wgpu-spike
  test "$(git rev-parse --show-toplevel)" = "$(pwd)"
  git status --porcelain            # empty @ a98fb421+ (tag T-151.5.1)
  # Do NOT checkout branches; do NOT run ./scripts/ticket run
  git lfs pull && make map-assets-link
  make wasm

═══ READ ═══
  1. .ai/artifacts/t151_6_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t151_6_mission_entities.md
  3. docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md  (§T-151.6 / W6)
  4. crates/map-engine-wasm/src/lib.rs          (MissionDoc refresh / slot_xy_ptr / slot_len)
  5. crates/map-engine-core/src/doc/{store,soa}.rs
  6. crates/map-engine-core/src/spatial/cluster.rs
  7. crates/map-engine-render/src/{engine,scene}.rs  (IconInstanced, LaneRole, upload_icon_lane)
  8. apps/.../state/{wasmDoc,ydoc,useMapStore,slotIconCache,slotClusterIndex,constants}.ts
  9. apps/.../layers/{useIconLayer,useClusterIconLayer}.ts   (Deck oracle)
  10. apps/.../WgpuTacticalMap.tsx + wgpu/wgpuWorldLoader.ts (W5 pattern)

═══ PROBLEM ═══
  wgpu mount has world layers + glyphs but no mission slots. Rust MissionDoc already exposes
  SoA pointers; ClusterIndex exists. Need GPU lanes + thin FE bridge with T-061 drag dual-layer
  and T-065 cluster gates. Gesture/camera rewire is T-151.7 — do not redesign useSelectTool.

═══ SHIPPED (do not reopen) ═══
  T-151.5.1 @ a98fb421 — DENSITY_ISO=2 Rust SoT; glyph-band green hide; vitest 374; wasm 4,055,075 B.
  T-151.5 @ 0b7621ed — IconInstanced 20 B + world glyph atlas.
  Forest Path B / T-149 polish deferred (operator: good enough until Fable 5).

═══ LOCKED ═══
  - Positions from MissionDoc SoA (refresh → slot_xy_ptr / slot_len) — not Zustand→GPU as SoT
  - LaneRoles Slots + SlotDrag + Clusters; IconInstanced ≤20 B; dedicated ring/disc atlas
  - Dirty-range uploads on _patch*; full upload on _applySnapshot only
  - Selection tint O(selection); drag = exclude + overlay + delta uniform (T-061)
  - Cluster: >500 slots AND zoom ≤ −4 (constants.ts)
  - No W7 gesture rewire; no forest/world glyph retune; no Deck deletion
  - Commit T-151.6: · tag T-151.6 · .ai/artifacts/t151_6_verify_log.md

═══ DO ═══
  1. Slot/cluster atlas + LaneRoles + engine upload/patch/selection/drag-delta APIs
  2. useWgpuSlots (or controller): doc subscribe → refresh → dirty upload; cluster mode
  3. Wire WgpuTacticalMap draw order (after badges, before grid/marquee)
  4. Automated gates: instance count == slot_len after add/paste/delete/undo; selection flags;
     drag delta math; cluster gate unit tests
  5. Verify log S1–S5; commit + tag T-151.6

═══ DO NOT ═══
  - Edit docs/registry/CLAUDE
  - Rewire useSelectTool / camera unproject (W7)
  - Feed GPU from slotsById as source of truth (SoA wins)
  - Reuse world-glyphs atlas for slot rings
  - Touch forest iso / Path B / T-149
  - Start T-151.7 / T-151.9 Deck deletion
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
  S1: wgpu mission — slot rings visible; count matches Deck/OBJ
  S2: selection tint yellow ↔ primary
  S3: drag overlay + delta uniform (no full re-upload/frame)
  S4: cluster discs @ zoom ≤ −4 when >500 slots
  S5: undo/redo instance count == slot_len

═══ RETURN ═══
  - SHA + tag T-151.6
  - .ai/artifacts/t151_6_verify_log.md
  - Ready for Cursor doc sync.
```
