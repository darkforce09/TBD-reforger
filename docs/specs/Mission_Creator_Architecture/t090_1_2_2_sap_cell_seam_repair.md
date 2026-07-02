# T-090.1.2.2 — SAP supertexture cell seam repair

**Ticket:** T-090 · **Slice:** T-090.1.2.2  
**Status:** **SHIPPED** @ `a3efdf6` — apron-bridge + automated gates; **110% bar deferred** to **T-090.1.2.4** (engine ortho) + **T-090.1.2.8** (unified delivery)  
**Git tag on ship:** **T-090.1.2.2**  
**Executor:** claude-code  
**Depends on:** **T-090.1.2.1** shipped @ `19bc785` (lossless z0–6 pyramid)  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md) · [`t090_1_2_sap_supertexture_satellite.md`](t090_1_2_sap_supertexture_satellite.md)

**Agent roles (locked):** **Cursor** authors/syncs documentation. **Claude Code reads this spec and implements code only** — return verify output to Cursor; do **not** edit docs/registry.

---

## In one sentence

Eliminate visible **vertical/horizontal gaps** where the 50×50 SAP supertexture cells meet in the stitched ortho — then rebuild the lossless pyramid — without re-inventing the decode pipeline or faking detail.

---

## Problem (operator-reported @ T-090.1.2.1)

At max zoom, Satellite shows **clear stitch lines** on a ~**256 m** grid (matches SAP cell size). Example: vertical seam + cross distortion where four cells meet (operator screenshot). This is **not** WebP pyramid tile seams (those are 200 m @ z6) — it is the **ortho composite** from `stitch-sap-ortho.mjs`:

- 2500 cells pasted **hard edge-to-edge** with no overlap blend
- Adjacent BC7-decoded cells can differ in exposure/tint at borders (BC7 is 4×4 block lossy)
- Any sub-pixel placement error amplifies as a visible line

Lossless VP8L (T-090.1.2.1) preserved the seam faithfully — compression is not the cause.

### Seams vs pyramid tiles (do not confuse)

| Artifact | Grid spacing | Layer |
|----------|--------------|-------|
| **This slice** | **256 m** (= `CELL_PX` = `CELL_M`) | Staged ortho `everon-sap-ortho.png` before pyramid |
| Pyramid tile boundaries | ~200 m effective @ z6 (256 px tiles in 12800 m world) | `tiles/satellite/{z}/{x}/{y}.webp` — **not the root cause** |

Fix the **ortho** first; rebuilding the pyramid propagates the repair.

---

## Goal

1. **Measure** seam severity on the 256 px grid (automated gate — P0 before blind blending).
2. **Fix** stitch compositing so cell boundaries are **invisible at max zoom** on representative terrain (fields, roads, forest).
3. **Rebuild** lossless z0–6 pyramid from repaired ortho (same command as T-090.1.2.1).
4. **Verify** orientation + alignment gates still PASS (no regression on T-090.1.2 north-up fix).
5. **Manual S1–S4** — operator sign-off in verify log.

---

## Out of scope

| Item | Ticket / reason |
|------|-----------------|
| **z7+ pyramid** | Interpolates 1 m/px source — fake detail |
| **AI upscaling / inpainting** | Operator rejected |
| **BC7 “decompression”** | Cannot recover discarded high frequencies from `.edds` |
| **Pan flicker / tile pop-in** | **T-090.1.2.3** (frontend tile cache) |
| **Water readability** | **T-090.1.2.5** (ortho composite — after this slice) |
| **Brightness / global tone** | Separate later pass |
| **Frontend LOD / `useTerrainBasemapLayer.ts`** | Touch only if S4 fails |

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| **Root fix location** | `stitch-sap-ortho.mjs` (or dedicated post-pass called from it) — **not** pyramid encode |
| **Decode contract** | Reuse `decode-edds.mjs` + `vendor/bc7.mjs` unchanged unless P0 proves decode bug |
| **P0 gate** | Run `analyze-sap-seams.mjs` **before** blending; STOP if placement off-by-one |
| **Default fix strategy** | **A. Edge feather** 2–8 px on interior cell edges (skip map outer border) |
| **Fallback strategies** | B per-cell gain on overlap band · C crop redundant border texels · D placement fix |
| **Forbidden fixes** | Grey fill, whole-map blur, AI inpainting, z7 pyramid |
| **Pyramid rebuild** | `build-tile-pyramid.sh --lossless --maxzoom 6` — identical to T-090.1.2.1 |
| **Orientation** | Must keep `verify-sap-ortho.mjs` orientation guard PASS (AE ratio < 0.2) |
| **LFS ship** | Replace full `packages/map-assets/everon/tiles/satellite/**` (~299M) |
| **Meta bump** | Update `TBD_SatExport_meta.json` with `seamRepair: "T-090.1.2.2"` + strategy id |

