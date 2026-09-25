# Aerial orthophoto stitching and seam checks

The Everon orthophoto lane of the `map` binary: stitching the game's 2,500 supertexture cells into
one 12,800 × 12,800 px north-up image, bridging the seams between cells, and the checks that measure
those seams and verify the stitched image. These files are the submodules
`tools_v2/developer-tools/src/map_raster_pipeline/aerial_orthophoto.rs` declares; it holds the
shared constants and seam metric types, and re-exports the entry points.

## Contents

```text
tools_v2/developer-tools/src/map_raster_pipeline/aerial_orthophoto/
├── analyze_sap_seams.rs  `analyze-sap-seams`, `verify-sap-ortho`, and the in-place seam bridge
├── sap_dir.rs            the scratch folder, the per-seam gradient metrics, and `verify-sap-seams`
└── stitch_sap_ortho.rs   `stitch-sap-ortho` from the paks, and `blend-sap-seams` on an existing image
```

## How it works

Every step works in `assets_v2/scratch/everon/sap/`, which git ignores, and answers any terrain
other than `everon` with exit 1.

```text
paks (ENFUSION_GAME_PATH) ─▶ stitch-sap-ortho ─▶ everon-sap-ortho.png + TBD_SatExport_meta.json
                                                   │
                  blend-sap-seams (bridge again) ◀─┤
                  verify-sap-seams               ◀─┤  seam gradients, steps, global contrast
                  analyze-sap-seams              ◀─┤  ─▶ .ai/artifacts/aerial_orthophoto/seam_analysis.json
                  verify-sap-ortho               ◀─┘  catalogue, metadata, size, orientation, tile 0/0/0
```

- `stitch_sap_ortho` opens the game's paks through `crate::enfusion_pak::PakVfs`, decodes all 2,500
  Eden cells with `crate::world_export_pipeline::enfusion_texture_decoder`, places cell `N = y*50 +
  x` with the south row at the image bottom, bridges the interior seams with `bridge_seams`, and
  writes the PNG and its metadata JSON. It refuses to write when any cell is missing or failed to
  decode.
- `blend_sap_seams_cli` runs the same bridge over an existing PNG in place and records the repair in
  the metadata.
- The seam metrics in `sap_dir.rs` measure, for every cell boundary, the minimum gradient across a
  band, the interior gradient either side, the apron widths and the colour step across the seam;
  `summarize` turns them into fill, step and anchor failures against the constants in the parent
  file (`FILL_FLOOR`, `STEP_CAP`, `DETAIL_MIN` and the others).
- `verify_sap_seams` fails on a wrong size, on seams that fail the fill, step or anchor check, on
  interior control lines that read flat, and on a flat image; `analyze_sap_seams` writes the same
  measurements with a diagnosis to the committed analysis artifact.
- `verify_sap_ortho` checks the cell catalogue that `world sap-catalog` writes (2,500 cells), the
  metadata's size, scale and bounds, the image size and contrast, that the image's land mask matches
  the terrain elevation model's land mask north-up (`ORIENT_MAX`), and that the satellite pyramid's
  `tiles/satellite/0/0/0.webp` exists.

## Boundaries

- Depends on: the parent's constants and types and `super::image_operations`;
  `crate::enfusion_pak::PakVfs`; `crate::world_export_pipeline::enfusion_texture_decoder` for the
  cell decode; `crate::repository_layout` for the scratch, terrain and artifact folders;
  `crate::browser_testing::server::repo_root` for the checkout root.
- Used by: `tools_v2/developer-tools/src/map_raster_pipeline/cli.rs` (the `stitch-sap-ortho`,
  `blend-sap-seams`, `verify-sap-seams`, `analyze-sap-seams` and `verify-sap-ortho` subcommands);
  `verify-sap-ortho` is a step of `cargo xtask ci map-water-everon`.
- Rules: no write goes out from an incomplete stitch (`refuse_empty_write` in
  `tools_v2/developer-tools/src/map_raster_pipeline/mod.rs`, pinned by
  `refuse_empty_write_reds_on_empty`); the gate thresholds are named constants in
  `aerial_orthophoto.rs`, and a change to one changes what every verifier accepts.
