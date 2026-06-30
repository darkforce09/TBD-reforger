# Copy-paste prompt for Claude Code

Send everything below the line to Claude Code (after `git pull` on `main`).

---

Implement **T-090.1.2.1** — lossless satellite pyramid (picture-perfect max zoom).

**Brief:**

```
./scripts/ticket brief T-090
```

**Read first (in order):**

1. `.ai/artifacts/t090_1_2_1_claude_code_handoff.md` — execution order
2. `docs/specs/Mission_Creator_Architecture/t090_1_2_1_lossless_satellite_pyramid.md` — ship gates + acceptance

**Problem:** Satellite basemap at max zoom is unacceptably blurry — lossy `cwebp -q 80` tiles + manifest capped at z5 while deck zoom goes to 6 (~1.56 m/px tiles stretched over 1 m/px native ortho).

**Do:**

1. Add `--lossless` to `scripts/map-assets/build-tile-pyramid.sh`
2. Rebuild z0–6 from existing ortho (do NOT re-stitch):

   ```bash
   scripts/map-assets/build-tile-pyramid.sh \
     --input packages/map-assets/everon/staging/sap/everon-sap-ortho.png \
     --out packages/map-assets/everon/tiles/satellite \
     --minzoom 0 --maxzoom 6 --tilesize 256 --lossless
   ```

3. Harden `verify-tile-pyramid.mjs` (complete levels + VP8L when `EXPECT_LOSSLESS=1`)
4. `manifest.json`: `maxZoom: 6`, `satellite.encoding: "webp-lossless"`
5. Ops log + `.ai/artifacts/t090_1_2_1_verify_log.md`
6. Run all ship gates in handoff; tag **`T-090.1.2.1`**

**Do not:** redo SAP decode/stitch; ship partial z6; use lossy q=95 instead of lossless; edit docs/registry.

**Return:** commit SHA, tag, verify output, tile dir size, **"Ready for Cursor doc sync."**