---

## Technical context (read before editing stitch)

### Constants (`decode-edds.mjs`)

```text
GRID = 50          → 50×50 = 2500 cells
CELL_PX = 256      → 256 px per cell mip0
CELL_M = 256       → 256 m world per cell
ORTHO_PX = 12800   → 12800² full ortho, 1 m/px
```

Cell index: `N = gridY * 50 + gridX` (row-major; `gridY=0` = north).

### Current stitch (`stitch-sap-ortho.mjs`)

1. Decode each cell RGBA via BC7 (`decodeCellRgba(n)`).
2. Paste at `(px = gridX * 256, pyTop = (49 - gridY) * 256)` with **per-row vertical flip inside cell** (north-up assembly — do **not** break this when blending).
3. **Hard memcpy** — no overlap, no feather, no exposure match.

Seam lines appear at every `x ∈ {256, 512, …, 12544}` and `y ∈ {256, 512, …, 12544}` in image space.

### Staging paths (gitignored except committed tiles)

```text
packages/map-assets/everon/staging/sap/everon-sap-ortho.png   ← fix target
packages/map-assets/everon/staging/sap/TBD_SatExport_meta.json
packages/map-assets/everon/staging/sap/cell-catalog.json
packages/map-assets/everon/tiles/satellite/**                 ← LFS rebuild output
```

### Environment preflight

```bash
git pull && git lfs pull && make map-assets-link
./scripts/ticket brief T-090

# Re-stitch requires pak decode:
export ENFUSION_GAME_PATH="${ENFUSION_GAME_PATH:-$HOME/.cache/enfusion-mcp-root}"
test -d "$ENFUSION_GAME_PATH" || echo "WARN: set ENFUSION_GAME_PATH to Reforger game root"

command -v magick && command -v cwebp
test -f packages/map-assets/everon/staging/sap/everon-sap-ortho.png && magick identify $_
```

If staging ortho is missing, run full pipeline: `catalog-sap-cells.mjs` → `stitch-sap-ortho.mjs` (baseline, pre-fix analysis).

---

## Investigation (P0 — gate before blind blending)

**Create:** `scripts/map-assets/analyze-sap-seams.mjs`

| Check | Purpose |
|-------|---------|
| Sample **edge strips** (default 8 px) along each **vertical** boundary `x = k * 256` for `k = 1..49` | Left vs right strip mean ΔRGB, max Δ, stddev |
| Sample **horizontal** boundaries `y = k * 256` | Top vs bottom strip same metrics |
| **Control:** random **interior** line not on 256 grid | Must show lower discontinuity than grid lines |
| Optional: decode adjacent cell pair raw before paste | Quantify BC7-only mismatch vs post-paste |

**Output:** `.ai/artifacts/t090_1_2_2_seam_analysis.json`

Suggested JSON shape:

