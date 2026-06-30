# T-090.1.2.1 — Claude Code handoff (lossless satellite pyramid)

**Generated:** 2026-06-30 · **Executor:** claude-code · **Commit on:** `main` (`T-090.1.2.1:` prefix)  
**Spec:** [`docs/specs/Mission_Creator_Architecture/t090_1_2_1_lossless_satellite_pyramid.md`](../docs/specs/Mission_Creator_Architecture/t090_1_2_1_lossless_satellite_pyramid.md)  
**Program hub:** [`docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md`](../docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md)  
**Parent:** T-090.1.2 shipped @ `c2730a3` (12800² SAP ortho + decode/stitch — do not redo)

---

## Context (read this first)

**T-090.1.2 is shipped** — real SAP supertexture ortho in staging, z0–5 **lossy q=80** tiles on disk, manifest **`maxZoom: 5`**.

**Operator report:** max zoom Satellite is **still incredibly blurry**. 110% bar = **picture-perfect** ground texture at full zoom. Lossy WebP + z5 stretch at deck zoom 6 defeats the purpose of SAP supertextures.

**T-090.1.2.1** replaces **tile encoding + pyramid depth only** — same ortho, same frontend wiring (unless L1 still fails after lossless z6).

**Not this slice:** Map cartographic tiles = **T-090.1.1** · re-decode SAP = out of scope.

---

## Preflight

```bash
git pull && git lfs pull
make map-assets-link
./scripts/ticket brief T-090

# Ortho must exist (do NOT re-stitch unless this fails):
test -f packages/map-assets/everon/staging/sap/everon-sap-ortho.png
magick identify packages/map-assets/everon/staging/sap/everon-sap-ortho.png   # expect 12800x12800

node scripts/map-assets/verify-sap-ortho.mjs TERRAIN=everon
command -v magick && command -v cwebp
```

**Staging ortho:** `packages/map-assets/everon/staging/sap/everon-sap-ortho.png` (12800×12800, north-up, gitignored — already on operator machine from T-090.1.2).

**Partial z6 junk:** untracked/incomplete `tiles/satellite/6/**` from aborted builds — `build-tile-pyramid.sh` **rm -rf's `--out`** before write; safe.

---

## Execution order (110% — no skip)

### P1 — Script: lossless encode

Edit `scripts/map-assets/build-tile-pyramid.sh`:

- Add `--lossless` flag → `cwebp -lossless` (mutually exclusive with `-q`; error if both).
- Keep default `-q 80` when `--lossless` omitted (T-090.1 compat).

### P2 — Rebuild pyramid (long-running)

```bash
scripts/map-assets/build-tile-pyramid.sh \
  --input  packages/map-assets/everon/staging/sap/everon-sap-ortho.png \
  --out    packages/map-assets/everon/tiles/satellite \
  --minzoom 0 --maxzoom 6 --tilesize 256 --lossless
```

- **No `--flip-v`**
- Expect **~15–25 min** (z6 = 4096 tiles)
- Log: `du -sh packages/map-assets/everon/tiles/satellite`
- Tile count z0–6 = **5461** (1+4+16+64+256+1024+4096)

### P3 — Verify script

Edit `scripts/map-assets/verify-tile-pyramid.mjs`:

1. Every level in `[minZoom, maxZoom]` must be **complete** (exact `(2^z)²` tiles).
2. When `EXPECT_LOSSLESS=1` or `manifest.tiles.satellite.encoding === "webp-lossless"`: assert **VP8L** (fail on VP8 lossy).

Optional: add `satellite.encoding` to `terrain-manifest.schema.json` enum.

### P4 — Manifest + ops log

`packages/map-assets/everon/manifest.json`:

- `tiles.maxZoom`: **6**
- `tiles.satellite.encoding`: **`webp-lossless`**

Update `.ai/artifacts/map_export_everon.json` → satellite method notes lossless z0–6.

Create `.ai/artifacts/t090_1_2_1_verify_log.md` with gate output + `du -sh`.

### P5 — Frontend (only if L1 still soft)

**Default: no change.** `useTerrainBasemapLayer.ts` / `tileUrl.ts` already correct.

If lossless z6 + manifest maxZoom 6 still blurry → debug `computeLod` at deck zoom 6 (expect z=6); do not ship `-q 95` as workaround.

---

## Do not

- Re-open EDDS decode, stitch, orientation (shipped @ c2730a3)
- Ship `maxZoom: 6` without **4096** z6 tiles on disk
- Use `-q 95+` instead of lossless — operator rejected “good enough”
- Edit docs/registry/CLAUDE.md (Cursor sync after ship)

---

## Automated ship gates (all PASS)

```bash
node scripts/map-assets/verify-sap-ortho.mjs TERRAIN=everon
EXPECT_LOSSLESS=1 node scripts/map-assets/verify-tile-pyramid.mjs TERRAIN=everon
make verify-terrain
make ci-local-frontend
node scripts/map-assets/verify-spike-ops-log.mjs TERRAIN=everon
```

---

## Manual acceptance (operator — log in verify file)

| ID | Pass |
|----|------|
| **L1** | Max deck zoom on field/road — **pixel-sharp**, no watercolor blur |
| **L2** | North-up unchanged (airfield north, mountains SE) |
| **L3** | H1/H2 click alignment unchanged |
| **L4** | Pan/zoom ≥55 fps |

---

## Return to operator

1. Commit hash + tag **`T-090.1.2.1`**
2. `du -sh` on `tiles/satellite/` + tile count confirmation
3. Copy-paste **all** automated verify output
4. Path to `.ai/artifacts/t090_1_2_1_verify_log.md`
5. End: **"Ready for Cursor doc sync."**
