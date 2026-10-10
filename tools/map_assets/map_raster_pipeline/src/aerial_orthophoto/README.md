# Aerial orthophoto stitching and seam checks

The Everon orthophoto lane of the `map` binary: stitching the game's 2,500 supertexture cells into
one 12,800 × 12,800 px north-up image, bridging the seams between cells, and the checks that measure
those seams and verify the stitched image. These files are the submodules
`tools/map_assets/map_raster_pipeline/src/aerial_orthophoto.rs` declares; it holds the
shared constants and seam metric types, and re-exports the entry points.

## Contents

```text
tools/map_assets/map_raster_pipeline/src/aerial_orthophoto/
├── supertexture_orthophoto_stitch.rs  `stitch-sap-ortho` from the paks, and `blend-sap-seams` on an existing image
├── supertexture_seam_analysis.rs      `analyze-sap-seams`, `verify-sap-ortho`, and the in-place seam bridge
└── supertexture_seam_metrics.rs       the scratch folder, the per-seam gradient metrics, and `verify-sap-seams`
```

## How it works

Every step works in `assets/scratch/everon/sap/`, which git ignores, and answers any terrain
other than `everon` with exit 1. The subcommands, the scratch folder and the output files keep the
lane's short name `sap` for the game's supertexture cells; the files here are named for the
supertexture work they hold.

```text
paks (ENFUSION_GAME_PATH) ─▶ stitch-sap-ortho ─▶ everon-sap-ortho.png + TBD_SatExport_meta.json
                                                   │
                  blend-sap-seams (bridge again) ◀─┤
                  verify-sap-seams               ◀─┤  seam gradients, steps, global contrast
                  analyze-sap-seams              ◀─┤  ─▶ documentation/tools/map_assets/decision_records/aerial_orthophoto/seam_analysis.json
                  verify-sap-ortho               ◀─┘  catalogue, metadata, size, orientation, tile 0/0/0
```

- `stitch_supertexture_orthophoto` opens the game's paks through `enfusion_pak::PakVfs`, decodes all 2,500
  Eden cells with `world_export_pipeline::enfusion_texture_decoder`, places cell `N = y*50 +
  x` with the south row at the image bottom, bridges the interior seams with `bridge_seams`, and
  writes the PNG and its metadata JSON. It refuses to write when any cell is missing or failed to
  decode.
- `blend_supertexture_seams_command_line` runs the same bridge over an existing PNG in place and records the repair in
  the metadata.
- The seam metrics in `supertexture_seam_metrics.rs` measure, for every cell boundary, the minimum gradient across a
  band, the interior gradient either side, the apron widths and the colour step across the seam;
  `summarize` turns them into fill, step and anchor failures against the constants in the parent
  file (`FILL_GRADIENT_FLOOR`, `STEP_DELTA_CAP`, `MINIMUM_INTERIOR_DETAIL` and the others).
- `verify_supertexture_seams` fails on a wrong size, on seams that fail the fill, step or anchor check, on
  interior control lines that read flat, and on a flat image; `analyze_supertexture_seams` writes the same
  measurements with a diagnosis to the committed analysis artifact.
- `verify_supertexture_orthophoto` checks the cell catalogue that `world sap-catalog` writes (2,500 cells), the
  metadata's size, scale and bounds, the image size and contrast, that the image's land mask matches
  the terrain elevation model's land mask north-up (`ORIENT_MAX`), and that the satellite pyramid's
  `tiles/satellite/0/0/0.webp` exists.

## Boundaries

- Depends on: the parent's constants and types and `super::image_operations`;
  `enfusion_pak::PakVfs`; `world_export_pipeline::enfusion_texture_decoder` for the
  cell decode; the `repository_layout` crate for the scratch and terrain folders and
  `crate::decision_record_locations` for the artifact folders;
  `repository_root::find_repository_root` for the checkout root.
- Used by: `tools/map_assets/map_raster_pipeline/src/command_line.rs` (the `stitch-sap-ortho`,
  `blend-sap-seams`, `verify-sap-seams`, `analyze-sap-seams` and `verify-sap-ortho` subcommands);
  `verify-sap-ortho` is a step of `cargo xtask ci map-water-everon`.
- Rules: no write goes out from an incomplete stitch (`refuse_empty_write` in
  `tools/map_assets/map_raster_pipeline/src/lib.rs`, pinned by
  `refuse_empty_write_reds_on_empty`); the gate thresholds are named constants in
  `aerial_orthophoto.rs`, and a change to one changes what every verifier accepts.
