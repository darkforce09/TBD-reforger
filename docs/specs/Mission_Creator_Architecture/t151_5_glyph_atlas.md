# T-151.5 — glyph atlas + LOD gates: trees, props, badges (W5)

**Status:** **shipped** @ `0b7621ed` (tag **T-151.5**, 2026-07-09) · verify log
[`t151_5_verify_log.md`](../../../.ai/artifacts/t151_5_verify_log.md) · **Program:**
[`t151_wgpu_engine_program.md`](t151_wgpu_engine_program.md) (W5) · **Executor:** claude-code ·
**Worktree:** `tbd-reforger-wgpu-spike/` (absolute:
`/var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike`; do **not** touch `main`) ·
**Baseline:** `552e68aa` (tag **T-151.4.1** — verify log
[`t151_4_1_verify_log.md`](../../../.ai/artifacts/t151_4_1_verify_log.md)).

**Shipped notes:** Atlas once + IconInstanced **20 B**; vitest **372** (+1 LOD parity); wasm
**4,054,850 B** (+45,482). Forest mass / Path B **not** retuned (operator analysis next).

## In one sentence

Upload the world glyph atlas once and draw **individual tree / vegetation / prop glyphs** (plus
building badges) on `WgpuTacticalMap` with Class **R** size/LOD parity vs Deck — so operators can
judge forest mass overdraw against real tree instances.

## Problem

W4 draws forest **mass polygons** + land-cover hulls but **no tree icons**. Operator feedback:
forest fill is overkill (towns + inter-patch gaps). Tuning mass/export without glyphs is guessing.
Deck already streams budget-capped glyphs via `treeStore` + `IconLayer` (`treePropLayer.ts`) from
`world-glyphs.webp` (28 keys). **W5** ports that lane to wgpu.

## Goal

1. **Atlas upload once:** `/map-assets/glyphs/atlas/world-glyphs.webp` + `.json` → GPU texture +
   28-entry UV uniform table (WebGL2-safe).
2. **Icon instance layout ≤ 20 B** (program hub): pos 2×f32, size f32, yaw snorm16, glyph u16,
   tint u32 — document exact layout in verify log.
3. **Tree / veg / prop streams:** viewport-driven instances from residency (or thin JS fetch
   mirroring `treeStore` → wasm compose) with `INSTANCE_BUDGET` / worker-visible set parity.
4. **Size math Class R:** `baseSizePx × treeSizeMultiplier(heightM) / 2^REF_ZOOM` (`REF_ZOOM=3`),
   `GLYPH_SIZE_MIN_PX=4`; badges 10 px min 8.
5. **LOD:** `classVisible` for `tree` / `vegetation` / `prop` / `rockLarge` / `buildingBadge`
   verbatim from `lodGates.ts` — exhaustive Rust↔TS scan.
6. **Draw order:** after forest outline, before grid (Deck slots 9–10): trees → props → badges.
7. **Prefs:** honor `worldLayerPrefs` toggles (trees/props) like Deck.
8. **GPU-R:** tree glyph readback — nonzero α at projected center + tint class match.

## Out of scope

- Forest mass / land-cover export retune (follow-up after operator glyph analysis).
- Slot ring / cluster discs / mission entity icons (W6 — may share atlas; **do not** wire slots).
- Editor pick on glyphs (W7).
- Retiring Deck `treeStore` / `IconLayer` (T-151.9).
- Registry/docs edits (Cursor-owned).

## Locked decisions

