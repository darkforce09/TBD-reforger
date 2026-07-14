# T-152.15 verify log — fence/pier/bridge visibility + orientation remediation

**Slice:** T-152.15 · **Executor:** claude-code · **Branch:** `ticket/T-152` · **Tag:** `T-152.15`
**Spec:** `docs/specs/Mission_Creator_Architecture/t152_15_fence_pier_bridge_visibility.md`
**Railing decision (M4):** **Path A — synthetic-always** (operator-chosen).

## Summary

- **S1 fences invisible → fixed:** new `fence` LOD class at `FENCE_MIN_ZOOM = 1.5` (was riding the
  `prop` band z ≥ 3) + a `STRIP_MIN_PX = 1.5` on-screen width floor (`clamp_strip_width_m`).
- **S2 fence yaw → fixed:** `obb_long_axis_endpoints` rebuilt to derive the strip frame directly
  from `obb_corners` (single source of truth with the fill) — the `extra_rot = 90` reconstruction is
  gone; strip long axis ≡ fill long axis by construction. Parity gate added (G2).
- **A1/D5 piers 0/2,299 → fixed:** `compose_pier_strip` always emits (aspect gate + `PIER_ASPECT_MIN`
  + `obb_aspect_ratio` deleted); width `min(hx,hy)·2` clamped ≤ `PIER_STRIP_MAX_WIDTH_M = 6.0`, then
  the px floor. Pier lane decoupled — new `piers_visible()` = Buildings toggle ∧ pier LOD (z ≥ −1.0);
  the fence-gated early-return + vacuous `building_visible` guard removed from `rebuild_strip_buffers`.
- **A13/D6 railings → implemented (Path A):** `BRIDGE_RAILING_RADIUS_M = 8.0` now consumed as the
  rail lateral-offset cap; every bridge emits 2 synthetic deck-edge rails (`compose_bridge_rail_strips`).
- **Bridge deck (Q6):** bridge stays in the fat-square fill but renders a dark casing rim (widened
  along the crossing axis) under a warm-stone deck tint — distinct from gray buildings.

## Census (real Everon data, full-island pin @ z=1.5)

| Quantity | Expected | Measured | Gate |
|----------|----------|----------|------|
| Pier/dock quay strips | 2,299 (≥ 0.99 = 2,276) | **2,299** | G3 PASS (anti-vacuous > 0) |
| Bridge instances | 144 | **144** | — |
| Bridge rail strips | 288 (2 × 144) | **288** | G5 PASS |
| Bridge deck fills / casing fills | 144 / 144 | **144 / 144** | Q6 PASS |
| Fence strips @ z=1.5 | > 0 | **> 0** | (decoupling baseline) |
| Orientation parity (fence + pier prefabs × yaws {0,37,90,123}) | ≤ 0.5° | **≈0° (fp; single-source)** | G2 PASS |

## Gates (spec §Mathematical acceptance matrix)

- **G1 LOD** — `class_visible("fence",1.5)=true /(1.49)=false`; `("pier",-1.0)=true /(-1.01)=false`;
  `prop` unchanged (z ≥ 3). `fence`/`pier` intentionally **not** in `WORLD_RENDER_CLASSES` (that array
  feeds the TS oracle-parity scan — R1). **PASS** (`lod_gates::tests::fence_pier_gate_boundaries`).
- **G2 parity** — strip axis ≡ fill OBB long axis ≤ 0.5° over all 255 fence + pier/dock prefabs ×
  4 yaws (checked ≥ 1020, non-vacuous). **PASS** (`cartographic_strip` synthetic + `t152_3_tests`
  real-prefab).
- **G3 census (anti-vacuous)** — pier strips = 2,299 ≥ 0.99 × 2,299; test asserts `> 0`. **PASS**.
- **G4 pixel floor** — `clamp_strip_width_m(0.35,1.5)·2^1.5 = 1.5`; engaged at z=1.5, disengaged
  (=0.35) at z=3.0. **PASS**.
