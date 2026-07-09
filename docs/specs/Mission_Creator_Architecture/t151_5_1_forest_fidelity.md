# T-151.5.1 — forest mass / landcover fidelity (W5 corrective)

**Status:** **ready** (executor queue) · **Program:**
[`t151_wgpu_engine_program.md`](t151_wgpu_engine_program.md) · **Executor:** claude-code ·
**Worktree:** `tbd-reforger-wgpu-spike/` (absolute:
`/var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike`; do **not** touch `main`) ·
**Baseline:** `0b7621ed` (tag **T-151.5** — verify log
[`t151_5_verify_log.md`](../../../.ai/artifacts/t151_5_verify_log.md)).

## In one sentence

Tighten forest **fill / outline / landcover** so the green envelope tracks tree glyphs (and thin
stands) instead of bloating 32 m cells and bridging gaps — runtime + LOD first; no Path B export
rebuild in this slice.

## Problem (operator @ T-151.5)

With tree glyphs on, the green highlight is clearly wrong:

1. **Overshoot** — mass/landcover extends far past glyph clusters into fields/roads.
2. **Gap fill** — small clearings between stands become one continuous green blob.
3. **Thin lines** — hedgerows / bush lines get huge blocky green envelopes.
4. **“Grid” in green** — zoomed-in checkerboard / cell seams inside the fill (32 m TBDD
   marching-squares tessellation — **not** the map grid lane).

This is **Deck-parity** geometry (`DENSITY_ISO=1` vs region export threshold **2**; Path B
mega-hulls; per-cell rings). Glyphs made it diagnosable; do not blame the atlas.

## Goal

1. **`DENSITY_ISO` 1 → 2** in TS + Rust (align mass with Path B region floor; drop sparse bleed).
2. **LOD when glyphs visible:** at `deckZoom ≥ TREE_GLYPH_MIN_ZOOM` (0), hide **forest fill** and
   **forest outline** (glyphs carry detail); keep readable mass at coarse zoom (&lt; 0).
3. **Landcover:** hide or strongly soften when glyphs visible; **re-push visibility on zoom** in
   wgpu (`landcoverPushed` one-shot bug — refresh like forest mass).
4. **Deck parity:** same iso + LOD changes in `forestMass.ts` / `lodGates.ts` / `forestMassLayer.ts`
   so `?engine=` off matches.
5. **Parity tests:** forest.parity + LOD scan stay green; update any iso=1 golden expectations.
6. **Document** residual 32 m cell look + mega-region as **T-149 / export follow-up** (finer grid /
   Chaikin / split `forest-everon-001`) — out of scope here.

## Out of scope

- Rebuild `forest-regions.json.gz` / Path B / TBDD density bins (export program).
- T-149 finer grid + contour smoothing (idea — link from verify log).
- Glyph atlas / residency / buildings / roads changes.
- T-151.6 mission entities.
- Registry/docs edits (Cursor-owned).

## Locked decisions

| # | Decision | Rationale |
|---|---|---|
| L1 | `DENSITY_ISO = 2` in `forestMass.ts` + `forest_mass.rs` (+ any wgpu hardcode) | Match region export threshold; reduce gap-bridging |
| L2 | When `classVisible('tree', zoom)` **or** `zoom ≥ TREE_GLYPH_MIN_ZOOM`: **no** forest fill upload/draw | Glyphs are the detail layer |
| L3 | Same gate for **forest outline** (no leftover cell-edge “grid” when zoomed in) | Operator symptom 4 |
| L4 | Landcover: not drawn when L2 gate active; wgpu re-evaluates landcover visibility on camera/zoom (fix sticky `landcoverPushed`) | Mega-hull underdraw + wgpu LOD bug |
| L5 | Coarse zoom (&lt; 0): mass + outline still on (iso=2) for island readability without glyphs | Default editor zoom −2 |
| L6 | Deck + wgpu both updated — Class R / visual parity | Dual-mount oracle |
| L7 | No export rebuild; verify log lists T-149 / Path B split as follow-ups | Scope control |
| L8 | W2–W5 regressions green; vitest ≥ **372** | Regression |
| L9 | Commit `T-151.5.1:` · tag **`T-151.5.1`** · verify log `.ai/artifacts/t151_5_1_verify_log.md` | House convention |

## Pinned numbers

| Quantity | Before | After |
|---|---|---|
| `DENSITY_ISO` | **1** | **2** |
| Forest fill @ zoom ≥ 0 | on | **off** |
| Forest outline @ zoom ≥ 0 | on | **off** |
| Landcover @ zoom ≥ 0 | on (sticky) | **off** (refreshed) |
| Vitest baseline | **372** | ≥ 372 |