| # | Decision | Rationale |
|---|---|---|
| L1 | New `PipelineKind::IconInstanced` (or textured-quad instanced) sampling atlas; one atlas bind group | Program hub W5 |
| L2 | Instance layout ≤ **20 B**; UV via 28-entry uniform table asserted at init | Spike §20M budget |
| L3 | Atlas paths: `world-glyphs.webp` + `world-glyphs.json` via `loadWorldGlyphAtlas` contract | Deck oracle |
| L4 | Size/angle/color pure math ports `treePropLayer.ts` (`treeSizeMultiplier`, `glyphSizeMeters`, `deckAngleForRotationDeg`, `hexToRgba`) — Class **R** vitest or native | Parity |
| L5 | LOD: `TREE_GLYPH_MIN_ZOOM=0`, `VEGETATION_MIN_ZOOM=1.5`, `PROP_MIN_ZOOM=3`, `ROCK_LARGE_MIN_ZOOM=1`, `BUILDING_BADGE_MIN_ZOOM=1` | `lodGates.ts` / N2 |
| L6 | Exhaustive LOD scan: Rust `class_visible` == TS for zoom ∈ {−6.0 … +6.0} step 0.1 × all glyph classes | Program hub gate |
| L7 | Stream policy mirrors `treeStore`: replace-not-accumulate visible set; clear below tree band; budget cap | Deck oracle |
| L8 | Draw order: … forest-outline → **trees** → **props** → **badges** → grid | `useWorldMapLayers` slots 9–10 |
| L9 | Building badges from same atlas (3 keys) when `classVisible('buildingBadge')` | Hub W5 |
| L10 | `stats()` additive: `tree_glyphs`, `prop_glyphs`, `badge_glyphs`, `atlas_bytes` — prior keys untouched | Regression |
| L11 | GPU-R tree glyph probe at pinned camera | Class **GPU-R** |
| L12 | W2/W3/W4 + T-151.4.1 regressions green; vitest ≥ **371** | Regression |
| L13 | Commit `T-151.5:` · tag **`T-151.5`** · verify log `.ai/artifacts/t151_5_verify_log.md` | House convention |

## Pinned numbers

| Quantity | Value | Source |
|---|---|---|
| Atlas glyphs | **28** | `world-glyphs.json` |
| Tree types (census) | **51** | W2 / T-090.3.2 |
| Tree instances (Everon) | **~501,861** | census (budget-capped on screen) |
| `REF_ZOOM` | **3** | `lodGates.ts` |
| `TREE_GLYPH_MIN_ZOOM` | **0** | below → forest mass only |
| `INSTANCE_BUDGET` | **150_000** | `lodGates.ts` |
| Vitest baseline | **371** | T-151.4.1 |
| Wasm baseline | **4,009,368 B** | T-151.4.1 verify log |

## Tasks

1. Atlas load + GPU upload + UV table.
2. Icon instance pipeline + ≤20 B layout.
3. Size/LOD pure ports + exhaustive LOD scan.
4. Tree/prop stream hook (`useWgpuTreeGlyphs` or residency extend) + badges.
5. Wire `WgpuTacticalMap` draw order + prefs.
6. GPU-R + verify log; tag **T-151.5**.

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

- **S1:** `?engine=wgpu` zoom **≥ 0** — individual tree glyphs visible over forest mass; pan/zoom stable.
- **S2:** zoom **&lt; 0** — tree glyphs hidden; forest mass remains (LOD).
- **S3:** Deck path unchanged; glyph density/feel comparable at same camera (advisory).
- **S4:** GPU-R tree probe JSON pasted (nonzero α).

## Operator note (forest analysis)

After S1, operator will use tree icons to judge land-cover / forest-mass overdraw. **Do not** retune
`DENSITY_ISO` or Path B hulls in this slice — that is a follow-up.

## Documentation sync (Cursor, after merge)

Registry `T-151.5 → shipped`; program hub W5; verify-log link; `./scripts/ticket sync`.

## Claude Code prompt — T-151.5 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the WORKTREE at tbd-reforger-wgpu-spike/ (NOT main).

Implement **T-151.5** — glyph atlas + LOD gates: trees, props, badges (W5).

═══ PREFLIGHT ═══
  cd /var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike
  test "$(git rev-parse --show-toplevel)" = "$(pwd)"
  git status --porcelain            # empty @ 552e68aa+ (tag T-151.4.1)
  # Do NOT checkout or create branches; do NOT run ./scripts/ticket run
  git lfs pull && make map-assets-link
  cd apps/website/frontend && npm ci && cd ../../..
  make wasm

