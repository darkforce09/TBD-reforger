# Unified satellite container and tile pyramid checks

The satellite side of a terrain's map assets: building the unified `TBDS` container
(`satellite/<terrain>-sat.tbd-sat`) from a stitched source raster, and verifying that container and
the terrain's XYZ tile pyramids against `manifest.json`. These files are the submodules
`tools_v2/developer-tools/src/map_raster_pipeline/satellite_archive.rs` declares; it re-exports the
three entry points, and the container's version 2 writer and reader live in the sibling
`satellite_archive_container.rs`.

## Contents

```text
tools_v2/developer-tools/src/map_raster_pipeline/satellite_archive/
├── build_unified_satellite.rs  `build-unified`: mip chain, tiling, VP8L encode, version 1 or 2 container
└── map_assets_root.rs          `verify-unified` and `verify-pyramid`, and the terrain assets root
```

## How it works

```text
source PNG (+ TBD_SatExport_meta.json beside it)
  ─▶ build_unified_satellite ─▶ mip chain to 1×1 ─▶ tiles of at most --tile-threshold px
  ─▶ lossless VP8L WebP blocks ─▶ version 2 (default) or version 1 container ─▶ --out
  ─▶ prints the manifest block (delivery, path, url, encoding, base size, mip count, bytes)
```

- `build_unified_satellite` knows the world bounds of `everon` (12,800 square) and `arland` (4,096
  square) and exits 1 for any other terrain, an unknown `--container-version` or a missing input. It
  halves the image level by level with `super::image_operations`, cuts each level into a grid of
  equal tiles (Everon's 12,800 px base at the default threshold of 8,192 becomes 2 × 2 tiles of
  6,400 px), and encodes every tile as lossless WebP. Both container versions are written from the
  same block vector, so their payloads are byte-identical and only the index differs: version 1 is
  `TBDS`, a `u32` version and a hand-packed JSON table carrying the source metadata and the input's
  SHA-256; version 2 is the 32-byte `TbdsHeader` and an rkyv `TbdSatIndexV2`. It prints the manifest
  block and never edits `manifest.json`; `map patch-unified-bytes` writes the byte size.
- `verify_unified_satellite` reads the terrain's manifest, refuses a missing bundle, a Git LFS
  pointer and a wrong magic, reads the version from bytes 4–8 and runs the version 1 checks here or
  the version 2 checks in `satellite_archive_container.rs`, then holds the manifest's `delivery`,
  `encoding`, `url`, base size, `mipCount` and `bytes` to what the file holds.
- `verify_tile_pyramid` checks `tiles/satellite/` (or `tiles/map/` with `--view-map`): every zoom
  level from the manifest's `minZoom` to `maxZoom` is complete, every tile is a WebP of
  `tileSizePx`, and, for the satellite view when `--expect-lossless` is given or the manifest says
  `webp-lossless`, every tile is VP8L. A terrain with no pyramid on disk prints `SKIP` and exits 0,
  because the pyramids are gitignored.

## Boundaries

- Depends on: `super::image_operations` for PNG decode, resizing and WebP encode;
  `super::satellite_archive_container` for the version 2 index, bytes and checks, and through it
  `website_map_engine::io::archives::satellite` and `website_map_engine::io::containers::tbds`;
  `crate::repository_layout::terrain_assets_dir` and `crate::browser_testing::server::repo_root` for
  `assets_v2/terrains/`; `crate::timestamp_formatting` and
  `crate::world_export_pipeline::json_number_formatting` for the version 1 table.
- Used by: `tools_v2/developer-tools/src/map_raster_pipeline/cli.rs` (`build-unified`,
  `verify-unified`, `verify-pyramid`); the `map-water-everon`, `map-cartographic-everon` and
  `map-cartographic-verify` tasks of `cargo xtask ci`; the container tests in
  `tools_v2/developer-tools/src/map_raster_pipeline/tests/satellite_archive_container/`.
- Rules: a version 1 and a version 2 container built from one source carry the same tiles at the
  same rectangles (`v1_and_v2_carry_the_same_tiles_at_the_same_rects`), and the version 2 tiling
  reproduces the committed Everon container's
  (`the_v2_derivation_reproduces_the_committed_everon_tiling`); the committed
  `assets_v2/terrains/everon/satellite/everon-sat.tbd-sat` is a version 1 container, so a rebuild
  meant to replace it passes `--container-version 1` or updates the manifest's `unified.encoding`
  with it.

## Related documentation

- [Map engine archives](/apps/website/map-engine/src/io/archives/README.md) — the `TbdSatIndexV2`
  archive the map engine reads.