```json
{
  "terrain": "everon",
  "orthoPath": "packages/map-assets/everon/staging/sap/everon-sap-ortho.png",
  "gridPx": 256,
  "stripPx": 8,
  "verticalEdges": [{ "x": 256, "meanDeltaRgb": 12.3, "maxDeltaRgb": 48, "p95": 31 }],
  "horizontalEdges": [],
  "controlInterior": { "meanDeltaRgb": 2.1 },
  "worstEdge": { "axis": "vertical", "x": 5120, "meanDeltaRgb": 18.7 },
  "diagnosis": "exposure_mismatch | placement | bc7_block"
}
```

**STOP** if analysis shows **placement bugs** (off-by-one row/col, internal cell mirror) rather than exposure — fix placement first (strategy D).

Print human summary to stdout; exit 0 always (analysis, not gate).

---

## Fix strategies (pick smallest that passes S1 + verify-sap-seams)

Apply in `stitch-sap-ortho.mjs` (or `blend-sap-seams.mjs` post-pass on RGBA canvas **before** PNG write):

| ID | Strategy | When |
|----|----------|------|
| **A** | **Edge feather** | Default — linear α blend 2–8 px on **interior** cell edges only (skip x=0, x=12800, y=0, y=12800 map borders) |
| **B** | **Per-cell gain/offset** | Edges differ mainly in brightness — match mean RGB on 16 px overlap band per edge |
| **C** | **Overlap discard** | Cells include redundant border texels — crop 1–2 px before paste |
| **D** | **Placement fix** | P0 shows row/col mirror or off-by-one at specific grid lines |

**Implementation notes for A (feather):**

- Blend in **paste order** or second pass over boundary bands only — avoid double-darkening corners (4-cell crosses).
- Preserve north-up row flip logic (lines 66–77 in current stitch) — feather must operate in **final canvas space**.
- Record `meta.seamRepairStrategy = "A-feather-4px"` (example) in `TBD_SatExport_meta.json`.

Forbidden: grey fill, blur-the-whole-map, AI inpainting.

---

## New ship gate: `verify-sap-seams.mjs`

**Create:** `scripts/map-assets/verify-sap-seams.mjs`

| Assert | Rule |
|--------|------|
| Ortho exists 12800² | Same as verify-sap-ortho |
| Grid edge discontinuity | For all interior 256 px grid lines, mean ΔRGB across 8 px strips **≤ threshold** |
| Threshold | Set from P0 baseline: post-fix **worst edge mean ΔRGB ≤ 50% of pre-fix worst** OR absolute cap **≤ 6.0** (normalized 0–255 channel delta) — **document chosen values in verify log** |
| Control | Interior non-grid sample line must stay **below** grid edge metrics |
| Regression | Must not increase global ortho stddev below `MIN_STDDEV` from verify-sap-ortho |

Exit 1 on fail; exit 0 prints `verify-sap-seams OK`.

---

## Rebuild + automated verify (full ship gate)

```bash
# 1) P0 baseline (if not done on current ortho)
node scripts/map-assets/analyze-sap-seams.mjs TERRAIN=everon

# 2) Re-stitch with fix
node scripts/map-assets/stitch-sap-ortho.mjs TERRAIN=everon

# 3) Post-fix analysis + gates
node scripts/map-assets/analyze-sap-seams.mjs TERRAIN=everon
node scripts/map-assets/verify-sap-seams.mjs TERRAIN=everon
node scripts/map-assets/verify-sap-ortho.mjs TERRAIN=everon

# 4) Lossless pyramid (minutes; ~299M LFS)
scripts/map-assets/build-tile-pyramid.sh \
  --input packages/map-assets/everon/staging/sap/everon-sap-ortho.png \
  --out packages/map-assets/everon/tiles/satellite \
  --minzoom 0 --maxzoom 6 --tilesize 256 --lossless

EXPECT_LOSSLESS=1 node scripts/map-assets/verify-tile-pyramid.mjs TERRAIN=everon
make verify-terrain
make ci-local-frontend
```

Commit LFS tiles + any manifest/meta updates. Tag **`T-090.1.2.2`**.

---

## Manual acceptance

