# T-165.4 verify log — golden S-gates + terrain/DEM + label gates → Rust

Slice ships in four commits: **.4a** (tbd-tools libs), **.4b** (S2–S14 golden gate), **.4c**
(glyphs + height-labels), **.4 close** (this commit: locations/town/road label gates,
terrain-alignment, aggregator flip, Makefile npm = 0, edge-list deletions).

## Ported this slice (cumulative)

| Node | Rust | Parity evidence |
|---|---|---|
| `lib/anchor-check.mjs` | `tbd_tools::geometry::check_anchors` | S12 via golden gate (below) |
| `lib/density-grid.mjs` | `tbd_tools::density` + core `encode_tbdd` (promoted `pub`) | S13 encode **byte-identity** vs committed `density-fixture.bin`; round-trip @ 1172 B; corner partition sum==count |
| `lib/forest-regions.mjs` (275 LOC) | `tbd_tools::forest` | S14 Value-equality vs fixture (`js_num` integral-f64→JSON-int semantics); F2 identity |
| `verify-map-object-golden.mjs` | `xtask schema map-object-golden` | S2–S9+S11–S14 12/12, verdict-set + rc parity side-by-side; negative probe both rc=1 |
| `verify-map-glyphs-manifest.mjs` | `xtask schema map-glyphs` | GL-G1..G6 parity; 0xFF-flip probe both rc=1 |
| `verify-height-labels.mjs` + `lib/height-labels-export.mjs` | `xtask schema height-labels` | Pure gates parity (floor probe value_m=10 → both rc=1). **Native restore:** wasm-era branch (G5/G6 declutter via core, ASL ±0.5 m oracle via `decode_png_to_meters` + `sample_elevation_from_meters_cache`, G3 completeness) — Node runner permanently skips these (retired wasm import); probe value_m=999: node rc=0 / rust rc=1 → Rust strictly stronger, by design |
| `verify-locations.mjs` | `xtask schema locations` | node rc=0 / rust rc=0; required-town removal probe: both rc=1 |
| `verify-town-labels.mjs` | `xtask schema town-labels --zoom=-2` | G1–G5 + fade-α endpoints + band edges on core `declutter_town_labels`/`town_label_fade_alpha`; probe rc=1 |
| `verify-road-names.mjs` | `xtask schema road-names --zoom 0` | G3–G7 on core `place_road_labels`/`declutter_road_labels`/`perpendicular_dist_to_polyline`; caps ≤24, perp ≤12 m |
| `verify-terrain-alignment.mjs` + `lib/dem-sample.mjs` | `xtask schema terrain-alignment [--strict]` | **Byte-identical stdout** both modes (11-anchor table, maxDeltaM=0.204, path-prefix normalized). Sampling = png-crate u16 decode + core `sample_elevation_meters` (bilinear∘affine ≡ affine∘bilinear ⇒ exact f64 parity). `js_fixed3` mirrors JS `toFixed(3)` tie-away-from-zero (anchors hold exact dyadic ties 0.0625 / −18.3125 where `{:.3}` half-to-even diverges). Probe: surfaceYM 80.875→95.0 → node rc=1 / rust rc=1; restore rc=0 |

## Rewired

- `make verify-terrain` / `verify-terrain-strict` → cargo (was the **last 2 npm lines in the
  Makefile**; Makefile npm surface now **0**).
- `make schema-validate` = 100% cargo (since .4c), re-verified rc=0 end-to-end.
- `scripts/map-assets/verify-t152-cartographic.mjs` label steps → `runCargo` (height-labels,
  locations, town-labels `--zoom=-2`, road-names `--zoom 0`); aggregator rc=0 end-to-end.
- `packages/tbd-schema/package.json` scripts → `{}` (package retained: ajv/pngjs
  `createRequire` borrowers live until T-165.9).

## Deleted (reverse-dep edge list clear)

`verify-locations.mjs`, `verify-height-labels.mjs`, `lib/height-labels-export.mjs` (sole consumer
was its verifier), `verify-town-labels.mjs`, `verify-road-names.mjs`,
`verify-terrain-alignment.mjs`, `lib/dem-sample.mjs` (sole importer was terrain-alignment).
Stale-ref sweep across scripts/packages/Makefile/.github/xtask/tools: **0 hits**.

**Kept (edges still live):** `raw-u16-to-dem-png.mjs` (spawned by copy-world-export-profile.mjs —
ports @ .8), `lib/locations-export.mjs` (imported by export-locations.mjs builder),
`verify-type-inventory.mjs` (spawned by census-types.mjs), tbd-schema npm tree (borrowers → .9).

## Gate suite at close

- `make schema-validate` rc=0 (pure cargo, zero node)
- `make verify-terrain` rc=0 · `make verify-terrain-strict` rc=0 (byte-identical vs Node)
- `node scripts/map-assets/verify-t152-cartographic.mjs` rc=0
- `cargo xtask schema t090-specs` — 36 spec files, 12/12 gates
- `cargo clippy -p xtask -p tbd-tools --all-targets -- -D warnings` rc=0 (one test-only
  `u64::from(&u32)` fixed) · `cargo fmt --check` rc=0 · `cargo test -p tbd-tools` 2/2
- `./scripts/ticket check` OK