- **G5 railings** — every bridge = 2 rail strips (rails == 2 × 144 bridges, offset ≤ 8 m). **PASS**.
- **G6 decoupling** — Fences off → 0 fence, piers (2,299) + rails (288) unaffected; Buildings off →
  0 pier + 0 rail, fences unaffected. **PASS**.
- **G7 regression** — full suite below, all exit 0. **PASS**.

## Verify suite (all exit 0)

```
cargo fmt --check                                             # clean
cargo clippy --all-targets --all-features -- -D warnings       # clean
cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings  # clean
cargo test -p map-engine-core --all-features                   # 206 + 5 + 5 pass, 0 fail
make wasm                                                       # ok (bundler pkg regenerated)
cd apps/website/frontend && npm test                           # 48 files / 356 tests pass
npm run build                                                  # tsc -b + vite build ok
npm run lint                                                   # clean
```

## Files changed (code only)

- `crates/map-engine-core/src/world/cartographic_strip.rs` — `STRIP_MIN_PX`,
  `PIER_STRIP_MAX_WIDTH_M`, `clamp_strip_width_m`; single-source `obb_long_axis_endpoints`;
  `compose_fence_strip`/`compose_pier_strip` take `deck_zoom` (pier non-Option);
  `compose_bridge_rail_strips`; deleted `PIER_ASPECT_MIN` + `obb_aspect_ratio`; G2/G4 tests.
- `crates/map-engine-core/src/world/lod_gates.rs` — `FENCE_MIN_ZOOM=1.5`, `PIER_MIN_ZOOM=-1.0`,
  match arms, G1 test. `WORLD_RENDER_CLASSES` left unchanged (R1).
- `crates/map-engine-core/src/world/mod.rs` — re-export update.
- `crates/map-engine-core/src/world/residency.rs` — `fences_visible` (fence class),
  `piers_visible`, `strips_visible`; per-class counts + exact accessors; restructured
  `rebuild_strip_buffers` (pier / rail / fence lanes, decoupled); bridge deck+casing in
  `rebuild_buffers`; `fill_color` bridge arm removed; G2/G3/G5/G6 + casing tests.
- `crates/map-engine-wasm/src/lib.rs` — `piers_visible`, `strips_visible`,
  `pier_strip_segment_count` passthrough; `fences_visible` doc corrected.
- `apps/website/frontend/src/features/tactical-map/wgpu/wgpuWorldLoader.ts` — strip-lane upload
  gate `fences_visible()` → `strips_visible()` (R4; policy stays in Rust).

## Manual acceptance

- **M1** harbor quays @ z=0 — **operator PASS (good enough)** 2026-07-13.
- **M2** rural fences @ z≈1.5 — **operator PASS (good enough)**; orientation follows field edges,
  no systematic 90° combs. Residual imperfection accepted; perfection deferred to future data pass.
- **M3** bridge deck + rails distinct — **operator PASS (good enough)** 2026-07-13.
- **M4** railing decision = **Path A synthetic-always** — recorded (operator-chosen).

## Notes / risks

- **R1** `WORLD_RENDER_CLASSES` deliberately excludes `fence`/`pier` (TS oracle-parity scan
  invariant); the match arms drive the lanes.
- **R2** G2 is exact by construction; residual "combs" from *transposed prefab OBB half-extents*
  are prefab **data** and out of scope (spec forbids re-measuring OBBs) — none observed in the
  automated set.
- **R3** G3 census uses one full-island residency (no per-chunk sum) → no `has_oversized`
  extra-ring / LRU double-count.
- Pre-existing worktree drift left untouched and uncommitted: `.ai/artifacts/map_export_everon.json`,
  `apps/website/src/services/registry_import.rs`, `.ai/tickets/*`, `docs/*` (Cursor owns docs/registry).
- wasm `pkg/` is gitignored (regenerated by `make wasm`); not committed.
