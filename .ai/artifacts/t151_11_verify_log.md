# T-151.11 verify log — remediation program (fix every T-151.10 / T-151.10.1 finding)

**Worktree:** `tbd-reforger-wgpu-spike/` · **Baseline:** T-151.10.1 `40def01a` · **Date:** 2026-07-10 ·
**Executor:** Claude Code (Fable 5) · **Tracker:** [`t151_10_fable_audit_report.md`](t151_10_fable_audit_report.md)
§Remediation round · **Operator gates:** [`t151_operator_signoff.md`](t151_operator_signoff.md)

**Slices / tags:**

| Slice | SHA | Scope |
|-------|-----|-------|
| T-151.11.1 | `b81317c5` | draw order (grid + compute-culled trees), marquee fill+border parity, dead twin, `marquee_self_check`, 4 native lane-order pins (new pure `draw_order.rs`) |
| T-151.11.2 | `b78205c0` | `set_camera_bounds` + mount call, DEV-gated HUD/route/hooks, prod damage-driven render, comment refresh, dead-code sweep (with 2 corrections), patch contract |
| T-151.11.3 | `c7bb5bd0` | 6 wasm policy exports + FE swaps, core-owned ingest budget, buildings full-lane toggle, forest cache LRU, oracle banners |
| T-151.11.4 | `2c3237fd` | f32-aligned cull rule + REAL GPU counter readback + `compute_cull_self_check`, tree check registered+executed, committed GPU harness + make target, Range satellite preview (+4 tests), CI jobs |
| T-151.11.5 | (this commit) | tracker accounting, operator sign-off checklist, this log |

## Per-slice battery results (all exit 0 at each slice's commit)

Commands: `cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` ·
`cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings` ·
`cargo test -p map-engine-core --all-features` · `cargo test -p map-engine-render` ·
`cargo build --workspace` · `make wasm` · `npm test` · `npm run build` · `npm run lint` ·
`wc -l wgpuSlots.ts`.

| Slice | core tests | render tests | vitest | wasm bytes | notes |
|-------|-----------|--------------|--------|------------|-------|
| 11.1 | 144 (−1: dead twin's test removed) | **25** (+4 lane-order pins) | 281 | 4,137,863 | pins run natively via new pure `draw_order.rs` (engine module is wasm32-gated) |
| 11.2 | 144 | 25 | 281 | 4,137,463 | prod `dist` proven free of the spike chunk + `/_spike/wgpu` string |
| 11.3 | **147** (+3: budget truth table, toggle hide/restore, zoom gate) | 25 | 281 | 4,139,285 | importer-graph grep: zero non-test importers of the 4 oracle files |
| 11.4 | 147 | 25 | **285** (+4 preview helpers) | 4,150,664 | harness run below |
| 11.5 | — artifacts only — | | | | |

`wgpuSlots.ts` = **56** LOC at every slice (≤ 60 LANGUAGE GATE held; all policy landed in crates).

## Committed GPU harness — first full run (T-151.11.4, `make verify-wgpu-gpu`, exit 0)

`scripts/website/verify-wgpu-gpu.mjs` (spawns vite :5199, chromium from the playwright cache,
SwiftShader WebGL2, raw CDP). **8/8 `allPass: true`:**

| Check | Result | Note |
|-------|--------|------|
| calibration | PASS (7/7 probes byte-exact) | T-151.0 regression |
| texture | PASS (3/3 byte-exact) | T-151.1 |
| worldBuilding | PASS (3/3 byte-exact) | T-151.3 S4 |
| seaBand | PASS (2/2 byte-exact) | T-151.4 L11 |
| roadCenterline | PASS (2/2 byte-exact) | T-151.4 L11 |
| **marquee** | **PASS** (interior ±1, border ±1 at column 299/300 over fill-or-clear, exterior byte-exact) | NEW — T-151.11.1 P-02 |
| **tree** | **PASS** (center tint match + exterior byte-exact) | FIRST EXECUTION EVER — closes C-5-02 (gate existed since T-151.5, never wired) |
| computeCull | PASS (`skipped:true` on WebGL2 by design) | real-hardware WebGPU row in the operator sign-off runs the cpu==gpu proof |

Probe-fix note (honesty): the first marquee run FAILED — the 1 px border on integer column
x=300 rasterizes to pixel 299 and blends over CLEAR, not over the fill
(got `[147,170,218]` = blend(border, clear) exactly). The probe now accepts either column with
either composite, both computed f64-exact ±1 — the failure was the probe's expectation, not the
render; the diagnosis is recorded because it *proves* the border draws with the correct color and
alpha.

## X-03 rule change (Class R domain)

The CPU cull oracle now quantizes the frustum to **f32** and compares in f32 — the exact domain
of the GPU uniform + WGSL kernel (`shader.wgsl::icon_in_frustum`, mirrored op-for-op). CPU==GPU
is equality by construction; the GPU counter is really read back
(`compute_cull_gpu_sampled` distinguishes mirror-until-first-sample), and
`compute_cull_self_check` asserts equality on a deterministic 512-icon field. All 1k-frustum
Class R tests re-pass under the new rule (no expectation changed — the test values sit far from
quantization edges by construction).

## Corrections earned during remediation (tracker updated)

- **`pan` is live** (spike pointer pan) — the 11.2 deletion attempt failed the FE build; restored
  with a caller note. X-05 corrected.
- **`worldSpatialIndex.ts` is an oracle**, not dead — deleting it broke
  `world.pick.parity.test.ts` staging; restored + ORACLE-ONLY banner. B-06 corrected.
- **Cluster policy was already Rust** — B-05 shrank to delegating `deckZoomToSuperZoom`.
- **`getViewport` must stay a frozen snapshot** — wiring it to a live engine viewport would
  feedback-loop during pan; documented on the callback instead (X-05 scope trimmed).

## Residual (not code)

- Operator sign-off session ([`t151_operator_signoff.md`](t151_operator_signoff.md)) closes
  C-ALL-01 + C-8-01 + the WebGPU-only visual rows (X-01 order, computeCull on hardware).
- Cursor doc-sync queue in the tracker (registry rows, hub heading/citations/links/decisions).
- CI runtime proof lands on the first push of this branch (jobs added; branch has no remote).
- Named deferral: slot-lane cull threshold (matrix row 43) → T-069 scale prerequisite.
