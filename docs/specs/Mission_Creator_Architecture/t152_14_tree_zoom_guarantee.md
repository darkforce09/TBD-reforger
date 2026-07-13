# T-152.14 — Tree glyph zoom-in guarantee (budget + handoff fix)

**Ticket:** T-152 · **Slice:** T-152.14 (remediation ladder #3)
**Status:** `queued`
**Executor:** **claude-code** (Claude Code)
**Authority:** T-152 program hub · audit [`t152_11_fidelity_audit_report.md`](../../../.ai/artifacts/t152_11_fidelity_audit_report.md) §6.1 (S7, A2, A3, D13) · T-151.5/.8 contracts
**Worktree:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152` · branch `ticket/T-152` · tag **`T-152.14`**
**Depends on:** T-152.11 audit (independent of .12/.13 — may run in parallel lane if sequencing allows)

## In one sentence

Zooming into forest must never lose the trees: make the heatmap-swap budget count **viewport-visible** trees instead of whole-chunk sums, add swap hysteresis, and keep forest mass alive until glyphs actually pack — enforced by a property gate over the full zoom range.

---

## Problem

Audit §6.1. At z ≥ 0 the forest fill turns off (`lod_gates.rs:51-53`) on the assumption tree glyphs replace it — but the heatmap predicate `heatmap_trees(exact) = exact > INSTANCE_BUDGET (150_000)` is fed by `exact_tree_count`, which sums the **entire tree+veg rows of every 512 m chunk overlapping the viewport** (`crates/map-engine-core/src/world/density_ladder.rs:20-36,58-60`) — a chunk 1 % on-screen contributes 100 % of its trees. Over budget → `pack_trees=false` → glyph buffer cleared (`residency.rs:858-859,881,913-916`; test `class_r_heatmap_swap_and_full_pack` pins `tree_glyph_count()==0`). Dense Everon forest (~501 k trees, mega-region ~479 k) therefore shows **no mass and no glyphs** — only the faint density wash — across the very zoom band (0…~2) where the operator expects individual trees. This violates the T-151.5 manual contract ("zoom ≥ 0 — individual tree glyphs visible") and T-151.8's "zoom in → glyphs".

The per-instance frustum cull (`compute_cull.rs`, `cs_icon_cull` in `shader.wgsl:271-283`) runs **after** packing and never informs the swap decision.

---

## Goal

1. **Frustum-refined budget count:** estimate viewport-visible trees as Σ over draw chunks of `row_len × clamp(area(chunk ∩ viewport)/area(chunk), 0..1)` (cheap, deterministic) — replaces raw chunk sums in the swap predicate. Exact per-instance counting allowed if it fits the ≤4 ms apply budget; area-fraction is the locked minimum.
2. **Hysteresis:** swap to heatmap at `count > INSTANCE_BUDGET`, swap back at `count < 0.85 × INSTANCE_BUDGET` — no flicker at the boundary.
3. **Mass-until-glyphs invariant:** forest fill stays visible when `class_visible("tree", z)` is true **but** the tree lane is heatmap-swapped or otherwise empty — i.e. fill-off is conditioned on glyphs actually packing, not on zoom alone.
4. **Property gate:** for a pinned dense-forest viewport ladder z ∈ {0, 0.5, 1, …, 6}, tree output is never empty: `tree_glyph_count > 0 ∨ forest_fill_visible ∨ (heatmap_trees ∧ heatmap grid nonzero)` — and specifically glyphs must appear by the zoom where refined count ≤ budget.
5. Verify log `.ai/artifacts/t152_14_verify_log.md`.

---

## Out of scope

- Retuning `INSTANCE_BUDGET`, `TREE_GLYPH_MIN_ZOOM`, glyph art, or density RGBA ramp.
- WebGPU compute-cull changes (`compute_cull.rs` stays as-is).
- Vegetation/prop budget semantics beyond sharing the refined count (props keep hard-stop).
- TS changes (`wgpuWorldLoader.ts` thin calls stay byte-compatible).

---

## Locked decisions

| # | Decision | Rationale |
|---|----------|-----------|
| L1 | Refined count = chunk∩viewport **area-fraction** weighting (minimum); exact instance test optional behind the same API | O(draw_ids) cost, deterministic, testable |
| L2 | Hysteresis band **[0.85×budget, budget]**; state kept in `WorldResidency` | Kill boundary flicker |
| L3 | Fill/outline visibility becomes a **residency decision** (`forest_fill_effective()`) consuming zoom + tree-lane state; `class_visible("forestFill")` stays the pure-zoom oracle | A3 handoff owned where the state lives |
| L4 | Existing test `class_r_heatmap_swap_and_full_pack` updated to the refined semantics — over-*visible*-budget still swaps (heatmap path stays reachable) | Ladder still exists for true mega-viewports |
| L5 | Wasm API additions only (no signature breaks for TS callers) | LANGUAGE GATE |
| L6 | Commit `T-152.14:` · tag `T-152.14` · verify log | House convention |

---

## Pinned numbers

| Quantity | Value | Source |
|----------|-------|--------|
| `INSTANCE_BUDGET` | 150_000 (unchanged) | `lod_gates.rs:26` |
| `TREE_GLYPH_MIN_ZOOM` | 0.0 (unchanged) | `lod_gates.rs:7` |
| Hysteresis re-enter | 0.85 × budget | This slice (L2) |
| Everon tree census | 501,861 | T-090.3.2 |
| Apply budget | ≤ 4 ms/frame | `residency.rs:67` |

---

## Tasks

1. `density_ladder.rs`: `visible_tree_count(chunks, draw_ids, viewport)` with area-fraction weighting + unit tests (corner cases: chunk fully inside/outside/partial).
2. `residency.rs`: swap predicate → refined count + hysteresis state; `forest_fill_effective()`; rewire `refresh_draw_set_and_glyphs`.
3. Update `rebuild_glyph_buffers` pack condition + the Class R tests; add the zoom-ladder property test on a synthetic dense forest fixture (chunks with row counts mirroring the Everon mega-region density).
4. Expose `forest_fill_effective` through existing wasm surface consumed by the forest-mass hook (additive).
5. Verify suite + verify log + commit + tag.

---

## Mathematical acceptance matrix

| Gate | Predicate | Class |
|------|-----------|-------|
| **G1** | `visible_tree_count`: partial-overlap chunk contributes `len × frac` (unit tests incl. 0 and 1 fractions) | Class R |
| **G2** | Hysteresis: count sequence budget+1 → budget−1 keeps heatmap until < 0.85×budget; then glyphs pack | State |
| **G3** | Property ladder z ∈ {0,…,6} on dense fixture: never (no glyphs ∧ no fill ∧ no heatmap); glyphs non-empty once refined count ≤ budget | Property |
| **G4** | `forest_fill_effective` true when z ≥ 0 ∧ heatmap active (mass persists); false when glyphs packed | Handoff |
| **G5** | Existing residency Class R/S suites green after semantic update | Regression |
| **G6** | cargo fmt/clippy (native+wasm)/tests, `make wasm`, FE test/build/lint exit 0; no `apps/**` logic diffs beyond thin additive call if required | LANGUAGE GATE |

---

## Verify

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
cargo fmt --check && cargo clippy --all-targets -- -D warnings
cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings
cargo test -p map-engine-core        # G1–G5
make wasm
cd apps/website/frontend && npm test && npm run build && npm run lint
```

---

## Manual acceptance

- **M1:** Everon mega-forest: zoom −2 → +4 continuously — green mass → (heatmap only if truly over visible budget) → individual trees; **no blank band**.
- **M2:** Pan at z=1 across forest edge — no swap flicker.

---

## Documentation sync (Cursor, after merge)

Registry `T-152.14 → shipped`; hub row; `./scripts/ticket sync`.

---

## Claude Code prompt — T-152.14 (copy-paste)

Authority: this spec. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the T-152 worktree:
  /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152

Implement **T-152.14** — tree glyph zoom-in guarantee (budget + handoff fix).

═══ PREFLIGHT ═══
  cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
  make wasm && cargo test -p map-engine-core 2>&1 | tail -3

═══ READ (in order — spec wins) ═══
  1. docs/specs/Mission_Creator_Architecture/t152_14_tree_zoom_guarantee.md
  2. .ai/artifacts/t152_11_fidelity_audit_report.md §6.1
  3. crates/map-engine-core/src/world/density_ladder.rs
  4. crates/map-engine-core/src/world/residency.rs (:817-999 draw set / glyph pack; :1519 test)
  5. crates/map-engine-core/src/world/lod_gates.rs (:51-53 forestFill)
  6. apps/website/frontend/src/features/tactical-map/wgpu/wgpuWorldLoader.ts (:547-567 — READ ONLY)

═══ PROBLEM ═══
  Heatmap swap counts whole 512 m chunks (chunk 1% on-screen = 100% of its trees) → dense forest
  at z 0..2 clears every glyph while forestFill is zoom-gated off → blank band (operator S7).

═══ SHIPPED (do not reopen) ═══
  T-152.12/.13 text lane. T-151.8 ladder semantics beyond this fix.

═══ LANGUAGE GATE ═══
  Rust OWNS: budget math, hysteresis, fill-handoff policy. TS: at most one thin additive call.
  STOP IF: about to write cull/ladder logic in wgpuWorldLoader.ts.

═══ LOCKED ═══
  - Area-fraction refined count (minimum); INSTANCE_BUDGET unchanged
  - Hysteresis 0.85×budget re-enter
  - forest_fill_effective(): mass persists while tree lane empty/heatmap at z ≥ 0
  - Property gate z∈{0..6}: never blank

═══ DO ═══
  1. visible_tree_count with area-fraction + unit tests
  2. Swap predicate + hysteresis in WorldResidency
  3. forest_fill_effective + wire to mass visibility
  4. Update class_r_heatmap_swap_and_full_pack + add zoom-ladder property test
  5. Verify suite; .ai/artifacts/t152_14_verify_log.md; commit "T-152.14: ..."; tag T-152.14

═══ DO NOT ═══
  - Edit docs/**, .ai/tickets/**
  - Retune INSTANCE_BUDGET / TREE_GLYPH_MIN_ZOOM / density RGBA
  - Break wasm API signatures TS already calls

═══ VERIFY (all exit 0) ═══
  (bash block from spec §Verify)

═══ MANUAL ═══
  M1–M2 per spec

═══ RETURN ═══
  - Commit SHA + tag; verify log path
  - Property-gate ladder table (z → glyphs/fill/heatmap state)
```
