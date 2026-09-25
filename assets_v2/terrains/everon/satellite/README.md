# Everon satellite image

Everon's satellite image as one file: a full-resolution mosaic of the island and its smaller
levels, cut into lossless WebP tiles behind an index. The map engine streams the level a device can
hold and uses it as the basemap of the
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s map.

## Contents

```text
assets_v2/terrains/everon/satellite/
└── everon-sat.tbd-sat  the 12,800 × 12,800 px satellite pyramid, 14 levels, in one `TBDS` container
```

## Format

- Encoding: a `TBDS` container, version 1, little-endian: the magic, the version and the index
  length as `u32`s, then a JSON index, then the tile bytes. The index gives the terrain, its world
  bounds, 1 m per pixel, and for each of the 14 levels (12,800 px down to 1 px) its size and tiles,
  each with its rectangle and its absolute byte offset and length; level 0 is four
  6,400 × 6,400 px tiles. The tiles are lossless WebP. The image is stitched from the game's
  supertexture cells, with the ocean and inland water tints composited in. 152,713,114 bytes,
  stored in Git LFS (`.gitattributes`: `assets_v2/terrains/**/*.tbd-sat`); a clone without LFS
  content holds a pointer file here.
- Schema: the manifest's `tiles.satellite.unified` block names the file, its URL, the encoding
  `tbd-sat-v1`, the base size, the level count and the byte size
  (`contracts_v2/definitions/terrain-manifest.schema.json`). The reader, which also reads the
  version 2 layout (a 32-byte header and an rkyv index), is
  `apps/website/map-engine/src/world/terrain/satellite/streamer/`.
- Adding a file: the committed file is written with `map build-unified --container-version 1`.
  `cargo xtask ci map-water-everon` rebuilds the image from the stitched mosaic in the gitignored
  export scratch and patches the manifest's byte count, but it runs `map build-unified` without
  `--container-version`, which writes version 2, so its closing `map verify-unified` step refuses
  the manifest's `tbd-sat-v1` encoding.

## Producers and consumers

- Producers: `map build-unified` (`tools_v2/developer-tools/src/map_raster_pipeline/`, the
  container in `satellite_archive_container.rs`), fed by the map raster pipeline's stitching and
  water composite steps.
- Consumers:
  - the map engine's satellite loader
    (`apps/website/map-engine/src/world/terrain/satellite/quadtree/`), which reads the header and
    the index with HTTP Range requests on `/map-assets/everon/satellite/everon-sat.tbd-sat`, shows
    a preview level first, then fetches every tile from the first level that fits the device's
    texture limit and memory budget down to 1 × 1; the
    [API](/documentation_v2/glossary.md#api) serves `/map-assets` below its rate limiter, so these
    requests are never refused;
  - `map verify-unified`, which checks the bundle against the manifest;
  - `cargo xtask ci lfs-sat`, which pulls only this LFS object, and the developer tools' headless
    editor checks, which serve it.

## Boundaries

- Depends on: the `tiles.satellite.unified` block of `assets_v2/terrains/everon/manifest.json`,
  which must state this file's encoding and byte count.
- Used by: the map engine, the map raster pipeline's verification, the headless editor checks and
  `cargo xtask ci lfs-sat`.
- Rules: the file and the manifest block change together, and `map verify-unified` refuses a
  manifest encoding that differs from the file's version; a Range request that the server answers
  with the whole file is refused, so the file is only ever read by parts.

## Related documentation

- [Satellite container format](/apps/website/map-engine/src/world/terrain/satellite/streamer/README.md)
  — both container versions and how a level is picked.
