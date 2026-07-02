# T-090.1.2.8 — Verify log (unified satellite texture)

**Slice:** T-090.1.2.8 · **Format:** tbd-sat v1 (decision: [`t090_1_2_8_format_spike.json`](t090_1_2_8_format_spike.json))
**Bundle:** `packages/map-assets/everon/satellite/everon-sat.tbd-sat` — 205.9 MB, 14 mips, 17 VP8L blocks (LFS)

## Automated gates (2026-07-02, Claude Code)

| Gate | Command | Result |
|------|---------|--------|
| Bundle build | `node scripts/map-assets/build-unified-satellite.mjs --input …/everon-sap-ortho.png --out …/everon-sat.tbd-sat --terrain everon` | **OK** — 205.9 MB in 79 s (level 0 = 4×6400² blocks 148.5 MB, level 1 42.3 MB, …, level 13 1×1) |
| Bundle structure | `node scripts/map-assets/verify-unified-satellite.mjs TERRAIN=everon` | **OK everon — 12800x12800, 14 mips, 17 VP8L blocks, 205.9 MB** (magic/version/JSON, GL halving chain, block ranges, VP8L fourcc + header dims, grid coverage, manifest agreement incl. bytes) |
| Block decode | `dwebp` on sliced blocks (levels 0, 1, 6, 13) | **OK** — all decode at declared dims |
| Lossless + tile order | `magick compare -metric AE` bundle level-0 quadrant 0 vs source top-left 6400² crop | **AE = 0 (bit-identical)** |
| Schema contracts | `cd packages/tbd-schema && node scripts/validate.mjs` | **PASS** incl. new golden `everon-unified-satellite` (+ dual/legacy manifests still pass) |
| Manifest cross-check | `node scripts/verify-terrain-manifest.mjs --terrain everon` | **PASS** (schema + terrains.ts) |
| FE tests | `npm run test` | **38/38** (+8 tbd-sat parser + 4 pickBaseLevel) |
| FE build | `npm run build` | **clean** (tsc + vite) |
| FE lint | `npm run lint` | **clean** |
| Serve smoke | vite dev `HEAD /map-assets/everon/satellite/everon-sat.tbd-sat` | **200**, `Content-Length: 205896703` (matches manifest `bytes` → determinate progress), body starts `TBDS`+v1 |

**Note — `make schema-validate`:** the composite target fails in `verify-t090-specs` on **6 pre-existing doc-lint findings** (spec docs reference `npm run build/lint`; program hub header names T-090.3.0). Confirmed pre-existing by re-running on a clean stash of this change — identical findings, zero deltas from this slice. Docs are locked to Cursor (no-docs rule); flagged for the doc sync pass. Every sub-gate this slice touches (validate.mjs, verify-terrain-manifest) passes.

## Manual acceptance (operator)

Run `make db-up api web`, dev-login, open a mission in the editor. Expect **one** `.tbd-sat` network fetch (progress toast → "Satellite ready"), preview `full.webp` visible within ~1 s, then the sharp texture swap; **zero** tile requests while panning/zooming afterwards.

| ID | Check | Status |
|----|-------|--------|
| U1 | Pan @ max zoom — no tile pop-in / flicker | **PENDING OPERATOR** |
| U2 | Zoom in/out — smooth GPU mip (trilinear), no discrete layer swap | **PENDING OPERATOR** |
| U3 | Detail acceptable @ operational zoom (SAP source — ~256 m apron grid may show @ max MC zoom, out of scope per spec) | **PENDING OPERATOR** |
| U4 | Pan fps ≥55 (FpsCounter) | **PENDING OPERATOR** |

**Structural argument for U1/U2:** unified mode renders exactly one full-extent `BitmapLayer` whose texture never changes identity after load — there is no layer mount/unmount and no HTTP on pan/zoom by construction; zoom LOD is the GPU's per-fragment mip selection (trilinear sampler baked at texture creation).

**Fallback verified by code path:** any unified fetch/parse/decode failure logs a warning, dismisses the progress toast, and re-resolves with the pyramid forced (kill switch: set manifest `tiles.satellite.delivery` back to `"pyramid"`). Grid-only toast only fires if the pyramid is also absent.

**Device adaptivity:** GPUs with `maxTextureDimension2D < 12800` (e.g. 8192) start the texture at the 6400² mip — the 655 MB base decode is skipped entirely (unit-tested `pickBaseLevel`). Full base ≈ 873 MB VRAM; 6400² base ≈ 218 MB.
