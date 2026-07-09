# T-151.8 — culling + density ladder (W8)

**Status:** **ready** (executor queue) · **Program:**
[`t151_wgpu_engine_program.md`](t151_wgpu_engine_program.md) · **Executor:** claude-code ·
**Worktree:** `tbd-reforger-wgpu-spike/` (absolute:
`/var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike`; do **not** touch `main`) ·
**Baseline:** `804f779a` (tag **T-151.7.3** — verify log
[`t151_7_3_verify_log.md`](../../../.ai/artifacts/t151_7_3_verify_log.md)).

## In one sentence

Productionize **viewport chunk cull**, the **TBDD density → heatmap ladder** when glyph
counts exceed budget, optional **WebGPU compute cull**, and **damage-driven render** — all in
**Rust** (D5 / LANGUAGE GATE). TS stays dumb UI + thin wasm calls.

## Problem

W3–W7 draw resident world glyphs with a hard `INSTANCE_BUDGET` drop (no aggregate rung). At
full Everon + zoomed-out views, either overdraw or silent drops. Continuous rAF always
`render()`s even when nothing changed. Hub W8 closes that before the Deck flip (W9).

## Goal

1. **CPU draw-set cull:** draw = resident chunks ∩ `visible_world_rect` + margin (Class **S**
   vs reference viewport set).
2. **Density ladder:** TBDD grids → density texture; when a class's **exact** visible-count
   estimate exceeds `INSTANCE_BUDGET`, swap that class's glyph batch to a **heatmap quad**
   (aggregate rung). Switch driven by per-chunk integer counts (Class **R** texel sums).
3. **Compute cull (WebGPU only):** boundary chunks and/or ≥ 1M-slot missions —
   `VERTEX|STORAGE` compaction + `draw_indirect`. WebGL2 keeps chunk granularity. If full
   compute path is too large for one Grok pass, ship CPU cull + ladder first and document
   compute as partial with a clear follow-up — **do not** fake it in TypeScript.
4. **Damage-driven render:** render on camera / doc / residency dirty; fps HUD may keep a
   continuous mode. Record `gpu_frame_ms` band table in verify log.
5. Verify log + tag **T-151.8**.

## Out of scope

- Deck retirement / default flip (**T-151.9**).
- Porting `useSelectTool` / growing TS controllers (D5).
- Forest Path B / T-149 polish.
- Supercluster port (still FE → `set_cluster_markers` OK).
- Registry/docs (Cursor-owned).

## Locked decisions

