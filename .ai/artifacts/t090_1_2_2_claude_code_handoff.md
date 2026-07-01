# T-090.1.2.2 — Claude Code handoff (SAP cell seam repair)

**Slice:** T-090.1.2.2 · **Executor:** claude-code · **Branch:** `ticket/T-090`  
**Parent shipped:** T-090.1.2.1 @ `19bc785` (lossless VP8L z0–6, 299M LFS)  
**Spec (ONLY source of truth):** [`docs/specs/Mission_Creator_Architecture/t090_1_2_2_sap_cell_seam_repair.md`](../docs/specs/Mission_Creator_Architecture/t090_1_2_2_sap_cell_seam_repair.md)

---

## Operator report

Visible **256 m grid seams** at max zoom — vertical lines + visible cross where four SAP cells meet. Documented in operator screenshot during T-090.1.2.1 acceptance.

**Not** WebP pyramid tile seams. **Not** lossless compression. The lossless pyramid **faithfully preserved** the stitch artifact from `stitch-sap-ortho.mjs`.

---

## What you are fixing

```
2500 Eden supertexture cells (256×256 px each)
  → BC7 decode (decode-edds.mjs)
  → hard paste edge-to-edge into 12800×12800 RGBA canvas
  → everon-sap-ortho.png
  → lossless pyramid (already shipped — you rebuild after ortho fix)
```

Seams at every **256 px** grid line in the ortho = SAP cell boundaries.

---

## Do not

| Forbidden | Why |
|-----------|-----|
| z7+ pyramid / fake upscaling | Operator rejected |
| AI upscale / inpaint | Operator rejected |
| Re-decode format / BC7 “fix” | BC7 is inherently block-lossy; fix compositing |
| Grey fill / whole-map blur | Spec forbidden |
| Edit docs / registry / TICKET_*.md / CLAUDE.md | Cursor sync after merge |
| Frontend changes | Unless S4 pan perf regresses (unlikely) |

**Do not reopen:** T-090.1.2 decode contract, T-090.1.2.1 lossless encode (`--lossless --maxzoom 6`).

---

## Execution order (strict)

```text
P0  analyze-sap-seams.mjs on CURRENT ortho → t090_1_2_2_seam_analysis.json
    READ diagnosis — STOP if placement bug; fix placement before feather

P1  Implement fix in stitch-sap-ortho.mjs (strategy A default: 2–8 px edge feather)
    Preserve north-up row-flip assembly (see spec §Technical context)

P2  Re-stitch → post-fix analyze → verify-sap-seams.mjs → verify-sap-ortho.mjs

P3  Rebuild lossless pyramid (same command as T-090.1.2.1)

P4  EXPECT_LOSSLESS=1 verify-tile-pyramid + make verify-terrain + make ci-local-frontend

P5  Write t090_1_2_2_verify_log.md (S1–S4 + before/after metrics)

P6  Commit LFS tiles + scripts; tag T-090.1.2.2; return "Ready for Cursor doc sync"
```

---

## Preflight (run first)

```bash
git pull && git lfs pull && make map-assets-link
./scripts/ticket brief T-090

export ENFUSION_GAME_PATH="${ENFUSION_GAME_PATH:-$HOME/.cache/enfusion-mcp-root}"
command -v magick && command -v cwebp

# Baseline ortho (gitignored — may need re-stitch from pak if missing):
test -f packages/map-assets/everon/staging/sap/everon-sap-ortho.png \
  && magick identify packages/map-assets/everon/staging/sap/everon-sap-ortho.png

# If missing:
#   node scripts/map-assets/catalog-sap-cells.mjs TERRAIN=everon
#   node scripts/map-assets/stitch-sap-ortho.mjs TERRAIN=everon
```

---

## Key files (read before edit)

| File | Role |
|------|------|
| `scripts/map-assets/stitch-sap-ortho.mjs` | **Primary fix target** — hard paste loop lines 52–77 |
| `scripts/map-assets/decode-edds.mjs` | Constants: GRID=50, CELL_PX=256, CELL_M=256 — **read-only** |
| `scripts/map-assets/verify-sap-ortho.mjs` | Must still PASS (orientation AE < 0.2) |
| `scripts/map-assets/build-tile-pyramid.sh` | `--lossless --maxzoom 6` rebuild |
| `scripts/map-assets/verify-tile-pyramid.mjs` | `EXPECT_LOSSLESS=1` |
| `.ai/artifacts/t090_1_2_1_verify_log.md` | Reference ship gates from prior slice |

---

## Pyramid rebuild (copy-paste)

```bash
scripts/map-assets/build-tile-pyramid.sh \
  --input packages/map-assets/everon/staging/sap/everon-sap-ortho.png \
  --out packages/map-assets/everon/tiles/satellite \
  --minzoom 0 --maxzoom 6 --tilesize 256 --lossless

EXPECT_LOSSLESS=1 node scripts/map-assets/verify-tile-pyramid.mjs TERRAIN=everon
```

Expect ~5461 tiles, ~299M total LFS. `git add packages/map-assets/everon/tiles/satellite`.

---

## Manual acceptance (operator)

| ID | What |
|----|------|
| **S1** | Former seam location invisible @ max zoom |
| **S2** | 3 other intersections (road, forest, coast) |
| **S3** | Alignment unchanged |
| **S4** | Pan fps ≥55 (should be unchanged — no FE work) |

Dev: `make api` + `make web` → dev-login mission_maker → Mission Creator → Satellite @ max zoom.

---

## Return to operator / Cursor

1. Commit SHA + tag `T-090.1.2.2`
2. `seam_analysis.json` summary (worst edge before/after)
3. Full automated verify output (all gates PASS)
4. Strategy used (A/B/C/D) + feather width if A
5. S1 location note (world coords or map landmark)
6. **`Ready for Cursor doc sync.`**