═══ READ (in order — spec wins on conflict) ═══
  1. .ai/artifacts/t151_5_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t151_5_glyph_atlas.md
  3. docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md   (W5 gates)
  4. docs/specs/Mission_Creator_Architecture/t090_render_lod_contract.md  (N2 glyph LOD)
  5. apps/website/frontend/src/features/tactical-map/layers/worldGlyphAtlas.ts
  6. apps/website/frontend/src/features/tactical-map/worldmap/{treePropLayer,treeStore,lodGates,useWorldMapLayers,buildingLayer}.ts
  7. apps/website/frontend/src/features/tactical-map/wgpu/{wgpuWorldLoader,useWgpuForestMass,WgpuTacticalMap}.ts*
  8. crates/map-engine-render/src/{engine.rs,lanes.rs,shader.wgsl,scene.rs}
  9. crates/map-engine-core/src/world/residency.rs
  10. public/map-assets/glyphs/atlas/world-glyphs.{webp,json}

═══ PROBLEM ═══
  W4 draws forest mass polygons but no individual tree/prop glyphs. Operator needs tree icons on
  ?engine=wgpu (zoom ≥ 0) to analyze forest overdraw. Deck already has IconLayer glyphs from the
  28-key world atlas via treeStore — port that lane to wgpu with Class R size/LOD parity.

═══ SHIPPED (do not reopen) ═══
  T-151.4.1 @ 552e68aa — building wipe race + road miter/caps; wasm 4,009,368 B; vitest 371.
  T-151.4 @ 723490a0 — vector stack (sea/landcover/contours/roads/forest mass/buildings).
  T-151.3 @ 32bf5ac5 — residency + building GPU.

═══ LOCKED (full table: spec §Locked decisions L1–L13) ═══
  - Atlas once; ≤20 B icon instances; 28 UV uniform table
  - Size/angle/color Class R vs treePropLayer.ts
  - LOD gates TREE≥0 / veg≥1.5 / prop≥3 / rockLarge≥1 / badge≥1
  - Exhaustive Rust↔TS class_visible scan (−6…+6 @ 0.1)
  - Draw order: forest-outline → trees → props → badges → grid
  - treeStore replace-not-accumulate + INSTANCE_BUDGET policy
  - GPU-R tree glyph probe; stats additive only
  - Do NOT retune forest mass / landcover export this slice
  - Deck path untouched; no slot ring / clusters (W6)

═══ DO ═══
  1. Load world-glyphs atlas → GPU texture + UV table (L1–L3)
  2. IconInstanced pipeline + ≤20 B layout (L2)
  3. Port size/LOD pure math + exhaustive LOD scan tests (L4–L6)
  4. Stream tree/veg/prop (+ badges) onto WgpuTacticalMap; prefs toggles (L7–L9)
  5. GPU-R + stats keys (L10–L11); regressions (L12)
  6. Write .ai/artifacts/t151_5_verify_log.md; commit T-151.5: · tag T-151.5

═══ DO NOT ═══
  - Edit docs/**, .ai/tickets/registry.json, docs/TICKET_*.md, CLAUDE.md status markers
  - Rewrite forest-regions / DENSITY_ISO / Path B hulls
  - Touch main, delete Deck treeStore/IconLayer, implement slots/clusters (W6)
  - Break world.parity/residency/pick, dem/forest parity, T-151.0–4.1 self-checks
  - git checkout -b / ./scripts/ticket run

═══ VERIFY (all exit 0) ═══
  cargo fmt --check
  cargo clippy --all-targets -- -D warnings
  cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings
  cargo test -p map-engine-core --all-features
  cargo test -p map-engine-render
  cargo build --workspace
  make wasm
  cd apps/website/frontend && npm test && npm run build && npm run lint
  ! grep -l map_engine_wasm_bg dist/assets/index-*.js

═══ MANUAL ═══
  S1: ?engine=wgpu zoom ≥ 0 — tree glyphs visible over forest mass
  S2: zoom < 0 — glyphs hidden; forest mass remains
  S3: Deck unchanged; advisory density compare
  S4: tree glyph GPU-R JSON

═══ RETURN ═══
  - Commit SHA + tag T-151.5
  - .ai/artifacts/t151_5_verify_log.md (gates + LOD scan + readback + wasm size)
  - Automated verify output (PASS)
  - Manual notes S1–S4
  - **Ready for Cursor doc sync.**
```
