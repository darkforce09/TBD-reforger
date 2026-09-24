# Satellite and map basemap loading

The browser side of the basemap: it reads a terrain's satellite image out of its `.tbd-sat`
container with HTTP Range requests, first a preview of at most 1024 pixels and then the full mip
chain at the largest level the GPU and the memory budget allow, and it loads the cartographic map
tiles in its place when a user picks the map view.

## Contents

```text
apps/website/map-engine/src/world/terrain/satellite/quadtree/
├── basemap.rs    the entry points: the satellite load, the map tile load, the satellite view
├── bootstrap.rs  the full mip chain: level choice, the memory-budget floor, fetch, decode, upload
├── decode.rs     WebP decode to an image bitmap, or to RGBA pixels under WebGL2; the tile upload
├── downloads.rs  the tiles' byte ranges, fetched in chunks with a bounded number in flight
├── mod.rs        the module tree; gathers the shared imports and re-exports the entry points
├── preview.rs    the preview level of at most 1024 pixels, and the `?sat=preview` switch
├── retry.rs      one Range request, tried up to five times with backoff
├── selection.rs  the GPU texture limit, the index read, and the report of a downscaled basemap
└── upload.rs     one decoded level committed as a single-level texture layer
```

## How it works

`crate::streaming::host` calls `load_satellite` at boot with the container's URL from the terrain
manifest (`tiles.satellite.unified.url`, `/map-assets/everon/satellite/everon-sat.tbd-sat` for
Everon):

```text
fetch_index_head      Range bytes 0-11 -> index_range_end -> Range for header + index
preview (try_preview) parse_tbd_sat_index_only; pick_preview_level(index, 1024) -> fetch its
                      tiles, decode, commit as a single-level layer
?sat=preview          the load stops here
full chain            parse_tbd_sat_index_strict; pick_base_level_for_limit(device limit);
(load_unified_full)   claim_satellite_floor may raise the level to fit the memory budget;
                      fetch every tile from that level down to 1x1, decode each, allocate one
                      texture with all those mips, upload each tile into its level, commit
```

The full chain reports its byte total and progress on the satellite boot segment and its texture
size and mip count to the statistics bridge, and announces a downscaled basemap with its cause: the
GPU's limit, a device granted less than its adapter offered, or the memory budget. It refuses to
choose a level without a render engine rather than guess a texture limit. Any Range that fails
five times, returns the wrong length or reports another file size, and any decode or upload
failure, abandons the full chain; a preview that loaded stays on screen with a warning.

`fetch_tiles` splits each tile's byte range into `SAT_CHUNK_BYTES` (4 MiB) requests, keeps
`SAT_FETCH_CONCURRENCY` (4) in flight and reassembles each tile in order. `fetch_range_resilient`
tries a request up to `RANGE_ATTEMPTS` (5) times, waiting 100, 250, 600 and 1200 ms after a 429
and half that after any other failure. `decode_webp` decodes through the browser's image bitmap
without colour-space conversion; under WebGL2 it draws the bitmap onto an offscreen canvas and
uploads RGBA pixels, and under WebGPU it uploads the bitmap itself.

`load_map_basemap` loads the cartographic map instead: the tiles
`/map-assets/<terrain>/tiles/map/{z}/{x}/{y}.webp` at the highest zoom up to 4 whose 2^z × 256
pixels fit the device's texture limit, stitched into one single-level texture in the same basemap
layer. `show_satellite_basemap` sets that layer's opacity back to 1.

## Boundaries

- Depends on: `crate::world::terrain::satellite::streamer` (the index parsers and level picks);
  `crate::streaming` (Range fetches, boot progress, the statistics bridge, the memory budget);
  `crate::frame` (the render engine and its texture-layer methods, defined in
  `crate::world::terrain::satellite::textures`); the browser's fetch, image bitmap and offscreen
  canvas.
- Used by: `crate::streaming::host`, which calls `load_satellite` at boot and
  `load_map_basemap` and `show_satellite_basemap` when the basemap view changes; and a
  source-scanning test of the [Mission Creator](/documentation_v2/glossary.md#mission-creator) in
  `apps/website/frontend/src/v2/apps/editor/tests/`, which reads all nine files by path.
- Rules: the folder compiles only for wasm32 with the `render` feature and runs only in a
  browser, so that source-scanning test holds its rules: a level is never chosen without the GPU's
  reported texture limit, a downscaled basemap always says why, and the full load logs what it
  loaded only after the commit. Renaming a file here means updating the test.
