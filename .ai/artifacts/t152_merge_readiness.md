# T-152 merge readiness — `ticket/T-152` → `main`

**Date:** 2026-07-13  
**Worktree:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152`  
**Branch:** `ticket/T-152`  
**Gate log:** [`.ai/artifacts/t152_10_verify_log.md`](t152_10_verify_log.md)

---

## Automated gate status

| Gate | Status |
|------|--------|
| G1–G7, G9–G10 | **PASS** (see verify log) |
| G8 operator O1–O12 | **PENDING** — required before `./scripts/ticket done T-152` |

---

## Pre-merge CI (run on worktree tip)

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
git lfs pull
make map-assets-link
cd packages/tbd-schema && npm ci --silent && cd ../..
node scripts/map-assets/verify-t152-cartographic.mjs
make schema-validate
make map-export-validate
make wasm
cargo test -p map-engine-core --all-features
cargo test -p map-engine-render
cd apps/website/frontend && npm ci && npm test && npm run build && npm run lint
./scripts/ticket check
```

Optional full replay: `make db-up && nvm use && make ci-local` (T-125 mirror).

---

## LFS / large assets

| Asset class | Path | Notes |
|-------------|------|-------|
| Everon DEM | `packages/map-assets/everon/dem/everon-dem-16bit.png` | LFS |
| Object chunks | `packages/map-assets/everon/objects/chunks/*.json.gz` | LFS; P5_props census |
| Satellite bundle | `packages/map-assets/everon/satellite/everon-sat.tbd-sat` | LFS (unchanged by T-152) |
| Glyph atlas | `packages/map-assets/glyphs/atlas/world-glyphs.webp` | Rebuilt @ T-152.10 (+ `prop-unknown`) |
| Cartographic tiles | `packages/map-assets/everon/tiles/map/` | gitignored locally; manifest points to workbench-cartographic |

Before merge: `git lfs pull` on operator machine; confirm `make map-assets-link` for dev.

---

## Promotion steps (human)

1. Complete **O1–O12** in `t152_10_verify_log.md` (browser @ Everon Map view).
2. From worktree: re-run aggregator + `make schema-validate` on tip.
3. Merge `ticket/T-152` → `main` (see conflict note below).
4. Tag merge commit **`T-152`** (program) after operator sign-off.
5. Cursor doc pass: `./scripts/ticket done T-152` + `./scripts/ticket sync` on `main`.

**Do not** run `./scripts/ticket done T-152` until **G8 PASS**.

---

## Known limitations (documented — not blockers)

| Item | Note |
|------|------|
| Taxiways | Path B — no taxiway linework (T-152.5 spike); runway + apron + structures only |
| Pier strips | 0 instances meet aspect ≥ 4.0 on Everon OBBs; pier fat-square fills suppressed |
| Arland | Out of program scope |
| `prop-unknown` glyph | Generic 10 px square for unclassified P5 props (444 prefabs) |
| Operator perf (O11) | Manual ≥55 fps check @ default zoom |

---

## Merge conflict watchlist

| File / area | Risk | Mitigation |
|-------------|------|------------|
| `.ai/tickets/registry.json` | **HIGH** — parallel **T-068** arsenal lane on `main` edits same file | Merge `main` into `ticket/T-152` first; resolve registry keeping **both** T-068 active slice + T-152 `ready`/shipped rows |
| `CLAUDE.md` | **MEDIUM** — `<!-- ticket-sync:status -->` block | Accept `main` then re-run `./scripts/ticket sync` on merged tip |
| `Cargo.lock` | **LOW** | Regenerate if conflict: `cargo build` |
| `apps/website/frontend/package-lock.json` | **LOW** | Prefer worktree if T-152-only FE changes |

**No expected overlap** with T-068 application paths (`apps/website/internal/`, mission compiler) — T-152 touches map-engine crates, `packages/map-assets/`, wgpu frontend map lane.

---

## Post-merge doc sync (Cursor)

- Registry: T-152 program → `shipped`; all child slices shipped
- Hub [`t152_map_cartographic_fidelity_program.md`](../docs/specs/Mission_Creator_Architecture/t152_map_cartographic_fidelity_program.md) status → shipped
- `CLAUDE.md` §Status T-152 bullet via `./scripts/ticket sync`
- Link verify log + this doc from program hub §Related