| # | Decision | Rationale |
|---|---|---|
| L1 | **LANGUAGE GATE:** cull / density / heatmap / damage flags live in Rust | D5 |
| L2 | Reuse `WorldResidency` + `visible_world_rect` + `decode_tbdd` + `INSTANCE_BUDGET` | Already shipped |
| L3 | Budget exceed → **heatmap swap**, not silent drop (today's hard-cap) | Hub W8 |
| L4 | Density texel sums == exact chunk instance counts (Class R) | Hub gate |
| L5 | Draw-chunk set == reference viewport set (Class S) | Hub gate |
| L6 | Thin TS only: call wasm / pass dirty signals; no cull policy in `.ts` | D5 |
| L7 | LOC budgets: no new fat `wgpu*Controller`; any new TS file ≤ **80** LOC | Drift control |
| L8 | W2–W7.3 regressions green; vitest ≥ **393** | Regression |
| L9 | Commit `T-151.8:` · tag **`T-151.8`** · verify log `.ai/artifacts/t151_8_verify_log.md` | House convention |

## Pinned numbers

| Quantity | Value | Source |
|---|---|---|
| `INSTANCE_BUDGET` | **150_000** | `lod_gates.rs` |
| Vitest baseline | **393** | T-151.7.3 |
| Wasm baseline | **4,071,877 B** | T-151.7.3 |
| Everon chunks | **275** | W2 census |

## Tasks

1. Rust: draw-set cull from residency ∩ visible rect (+ margin).
2. Rust: TBDD → density texture + heatmap quad; budget→swap ladder.
3. WebGPU compute cull if feasible; else document partial + CPU path complete.
4. Damage-driven `render` gate + band table in verify log.
5. Thin TS dirty hooks only; tag **T-151.8**.

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
# LANGUAGE GATE: no new fat TS policy files
find src/features/tactical-map/wgpu -name '*.ts' -newer ../../../../.git/refs/tags/T-151.7.3 2>/dev/null | xargs -r wc -l
```

## Manual acceptance

- **S1:** Zoom/pan Everon — draw set tracks viewport (no off-screen glyph thrash).
- **S2:** Zoom out until budget would exceed — heatmap/aggregate rung appears; zoom in → glyphs.
- **S3:** Idle camera — no continuous GPU thrash when damage-driven (HUD continuous OK).
- **S4:** Band table in verify log (`fps` + `gpu_frame_ms` per LOD band).

## Documentation sync (Cursor, after merge)

Registry `T-151.8 → shipped`; hub note; `./scripts/ticket sync`.

## Claude Code prompt — T-151.8 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the WORKTREE at tbd-reforger-wgpu-spike/ (NOT main).

Implement **T-151.8** — culling + density ladder (W8).

═══ PREFLIGHT ═══
  cd /var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike
  test "$(git rev-parse --show-toplevel)" = "$(pwd)"
  git status --porcelain            # empty @ 804f779a+ (tag T-151.7.3)
  # Do NOT checkout branches; do NOT run ./scripts/ticket run
  git lfs pull && make map-assets-link
  make wasm

═══ READ ═══
  1. .ai/artifacts/t151_8_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t151_8_culling_density.md
  3. docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md  (§T-151.8 + D5)
  4. .ai/tickets/CLAUDE_CODE_PROMPT.md  (§T-151 language gate)
  5. crates/map-engine-core/src/world/{residency,lod_gates,chunk_math}.rs
  6. crates/map-engine-core/src/geometry/tbdd.rs
  7. crates/map-engine-core/src/camera/ortho.rs  (visible_world_rect)
  8. crates/map-engine-render/src/engine.rs
  9. apps/.../wgpu/wgpuWorldLoader.ts  (thin only — do not grow policy)

═══ PROBLEM ═══
  Glyphs hard-cap at INSTANCE_BUDGET (drop). Need viewport cull + TBDD density→heatmap ladder
  + damage-driven render. Compute cull WebGPU-only. All policy in Rust (D5).

═══ SHIPPED (do not reopen) ═══
  T-151.7.3 @ 804f779a — SlotGpuBridge in Rust; wgpuSlots.ts 56 LOC; wasm 4,071,877 B.
  LANGUAGE GATE is binding — do not recreate TS controllers.

═══ LANGUAGE GATE (MANDATORY — D5) ═══
  Rust OWNS: draw-set cull, density texture, heatmap swap, compute cull, damage flags,
  INSTANCE_BUDGET ladder policy.
  TypeScript ONLY: React, pointer, thin wasm calls / dirty notify.
  STOP IF: about to put cull/density/ladder policy in .ts → crates/map-engine-* instead.
  LOC budget: any new wgpu TS file ≤ 80; do not grow wgpuSlots.ts past 60.

═══ LOCKED ═══
  - Reuse WorldResidency + visible_world_rect + decode_tbdd + INSTANCE_BUDGET
  - Budget exceed → heatmap swap (not silent drop)
  - Class S draw-set; Class R density texel sums
  - WebGL2: chunk granularity OK if compute is WebGPU-only
  - No W9 Deck delete; no fat TS
  - Commit T-151.8: · tag T-151.8 · .ai/artifacts/t151_8_verify_log.md

═══ DO ═══
  1. Rust CPU draw-set cull (resident ∩ visible rect + margin)
  2. Rust TBDD → density texture + heatmap ladder swap
  3. WebGPU compute cull if feasible; else CPU-complete + document partial
  4. Damage-driven render + band table in verify log
  5. Thin TS hooks only; commit + tag T-151.8

═══ DO NOT ═══
  - Edit docs/registry/CLAUDE
  - Grow wgpu*Controller / put ladder policy in TypeScript
  - Start T-151.9 Deck retirement
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
  S1: draw set tracks viewport
  S2: over-budget → heatmap; zoom in → glyphs
  S3: idle damage-driven (no thrash)
  S4: band table in verify log

═══ RETURN ═══
  - SHA + tag T-151.8
  - .ai/artifacts/t151_8_verify_log.md
  - Ready for Cursor doc sync.
```
