# T-152.21 — Claude Code handoff (landmark early visibility)

**Slice:** T-152.21 · **Branch:** `ticket/T-152` · **Worktree:** `.ai/artifacts/worktrees/TBD-T-152`
**Spec:** [`t152_21_landmark_early_visibility.md`](../docs/specs/Mission_Creator_Architecture/t152_21_landmark_early_visibility.md)
**Executor:** claude-code · **Tag:** `T-152.21`

## Context

`importanceZoom` exists in schema, classify rules (−4 landmarks), LOD contract, and `PrefabInfo` parse — **never read at render**. Default editor zoom −2 → landmarks are gray OBBs only (original P1).

T-152.18 deferred — current redraw atlas is fine for this slice.

## Scope (Rust primary)

Wire badge/glyph path: `deck_zoom ≥ importance_zoom` overrides `BUILDING_BADGE_MIN_ZOOM` (1.0). Data-driven from prefab rules, not hardcoded class list.

## Touch files (expected)

- `crates/map-engine-core/src/world/residency.rs` — badge emission gate
- `crates/map-engine-core/src/world/glyph_math.rs` — `BADGE_SIZE_MIN_PX` floor at coarse zoom
- `lod_gates.rs` / fill de-emphasis as spec
- `apps/website/frontend/.../lodGates.ts` — wire or delete dead `landmarkVisible()`
- Tests: lighthouse @ z=−2, class R census

## Deferred

- T-152.18 icon extract (redraw art OK)
