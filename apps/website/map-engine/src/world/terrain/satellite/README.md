# Satellite imagery

A terrain's satellite image on the 2D map: the reader of the `.tbd-sat` container that holds it,
the browser loads of its preview, of its full mip chain and of the cartographic map tiles that can
replace it, and the render engine's texture layers that show the basemap and the hillshade.

## Contents

```text
apps/website/map-engine/src/world/terrain/satellite/
├── mod.rs       the module tree
├── quadtree/    the browser loads: the satellite preview and full mip chain, and the map tiles
├── streamer/    the `.tbd-sat` container reader: header, index versions, checks and level picks
└── textures.rs  the render engine's texture layers: the basemap and the hillshade
```

## How it works

At boot `crate::streaming::host` takes the container's URL from the terrain manifest's
`tiles.satellite.unified` block and calls `quadtree::load_satellite`, which reads the index through
`streamer/` and uploads each level through the texture layers.

A texture layer is one of two roles: 0, the basemap (the satellite image or the cartographic map),
and 1, the hillshade that `crate::world::terrain::relief` computes and the host uploads at opacity
0.4. The layers are `RenderEngine` methods exported to JavaScript:

```text
tex_layer_begin        allocate an RGBA8 texture with its mips as the role's pending texture
                       (a role other than 0 or 1, or a zero size, is refused)
tex_layer_write_bitmap copy a decoded image bitmap into one level at (x, y)
tex_layer_write_rgba   copy RGBA bytes into one level at (x, y); the length must be w * h * 4
tex_layer_commit       draw the pending texture as one quad over its world rectangle, tinted to
                       the opacity, in place of the role's satellite or hillshade lane
tex_layer_clear        drop the role's lane and any half-written pending texture
```

A write or commit before `tex_layer_begin` is refused. The quad's corners are made relative to the
scene anchor (`crate::world::scene::world_rect_rel`), and a layer with mips counts a third more
bytes than its base level in the diagnostics.

## Public surface

- `streamer`: the container reader, whose README lists its items.
- `quadtree`: `load_satellite`, `load_map_basemap`, `show_satellite_basemap` and
  `sat_preview_only`.
- `textures`: the `RenderEngine` methods `tex_layer_begin`, `tex_layer_write_bitmap`,
  `tex_layer_write_rgba`, `tex_layer_commit` and `tex_layer_clear`; and, inside the crate,
  `TexLane` and `PendingTex`, the textured-lane bookkeeping the render engine keeps.

## Boundaries

- Depends on: `crate::io` (the `TBDS` container and the archived index); `crate::streaming` (Range
  fetches, boot progress, the statistics bridge, the memory budget); `crate::frame` (the render
  engine), `crate::overlay::lanes` (lane roles) and `crate::world::scene` (the anchor);
  `website-graphics-engine` (the quad instance layout), `wgpu` and the browser's image APIs.
- Used by:
  - `crate::streaming::host`, which loads the basemap at boot, switches the basemap view and
    uploads the hillshade;
  - `crate::frame`, whose engine keeps the `TexLane`s it draws, and
    `crate::spatial::los::terrain` and `crate::world::environment::vegetation`, which build
    `TexLane`s for the viewshed overlay and the forest density texture;
  - the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s tests in
    `apps/website/frontend/src/v2/apps/editor/tests/`, which parse Everon's index and read the
    loading code.
- Rules: `textures.rs` and `quadtree/` compile only for wasm32 with the `render` feature, and
  `streamer/` with `streaming`; the texture-layer roles are exactly 0 and 1, the numbers the
  basemap and hillshade callers pass.
