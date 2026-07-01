# Copy-paste prompt for Claude Code — T-090.1.2.2

Copy everything inside the fenced block below into Claude Code (`./scripts/ticket run` or direct session on branch `ticket/T-090`).

---

```
Read CLAUDE.md first.

Implement T-090.1.2.2 — SAP supertexture cell seam repair.

Preflight:
  git pull && git lfs pull && make map-assets-link
  ./scripts/ticket brief T-090
  export ENFUSION_GAME_PATH="${ENFUSION_GAME_PATH:-$HOME/.cache/enfusion-mcp-root}"
  command -v magick && command -v cwebp

Read (in order):
  .ai/artifacts/t090_1_2_2_claude_code_handoff.md
  docs/specs/Mission_Creator_Architecture/t090_1_2_2_sap_cell_seam_repair.md
  scripts/map-assets/stitch-sap-ortho.mjs
  scripts/map-assets/decode-edds.mjs
  scripts/map-assets/verify-sap-ortho.mjs
  .ai/artifacts/t090_1_2_1_verify_log.md

PROBLEM:
  At max zoom, Satellite basemap shows visible ~256 m grid seams (vertical lines + crosses at
  cell corners). This is NOT WebP pyramid tile seams — it is the stitched SAP ortho from
  stitch-sap-ortho.mjs: 2500 cells (256×256 px) pasted hard edge-to-edge with no feather.
  T-090.1.2.1 lossless VP8L pyramid preserved the artifact faithfully. BC7 decode is 4×4
  block lossy; adjacent cells can mismatch at borders.

LOCKED:
  - Fix compositing in stitch-sap-ortho.mjs (default strategy A: 2–8 px edge feather on
    interior cell edges only; skip map outer border)
  - P0 analyze-sap-seams.mjs BEFORE blind blending — STOP if placement bug (strategy D)
  - Do NOT change decode-edds.mjs / BC7 contract unless analysis proves decode bug
  - Preserve north-up row-flip assembly in stitch (lines 66–77) — orientation guard must PASS
  - Rebuild lossless z0–6 pyramid identical to T-090.1.2.1
  - No z7, no AI upscale, no grey fill, no whole-map blur
  - DO NOT edit documentation or registry

DO (execution order):
  1. Create scripts/map-assets/analyze-sap-seams.mjs
     - Sample 8 px strips on all interior 256 px grid lines (vertical + horizontal)
     - Control: interior non-grid line
     - Write .ai/artifacts/t090_1_2_2_seam_analysis.json
     - Run on current ortho first (baseline metrics)

  2. Fix scripts/map-assets/stitch-sap-ortho.mjs (or blend-sap-seams.mjs post-pass)
     - Smallest strategy that passes gates (A feather default)
     - Bump TBD_SatExport_meta.json: seamRepair + strategy id

  3. Create scripts/map-assets/verify-sap-seams.mjs
     - Grid-edge mean ΔRGB threshold (post-fix ≤ 50% pre-fix worst OR absolute ≤ 6.0)
     - Document chosen threshold in verify log

  4. Re-stitch + verify:
     node scripts/map-assets/stitch-sap-ortho.mjs TERRAIN=everon
     node scripts/map-assets/analyze-sap-seams.mjs TERRAIN=everon
     node scripts/map-assets/verify-sap-seams.mjs TERRAIN=everon
     node scripts/map-assets/verify-sap-ortho.mjs TERRAIN=everon

  5. Rebuild pyramid:
     scripts/map-assets/build-tile-pyramid.sh \
       --input packages/map-assets/everon/staging/sap/everon-sap-ortho.png \
       --out packages/map-assets/everon/tiles/satellite \
       --minzoom 0 --maxzoom 6 --tilesize 256 --lossless
     EXPECT_LOSSLESS=1 node scripts/map-assets/verify-tile-pyramid.mjs TERRAIN=everon

  6. Ship gates:
     make verify-terrain
     make ci-local-frontend

  7. Write .ai/artifacts/t090_1_2_2_verify_log.md (S1–S4 + before/after seam metrics)

  8. Commit LFS tiles + scripts. Tag T-090.1.2.2. Commit message prefix: T-090.1.2.2:

MANUAL (Mission Creator @ max zoom, Satellite view, hillshade optional):
  S1 — operator seam location: no visible line
  S2 — 3 other cell intersections (road, forest, coast)
  S3 — north-up alignment unchanged
  S4 — pan fps ≥55

RETURN:
  - Commit SHA + tag T-090.1.2.2
  - Seam analysis before/after summary (worst edge mean ΔRGB)
  - Strategy used + parameters (e.g. feather 4 px)
  - All automated verify output (PASS)
  - S1 landmark note
  - "Ready for Cursor doc sync."
```

---

## Alternate: ticket runner

```bash
git checkout main && git pull && git lfs pull
git checkout -B ticket/T-090 main   # or existing branch
./scripts/ticket run T-090            # runs claude-code slices only; active = T-090.1.2.2
```

Paste the fenced prompt above if the runner needs explicit scope.
