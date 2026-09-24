# Map symbology

The map engine's cartographic vocabulary: the bespoke unit-role and vehicle glyphs with their side
tints, the glyph atlases, the icon instances and [slot](/documentation_v2/glossary.md#slot) GPU
bridge, map labels, squad tether links,
briefing marker glyphs and captions, and the label text packing. It holds what the
[Mission Creator](/documentation_v2/glossary.md#mission-creator) and the streamed world draw, so the
graphics engine only ever sees cells, instances and uniforms.

## Contents

```text
apps/website/map-engine/src/overlay/symbology/
├── atlas/            the slot and symbology glyph atlas raster, and its browser upload
├── instances/        icon instances for slots, vehicles, comments, clusters; the GPU bridge
├── labels/           label declutter, town-label bands, glyph sizing and the icon instance packer
├── links/            the squad leader-to-member tether hairlines, at rest and mid-drag
├── markers.rs        `MarkerGlyph`: icon aliases to eleven glyphs, their atlas, captions
├── mod.rs            the module tree; `instances`, `markers` and the text files need `streaming`
├── roles/            the role, vehicle and side classification tables
├── tests/            unit tests for the marker vocabulary, its atlas, captions and the text atlas
├── text_metrics.rs   the graphics engine's text atlas, font and metrics; height label spacing
└── text_packing.rs   label, town, height and road text packed into glyph instances
```

## How it works

```text
document rows / streamed world
  │ roles/       role, alias, side ──► glyph class, silhouette, tint
  │ labels/      which labels draw at this zoom; glyph size and yaw
  │ links/       squad tethers
  │ markers.rs   marker alias ──► MarkerGlyph cell; caption glyphs
  ▼
instances/, text_packing.rs   20-byte icon and text instances
  │ atlas/       cells the instances index
  ▼
RenderEngine lanes (instances/ bridge, frame/)  ──► website-graphics-engine draws
```

The symbology is bespoke: five unit roles, three vehicle kinds and three side tints, with no
MIL-STD-2525 or APP-6 frames. `markers.rs` maps every marker `icon` alias of the mission schema to
one of `MARKER_GLYPH_COUNT` (11) glyphs, folding case and separators and falling back to the disc,
the same downgrade the game [mod](/documentation_v2/glossary.md#mod) makes; its atlas shares cells 0
and 1 (ring, disc) with the slot
atlas. Captions and place names go through one text pipeline: `text_packing.rs` lays glyphs of the
graphics engine's baked ASCII atlas beside their anchors, and hands `LabelSpec` labels to the
renderer without their importance, which only decides what reaches it. Nothing here depends on the
UI framework or editor state; browser I/O (the atlas upload, the lane binds) compiles only on
`wasm32` with the `render` feature.

## Public surface

- `roles::classify` (`side_rgba`, `side_tints_rgba_bytes`, the role and alias tables), for
  `crate::editing::lanes` and the Mission Creator's document host and canvas mount.
- `links::squad_links`, for `crate::editing::picking` and the Mission Creator's document host and
  select tool.
- `markers` (`MarkerGlyph`, `MARKER_GLYPH_COUNT`, `marker_glyph_for_alias`), for the Mission
  Creator's marker dock and canvas mount.
- `instances`: the packers, `drag::pack_vehicle_drag_preview` and the `RenderEngine` binds, for
  `crate::frame`, `crate::camera::viewport`, the streaming world loader and the Mission Creator.
- `labels`, `text_metrics` and `text_packing`, for `crate::streaming`, the location loaders in
  `crate::world::environment::locations`, `crate::frame` and `crate::diagnostics`, and the
  town-label verification in `tools_v2/developer-tools/src/map_verification/labels/`.

## Boundaries

- Depends on: `crate::frame` (`RenderEngine` and its lanes), `crate::overlay::lanes` and
  `crate::overlay::lod`, `crate::world::scene` (the anchor and Everon bounds),
  `website_graphics_engine` (`text` atlas, font, metrics, layout and pack; `layout`), `serde` and
  `wasm_bindgen`.
- Used by: `crate::editing` (lanes, picking), `crate::frame`, `crate::camera`, `crate::streaming`,
  `crate::world::environment::locations`, `crate::diagnostics`; the Mission Creator in
  `apps/website/frontend/src/v2/apps/editor/`; `tools_v2/developer-tools/`.
- Rules: every marker alias of the schema maps to a glyph (`every_schema_alias_maps` in
  `tests/markers_tests.rs`); marker atlas cells 0 and 1 equal the slot atlas
  (`marker_atlas_cells_0_and_1_match_slot_atlas`); the committed label data has no glyph without
  an atlas cell (`g3_committed_label_data_no_tofu` in `tests/text_layout.rs`); the side tints are
  pinned by `cargo xtask verify editor-orbat-coherency`; no name in the graphics engine may say
  symbology, which is why this vocabulary lives here (rule 2 of `cargo xtask verify engine-layers`).
