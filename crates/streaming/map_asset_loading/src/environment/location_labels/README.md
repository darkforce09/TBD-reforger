# Map labels: the browser label loader

The cartographic labels of a terrain as the browser loads them: town and place names, road names placed
along their roads, and spot heights on the hilltops. The label rules (peak finding, the label
files and archive, road name placement, declutter and glyph packing) live in the
[`place_names`](/crates/world_objects/place_names/README.md) crate; this folder keeps the browser
loader that fetches a terrain's label sources and uploads the label lanes.

## Contents

```text
crates/streaming/map_asset_loading/src/environment/location_labels/
├── loader.rs  `LabelHost`: loads the label sources in the browser and uploads the lanes
└── mod.rs     the module tree: `loader`
```

## How it works

```text
manifest.json "labels" block
  ├─ names the archive ─► locations/map_labels.rkyv ─ map_labels_from_bytes ─► towns,
  │                                                            placed road name candidates
  └─ names none ────────► locations.json, road-names.json ─ parse_* ─────────► towns,
                                                                     curated road list
DEM raster ─ find_peaks ─► spot heights

LabelHost::push(zoom, layer toggles)
  ├─ road names ─► placed and decluttered by place_names
  ├─ towns, spot heights ─► decluttered and packed by place_names::label_packing
  └─► the town, road and height label lanes of the engine
```

`LabelHost::init` reads the terrain's `manifest.json` (`assets/terrains/everon/` for Everon,
served under `/map-assets/`): when the `labels` block names an archive of encoding
`rkyv-map-labels-v1`, the towns and the baked road name candidates come from that archive, and
stay empty if it fails to load; when the manifest names none, they come from `locations.json` and
`road-names.json`. Spot heights come from the elevation model itself through `find_peaks`. `push`
recomputes the lanes only when the half-zoom band or a layer toggle changes.

## Public surface

- `loader::LabelHost`: `init` and `push`, owned by the map host's state.

## Boundaries

- Depends on: `place_names`, `label_layout::importance`, `world_chunks::terrain_manifest` (the
  labels block), `terrain_elevation::manifest`, `road_network::network` (the road segments),
  `map_streaming_model` (boot progress, layer preferences, the asset sink the labels upload
  through), `crate::browser_asset_sink` (`BrowserAssetSinkHandle`), `browser_platform::fetch` and
  `render_primitives::text`. The loader owns `WORLD_LABEL_FILES`, the label files the map
  host's boot declares against the world segment.
- Used by: the map host of `map_streaming_host`, whose state owns the `LabelHost`.
- Rules: `loader.rs` compiles only for wasm32.