| ID | Pass |
|----|------|
| **S1** | Operator screenshot location — **no visible line** at former seam @ max zoom |
| **S2** | Spot-check 3 other cell intersections (road, forest, coast) |
| **S3** | North-up + H1/H2 alignment unchanged (`verify-sap-ortho` orientation guard) |
| **S4** | Pan/zoom ≥55 fps (pyramid structure unchanged — no frontend work expected) |

Log: `.ai/artifacts/t090_1_2_2_verify_log.md` (include before/after worst-edge metrics from analysis JSON).

---

## Tasks (implementer checklist)

1. **`analyze-sap-seams.mjs`** — P0 metrics → `t090_1_2_2_seam_analysis.json` (run on pre-fix ortho if available).
2. **`stitch-sap-ortho.mjs`** — implement chosen strategy; bump meta `seamRepair` fields.
3. **`verify-sap-seams.mjs`** — automated grid-edge gate with documented threshold.
4. **Re-stitch + verify** — `verify-sap-seams` + `verify-sap-ortho` PASS.
5. **Pyramid rebuild** — lossless z0–6; `EXPECT_LOSSLESS=1 verify-tile-pyramid` PASS.
6. **CI gates** — `make verify-terrain` + `make ci-local-frontend`.
7. **Verify log** — `.ai/artifacts/t090_1_2_2_verify_log.md` with S1–S4 + analysis summary.
8. **Commit + tag** `T-090.1.2.2` — **do not** edit docs/registry.

---

## Files (expected touch list)

| Action | Path |
|--------|------|
| **Create** | `scripts/map-assets/analyze-sap-seams.mjs` |
| **Create** | `scripts/map-assets/verify-sap-seams.mjs` |
| **Edit** | `scripts/map-assets/stitch-sap-ortho.mjs` (or `blend-sap-seams.mjs` + import) |
| **Replace** | `packages/map-assets/everon/tiles/satellite/**` (LFS rebuild ~299M) |
| **Maybe edit** | `packages/map-assets/everon/manifest.json` (only if meta fields added) |
| **Artifacts** | `.ai/artifacts/t090_1_2_2_seam_analysis.json`, `.ai/artifacts/t090_1_2_2_verify_log.md` |

**Do not touch** unless S4 fails: `useTerrainBasemapLayer.ts`, `decode-edds.mjs`, `vendor/bc7.mjs`.

---

## Ship

Tag **`T-090.1.2.2`** · commit prefix **`T-090.1.2.2:`** · tell Cursor **"doc sync for T-090.1.2.2"** → `active_slice` → **T-090.1.2.3**.

---

## Documentation sync (Cursor — after human merge)

```bash
./scripts/ticket ship T-090.1.2.2   # or manual registry shipped_at + active_slice
./scripts/ticket sync
```

Update [`t090_1_2_satellite_backlog.md`](t090_1_2_satellite_backlog.md) seam row + hub status.

---

## Claude Code prompt — T-090.1.2.2 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**  
Extract: `./scripts/ticket prompt T-090` · standard: [`.ai/tickets/CLAUDE_CODE_PROMPT.md`](../../../.ai/tickets/CLAUDE_CODE_PROMPT.md)