## Tasks

1. Raise iso TS+Rust; update forest.parity / unit tests.
2. LOD gates / layer builders: suppress fill+outline (+ landcover) when glyphs band active.
3. wgpu: forest mass + landcover visibility refresh on zoom.
4. Deck: same gates in `useWorldMapLayers` / `forestMassLayer` / landcover.
5. Manual A/B screenshots; verify log; tag **T-151.5.1**.

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

- **S1:** zoom ≥ 0 — tree glyphs visible; **no** bloated green fill/outline/grid over fields; thin
  stands not wrapped in huge blocks.
- **S2:** zoom −2 — forest context still readable (iso=2 mass); no glyphs (LOD).
- **S3:** Deck (`?engine=` off) matches wgpu forest behavior.
- **S4:** Note residual mega-region / 32 m cell limits → T-149 / export follow-up.

## Documentation sync (Cursor, after merge)

Registry `T-151.5.1 → shipped`; hub note; `./scripts/ticket sync`.

## Claude Code prompt — T-151.5.1 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the WORKTREE at tbd-reforger-wgpu-spike/ (NOT main).

Implement **T-151.5.1** — forest mass / landcover fidelity (tighten green envelope vs tree glyphs).

═══ PREFLIGHT ═══
  cd /var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike
  test "$(git rev-parse --show-toplevel)" = "$(pwd)"
  git status --porcelain            # empty @ 0b7621ed+ (tag T-151.5)
  # Do NOT checkout branches; do NOT run ./scripts/ticket run
  git lfs pull && make map-assets-link
  make wasm

═══ READ ═══
  1. .ai/artifacts/t151_5_1_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t151_5_1_forest_fidelity.md
  3. apps/website/frontend/src/features/tactical-map/worldmap/{forestMass,forestMassLayer,lodGates,landCoverRegions,useWorldMapLayers}.ts
  4. crates/map-engine-core/src/geometry/forest_mass.rs
  5. crates/map-engine-core/src/world/lod_gates.rs
  6. apps/website/frontend/src/features/tactical-map/wgpu/{useWgpuForestMass,wgpuWorldLoader}.ts
  7. .ai/artifacts/t151_4_1_verify_log.md  (P2 forest note)
  8. features/_wasm/forest.parity.test.ts

═══ PROBLEM ═══
  Operator with T-151.5 glyphs: green forest fill/outline/landcover overshoots tree clusters,
  bridges gaps, wraps thin stands in huge blocks, and shows 32 m cell "grid" seams when zoomed in.
  Root: DENSITY_ISO=1 (regions use 2), Path B mega-hulls, per-cell marching squares, landcover
  sticky LOD on wgpu. Tighten runtime + LOD; do NOT rebuild export assets this slice.

═══ SHIPPED (do not reopen) ═══
  T-151.5 @ 0b7621ed — glyph atlas; vitest 372; wasm 4,054,850 B.
  T-151.4.1 @ 552e68aa — buildings + road joins.

═══ LOCKED ═══
  - DENSITY_ISO 1 → 2 (TS + Rust)
  - zoom ≥ 0 (tree glyph band): hide forest fill + forest outline + landcover
  - zoom < 0: mass/outline still on (iso=2) for coarse context
  - Fix wgpu landcover sticky visibility (re-push on zoom)
  - Deck + wgpu parity
  - No Path B / TBDD / forest-regions rebuild; document T-149 follow-up
  - Commit T-151.5.1: · tag T-151.5.1 · .ai/artifacts/t151_5_1_verify_log.md

═══ DO ═══
  1. Raise DENSITY_ISO to 2 everywhere; update parity/unit tests
  2. LOD: suppress forestFill + forestOutline (+ landcover) when tree glyph band active
  3. wgpuWorldLoader: refresh landcover visibility on camera/zoom (not one-shot forever)
  4. useWgpuForestMass: honor new gates (no fill/outline upload when suppressed)
  5. Deck layers same behavior
  6. Verify log with S1–S4 notes; commit + tag T-151.5.1

═══ DO NOT ═══
  - Edit docs/registry/CLAUDE
  - Rebuild forest-regions.json.gz or density bins
  - Change glyph atlas / building / road code except if a shared LOD helper requires it
  - Start T-151.6 slots
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
  S1: zoom ≥ 0 — glyphs on; no bloated green fill/outline/grid over fields; thin stands tight
  S2: zoom −2 — readable forest context (iso=2); no glyphs
  S3: Deck matches
  S4: residual mega-region / 32 m limits noted → T-149

═══ RETURN ═══
  - SHA + tag T-151.5.1
  - .ai/artifacts/t151_5_1_verify_log.md
  - Ready for Cursor doc sync.
```
