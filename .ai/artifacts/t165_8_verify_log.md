# T-165.8 verify log — world-export pipeline → Rust + the E6 content-proven re-encode

The largest slice: the T-090 export/verify lane (16 .mjs, ~2.6k LOC) → `tbd_tools::world`
(`pak`/`topo`/`edds`/`classify`/`jsval`/`build`/`gates`/`aux` + the `world` bin), including
the one-time E6 gz-container migration the plan anchored at verify-phase.mjs:574.

## Decode substrate (proven byte-exact)

- **pak**: Enfusion `.pak` VFS ported from enfusion-mcp's reader/vfs (FORM/PAC1 chunk walk,
  recursive FILE entry tree, zlib inflate, sorted-pak first-wins merge).
- **topo**: `.topo` decoder — stats output **identical** to `node decode-topo.mjs` modulo the
  enfusion-mcp init log line (6 sections × 888 records, per-type histograms equal).
- **edds + BC7**: EDDS header/chunk-table + hand-ported LZ4 raw-block + **bcdec_rs** (pure
  Rust) replacing the vendored bcdec.wasm. The Pillow-derived independent golden from
  bc7.test.mjs ported as a Rust test (green); **10/10 sampled Eden supertexture cells
  (0/49/625/777/1250/1600/1875/2048/2450/2499) decode to sha256-identical RGBA** vs the Node
  wasm path — the full pak→LZ4→BC7 chain is byte-exact.

## Builders (content-identical)

`world build-objects` + `world build-roads` vs the Node builders, same staged raw
(1,409,998 lines), scratch-vs-scratch: **945/945 artifacts content-identical** (gz compared
decompressed; .bin/.json raw) — 315 chunks, 625 density bins, prefabs, roads, forest-regions,
type-inventory, sidecar manifest — and both summary stdout lines byte-equal. One fix en
route: `js_normalize` (rule JSON authors `1.0`; Node's parse+stringify prints `1`). JS
semantics preserved throughout: `js_num` integral numbers, `Math.round` = floor(x+0.5)
2-dp rounding, insertion-ordered maps, stable sorts, UTF-16-safe ASCII name sort.

## verify-phase (`world verify-phase`)

- **P5_props (committed phase): 15/15 gates PASS, rc=0** — output identical to the Node
  baseline except the chunk-gz aggregate KB (14823→14918; flate2 level-9 is ~0.6% larger,
  SIZE gate ≤40 MB far clear). Census line exact: **1623 prefabs, 1216109 instances, 315
  chunks, 888 road segments**.
- **P2_trees (phase-scope red lane): failure-set EXACT parity** — PH-P2-1 1232 + D2 312 =
  1544 errors on both sides, all other gate verdicts equal.
- E6 runs the double scratch build **in-process** (the Node gate spawned itself); the quiet
  flag keeps builder summaries out of gate stdout like stdio:pipe did.

## E6 / N5 migration (the plan's trap, executed)

1. Rust double-build determinism: byte-identical scratch pairs (no `nondeterministic:` rows).
2. Content-proof: committed artifacts ≡ Node scratch build, all 945 files (gz decompressed).
3. One-time re-encode: `world build-objects --patch-manifest --ops-log` + `build-roads`
   over the real tree — **exactly 318 .gz files changed, zero non-gz changes** (manifest,
   inventories, sidecar, 625 .bin byte-identical rewrites).
4. E6 raw-byte thereafter: `world verify-phase P5` rc=0 on the migrated tree.

## Aux lane (side-by-side)

- `world validate-exports` vs validate-export-artifacts.mjs: **stdout byte-identical**, rc
  0/0 (I-gates delegate to `xtask schema type-inventory`, output piped like stdio:pipe;
  E2c literal-terrain-id gate now scans the Rust modules + export-terrain.sh — topo.rs is
  the sanctioned config-table exclusion exactly as decode-topo.mjs was).
- `world census` vs census-types.mjs: stdout identical, rc 0/0.
- Spike lane (staging/spike present): `spike-k1` / `spike-census` / `spike-ops-log` all
  stdout-identical rc 0/0; the census-spike-rewritten `type-inventory-spike.json`
  **byte-identical**.
- `world sap-catalog` vs catalog-sap-cells.mjs: cell-catalog.json identical mod the
  wall-clock `generatedAt`.
- `world copy-export-profile` + `world raw-u16-dem-png` (png-crate encode; pixel-parity
  acceptance — IHDR 16-bit/grayscale + 3-pixel round-trip self-check preserved) +
  `world phase-gate` (the export-terrain.sh heredoc) ported.

## Rewired / deleted

- `export-terrain.sh` (phase gate + builders), `verify-spike-all.sh` (4 gates), Makefile
  `map-export-validate` / `map-verify-phase` / `map-census` → the `world` bin. Both shell
  chains re-run green end-to-end post-flip.
- **Deleted 16 .mjs** (tracked non-mod count 41 → 25): verify-phase, build-world-objects,
  build-roads-from-topo, validate-export-artifacts, census-types, verify-spike-k1,
  census-spike, verify-spike-ops-log, copy-world-export-profile, catalog-sap-cells,
  lib/{classify-prefab, anchor-check, density-grid, forest-regions},
  tbd-schema/{verify-type-inventory, raw-u16-to-dem-png}.
- **Kept for T-165.9** (image-lane importers): decode-topo.mjs (build-landcover-mask /
  build-map-cartographic / analyze-water-sources), decode-edds.mjs + vendor/bc7* (stitch /
  blend / seam-metrics) — the vendor wasm dies with decode-edds at .9.

## Gate suite at close

`xtask schema t090-specs` 12/12 · `make schema-validate` rc=0 · `./scripts/ticket check` OK ·
`cargo clippy -p tbd-tools --all-targets -- -D warnings` rc=0 · fmt clean ·
`cargo test -p tbd-tools` 6/6 · final `world verify-phase P5` rc=0.