```
Read CLAUDE.md first.

Implement **T-090.1.2.2** — SAP supertexture cell seam repair.

═══ PREFLIGHT ═══
  git pull && git lfs pull && make map-assets-link
  ./scripts/ticket brief T-090
  export ENFUSION_GAME_PATH="${ENFUSION_GAME_PATH:-$HOME/.cache/enfusion-mcp-root}"
  command -v magick && command -v cwebp

═══ READ (in order — spec wins on conflict) ═══
  1. .ai/artifacts/t090_1_2_2_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t090_1_2_2_sap_cell_seam_repair.md
  3. scripts/map-assets/stitch-sap-ortho.mjs
  4. scripts/map-assets/decode-edds.mjs
  5. scripts/map-assets/verify-sap-ortho.mjs

═══ PROBLEM ═══
  Satellite @ max zoom shows ~256 m grid seams where 50×50 SAP cells meet in the stitched ortho.
  Hard edge-to-edge paste in stitch-sap-ortho.mjs; BC7 border mismatch. NOT pyramid tile seams.
  T-090.1.2.1 lossless VP8L preserved the artifact faithfully.

═══ SHIPPED (do not reopen) ═══
  - T-090.1.2 @ c2730a3 — SAP decode/stitch/orientation
  - T-090.1.2.1 @ 19bc785 — lossless z0–6 pyramid encode

═══ LOCKED ═══
  - P0 analyze-sap-seams.mjs before blind blending; STOP if placement bug (strategy D)
  - Default fix: edge feather 2–8 px on interior cell edges (strategy A)
  - Preserve north-up row-flip in stitch (lines 66–77); orientation guard must PASS
  - Rebuild: build-tile-pyramid.sh --lossless --maxzoom 6 (~299M LFS)
  - No z7, AI upscale, grey fill, whole-map blur, decode contract changes
  - Full locked table: spec §Locked decisions

═══ DO ═══
  1. P0 — analyze-sap-seams.mjs → .ai/artifacts/t090_1_2_2_seam_analysis.json (baseline)
  2. Fix stitch-sap-ortho.mjs (smallest strategy A–D that passes)
  3. Create verify-sap-seams.mjs (threshold from P0 baseline)
  4. Re-stitch; post-fix analyze; verify-sap-seams + verify-sap-ortho PASS
  5. Rebuild lossless z0–6 pyramid; EXPECT_LOSSLESS=1 verify-tile-pyramid
  6. make verify-terrain && make ci-local-frontend
  7. .ai/artifacts/t090_1_2_2_verify_log.md (S1–S4 + before/after metrics)
  8. Tag **T-090.1.2.2** · prefix **T-090.1.2.2:**

═══ DO NOT ═══
  - Edit docs/**, registry, docs/TICKET_*.md, CLAUDE status markers
  - useTerrainBasemapLayer.ts unless S4 fails
  - decode-edds.mjs / vendor/bc7.mjs unless P0 proves decode bug

═══ VERIFY (all exit 0) ═══
  node scripts/map-assets/analyze-sap-seams.mjs TERRAIN=everon
  node scripts/map-assets/stitch-sap-ortho.mjs TERRAIN=everon
  node scripts/map-assets/verify-sap-seams.mjs TERRAIN=everon
  node scripts/map-assets/verify-sap-ortho.mjs TERRAIN=everon
  scripts/map-assets/build-tile-pyramid.sh \
    --input packages/map-assets/everon/staging/sap/everon-sap-ortho.png \
    --out packages/map-assets/everon/tiles/satellite \
    --minzoom 0 --maxzoom 6 --tilesize 256 --lossless
  EXPECT_LOSSLESS=1 node scripts/map-assets/verify-tile-pyramid.mjs TERRAIN=everon
  make verify-terrain && make ci-local-frontend

═══ MANUAL ═══
  S1: operator seam location invisible @ max zoom
  S2: 3 other intersections (road, forest, coast)
  S3: north-up alignment unchanged
  S4: pan fps ≥55

═══ RETURN ═══
  - Commit SHA + tag T-090.1.2.2
  - Seam analysis before/after (worst edge mean ΔRGB) + strategy used
  - All automated verify output (PASS)
  - S1 landmark note
  - **Ready for Cursor doc sync.**
```

---

## Related

- Handoff: [`.ai/artifacts/t090_1_2_2_claude_code_handoff.md`](../../../.ai/artifacts/t090_1_2_2_claude_code_handoff.md)
- Send-off: [`.ai/artifacts/t090_1_2_2_SEND_TO_CLAUDE.md`](../../../.ai/artifacts/t090_1_2_2_SEND_TO_CLAUDE.md)
- Resume: [`t090_1_2_satellite_backlog.md`](t090_1_2_satellite_backlog.md)
- Pan flicker next: **T-090.1.2.3**
