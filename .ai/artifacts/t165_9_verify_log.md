# T-165.9 verify log — image pipeline → pure Rust; tbd-schema npm deleted

Scope: the last big lane — every image builder/verifier under scripts/map-assets plus the
label exporters, onto `tbd_tools::map` and the new `map` bin. Encoders per the N3 policy:
image-webp (pure Rust) for every lossless leg; the `webp` crate (vendored libwebp C, no
external process) for the single lossy leg (the map-view pyramid's `webp-lossy` contract);
resvg replaces the magick RSVG/MVG passes; png+image replace decode/resize/identify.

## Verifier parity (side-by-side on committed/staged assets)

| Gate | Result |
|---|---|
| `map verify-unified` vs verify-unified-satellite.mjs | rc 0/0, **stdout byte-identical** (205.9 MB TBDS bundle fully parsed: 14 mips, VP8L blocks, manifest agreement) |
| `map verify-pyramid` (satellite + `--view-map`) | rc 0/0 both views, **stdout byte-identical** |
| `map verify-sap-seams` | rc 0/0, **stdout byte-identical** (78 textured seams, FILL/STEP/ANCHOR/CONTROL + stddev after the magick-semantics fix) |
| `map verify-sap-ortho` | rc 0/0, identical except the orientation-guard AE readout (0.008 vs 0.003 — Lanczos-vs-magick resampler; threshold 0.2, match ≈0.08 band) |
| `map verify-t152` | **rc=0 end-to-end** — slice logs + make map-glyphs-verify + map-export-validate + map-verify-phase P5 + the 4 Rust label gates |

Two ImageMagick semantics were reverse-engineered en route and pinned in code comments:
`%[fx:standard_deviation]` averages per-channel stddevs over EVERY channel including a
constant alpha plane (÷4 on TrueColorAlpha), and the image crate's default decode limits
reject 12800² inputs (`no_limits()`).

## Builder acceptance

- **Glyph atlas** (resvg + image-webp): rebuilt over the 29 committed SVGs —
  `world-glyphs.json` mapping **byte-identical to committed**; `xtask schema map-glyphs`
  green on the Rust-encoded atlas (41.5 KB; committed webp becomes Rust-encoded, gate-safe
  by design — no gate byte-compares webp).
- **Water classifier** (`map analyze-water`): on the pre-water ortho (the pipeline's actual
  input — step 1 of `make map-water-everon` restores it): **153/153 accepted bodies exact**
  vs the committed spike (classes 123 wet-channel / 24 grey-river / 6 compact), road
  corridor **256935 px exact**, airfield-flat check 0.833 exact, rejected components 9153
  vs 9154 (one border-speckle component at the resampler level; the accepted set is what
  ships). Committed spike JSON + staged mask restored after the probe.
- **Pyramid builder**: lossy-leg smoke (map ortho → z0–1 + full.webp, VP8 tiles) green.
- **Unified builder + stitch/blend/composite/landcover/cartographic**: ported (stitch runs
  the .8-proven pak→EDDS→BC7 chain that is RGBA-hash-identical to the Node decoder; the
  compose replaces the magick MVG road pass with resvg polylines). Full-rebuild visual
  output = operator review at next rebuild per the plan's acceptance.

## Exporters

- `map export-locations`: output **byte-identical to the Node exporter AND to the committed
  locations.json** (zero churn) after fixing the sort to JS `localeCompare` case-insensitive
  collation (byte order filed "Peninsula" before "beach").
- `map export-height-labels`: the Node script has been dead since T-159.29.3 (it imported
  the deleted React-era wasm pkg) — **natively restored** on `map_engine_core::dem`
  (decode_png_to_meters/find_peaks/sample/declutter). Output **byte-identical to the
  committed height-labels.json** (26 labels, 23 named + 3 DEM, floor drops identical).

## Rewired / deleted

- Makefile: the entire image lane — `map-water-everon` (7 lines incl. two `node -e` JSON
  patches), `map-cartographic-everon` (+ its `node -e`), `map-cartographic-verify`,
  `map-glyphs-build` — now calls the `map` bin. **One `node` line remains in the Makefile**
  (`verify-file-length.mjs`, T-165.10 scope).
- **Deleted 28 files**: decode-edds/decode-topo, vendor/{bc7.mjs,bc7.test.mjs,bcdec.wasm,
  bcdec_bc7.c,bcdec.h,build-bcdec-wasm.sh}, stitch/blend + lib/sap-seam-metrics,
  verify-{sap-ortho,sap-seams,unified-satellite,tile-pyramid,t152-cartographic} +
  analyze-sap-seams, build-{unified-satellite,glyph-atlas,landcover-mask,map-cartographic} +
  composite-water-ortho + analyze-water-sources, export-locations + lib/locations-export,
  export-height-labels + lib/road-names, build-tile-pyramid.sh.
- **npm removed from packages/tbd-schema**: package.json + package-lock.json + node_modules
  (the last ajv/pngjs borrowers died with this lane). xtask gate-7 now tolerates the deleted
  manifest (empty live-script set; the retired-script allowlist carries the archival names).
- Tracked non-mod `.mjs`: **25 → 2** (`scripts/website/{verify-file-length,
  gen-text-font-table}.mjs` — the T-165.10 closure set).

## Fixed en route

- **T-165.8.2** (committed separately): `map-export-validate`/`map-verify-phase` Makefile
  recipes still called the deleted .8 .mjs (the .8 flip script died on a mid-run assert
  after rewriting the shell orchestrators but before the Makefile section).
- catalog-sap-cells E2c-allow markers moved onto the offending lines (the gate matches
  per-line); `make map-export-validate` re-verified rc=0 directly (not through a pipe).

## Gate suite at close

`map verify-t152` rc=0 · `xtask schema t090-specs` 12/12 · `make schema-validate` rc=0 ·
`make map-export-validate` rc=0 · `./scripts/ticket check` OK ·
`cargo clippy -p tbd-tools -p xtask --all-targets -- -D warnings` 0 errors · fmt clean ·
`cargo test -p tbd-tools` 6/6.
