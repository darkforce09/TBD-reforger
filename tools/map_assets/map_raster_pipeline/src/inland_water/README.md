# Inland water classification and water tint

The image side of Everon's water: finding inland lakes, ponds and rivers in the stitched orthophoto,
and tinting the ocean and those inland bodies into it before the satellite image is built. These
files are the submodules `tools/map_assets/map_raster_pipeline/src/inland_water.rs`
declares; it holds the tint colours and every classifier threshold, and re-exports the two entry
points. The water binaries the map engine loads come from the sibling `inland_water_archive/`
instead.

## Contents

```text
tools/map_assets/map_raster_pipeline/src/inland_water/
├── analyze_water_sources.rs  `analyze-water`: the classifier run, its mask, preview and decision record
├── sap_dir.rs                `composite-water`, the scratch folder, the elevation reader and dilation
└── source_components.rs      the per-pixel classes and the acceptance of connected water components
```

## How it works

Both steps are Everon-only and run in `assets/scratch/everon/sap/`, which git ignores; `cargo
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

- Depends on: `super::image_operations`; `enfusion_pak::PakVfs` and
  `world_export_pipeline::topo` for the road corridor;
  `world_export_pipeline::json_number_formatting` for the numbers the record prints;
  the `repository_layout` crate for the scratch and terrain folders and
  `crate::decision_record_locations` for the artifact folders;
  `repository_layout::find_repository_root` for the checkout root.
- Used by: `tools/map_assets/map_raster_pipeline/src/command_line.rs` (`analyze-water`,
  `composite-water`); the `map-water-everon` task in
  `tools/commands/ci_task_catalog/src/task_definitions.rs`; the cartographic render in
  `tools/map_assets/map_raster_pipeline/src/cartographic_rendering/` reads the mask.
- Rules: the composite is applied once per stitched image, held by the `waterComposite` check and
  undone only by restoring the `.pre-water.png` copy and running `map reset-water-meta`; the
  analysis fails on an elevation model that is not 6,400 px square.
