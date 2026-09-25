# Inland water classification and water tint

The image side of Everon's water: finding inland lakes, ponds and rivers in the stitched orthophoto,
and tinting the ocean and those inland bodies into it before the satellite image is built. These
files are the submodules `tools_v2/developer-tools/src/map_raster_pipeline/inland_water.rs`
declares; it holds the tint colours and every classifier threshold, and re-exports the two entry
points. The water binaries the map engine loads come from the sibling `inland_water_archive/`
instead.

## Contents

```text
tools_v2/developer-tools/src/map_raster_pipeline/inland_water/
├── analyze_water_sources.rs  `analyze-water`: the classifier run, its mask, preview and decision record
├── sap_dir.rs                `composite-water`, the scratch folder, the elevation reader and dilation
└── source_components.rs      the per-pixel classes and the acceptance of connected water components
```

## How it works

Both steps are Everon-only and run in `assets_v2/scratch/everon/sap/`, which git ignores; `cargo
xtask ci map-water-everon` runs them in this order after restoring the pre-water image.

```text
everon-sap-ortho.png + dem/everon-dem-16bit.png + roads from the .topo file
  ─▶ analyze_water_sources ─▶ water-inland-mask.png, water-spike-preview.png
                           ─▶ .ai/artifacts/inland_water/source_spike.json
everon-sap-ortho.png + water-inland-mask.png + the elevation model
  ─▶ composite_water_ortho ─▶ everon-sap-ortho.png tinted in place
                           ─▶ everon-sap-ortho.pre-water.png (first run), waterComposite in the metadata
```

- `analyze_water_sources` reads the 6,400 px elevation model and the orthophoto at 3,200 px
  (`DETECT_DIM`), builds sea, flat, slope, valley and road-corridor planes, and hands them to
  `classify_source_components` in `source_components.rs`, which marks grey and wet pixels, groups
  them into connected components and accepts each by its area, colour, slope, flatness, road overlap
  and shape, with a narrower rule for river ribbons. It writes the accepted bodies as a full-size
  mask, a preview, and a decision record that compares them with the bodies of the committed
  `.ai/artifacts/inland_water/refine_spike.json`.
- `composite_water_ortho` paints the ocean with a depth ramp from the elevation model and the inland
  mask in `INLAND_COLOR`, feathered inward, then records a `waterComposite` block in
  `TBD_SatExport_meta.json`. It copies the untinted image to `everon-sap-ortho.pre-water.png` once,
  and refuses to run when the metadata already holds the block, so the tint is never applied twice.

## Boundaries

- Depends on: `super::image_operations`; `crate::enfusion_pak::PakVfs` and
  `crate::world_export_pipeline::topo` for the road corridor;
  `crate::world_export_pipeline::json_number_formatting` for the numbers the record prints;
  `crate::repository_layout` for the scratch, terrain and artifact folders;
  `crate::browser_testing::server::repo_root` for the checkout root.
- Used by: `tools_v2/developer-tools/src/map_raster_pipeline/cli.rs` (`analyze-water`,
  `composite-water`); the `map-water-everon` task in
  `tools_v2/xtask/src/commands/ci/task_definitions.rs`; the cartographic render in
  `tools_v2/developer-tools/src/map_raster_pipeline/cartographic_rendering/` reads the mask.
- Rules: the composite is applied once per stitched image, held by the `waterComposite` check and
  undone only by restoring the `.pre-water.png` copy and running `map reset-water-meta`; the
  analysis fails on an elevation model that is not 6,400 px square.
