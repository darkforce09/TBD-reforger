# Frame upload belts

The render engine's upload belts: each takes geometry its caller has already computed, uploads it
into GPU buffers and puts the lane's `DrawBatch` into the engine's batch list, with the pipeline and
bind group that lane draws with.

## Contents

```text
apps/website/map-engine/src/frame/upload/
├── hairlines.rs  hairline segment lanes by role id, and the mission connection lines
├── mod.rs        the module tree
├── polygons.rs   indexed polygon meshes and triangle strips for the vector lanes, by role id
├── selection.rs  the selection marquee: a translucent fill and its outline
└── text.rs       the text atlas, the height, town and road label lanes, and the texture limits
```

## How it works

Each belt is a `#[wasm_bindgen]` method on `crate::frame::engine::RenderEngine` that takes flat
arrays in world metres:

| Method | Input | Lane, pipeline |
|---|---|---|
| `upload_polygon_mesh` | positions, RGBA colours, indices | the role's lane, `PIPE_POLYGON` |
| `upload_strip_tris` | six floats per vertex | the role's lane, `PIPE_POLYGON` |
| `upload_hairline_segments` | six floats per vertex | the role's lane, `PIPE_LINE` |
| `connections_bind` | six floats per vertex | `MissionConnections`, `PIPE_LINE` |
| `upload_marquee` | the rectangle's corners | `Marquee`, `PIPE_POLYGON`; `MarqueeOutline`, `PIPE_LINE` |
| `upload_text_labels`, `upload_town_labels`, `upload_road_labels` | 20-byte glyph instances | `WorldLabels`, `WorldTownLabels`, `WorldRoadLabels`, `PIPE_TEXT` |

A role id is one of `crate::overlay::lanes::role_id`; a belt ignores an id that names no lane.
Coordinates cross to the GPU relative to `crate::world::scene::ANCHOR`, so f32 keeps them precise:
the graphics engine's `draw::polygons` and `draw::lines` shift meshes and lines, and
`convert_icon_world_to_anchor` shifts glyph instances. The belt then calls `upsert_lane`, which
replaces the lane's batch in lane order and marks the frame damaged. Malformed input (an empty
array, or a length that is not a whole number of vertices or instances) removes the lane instead.
The vector belts pass on their item count, which `stats()` keeps for the sea, land-cover, contour,
road and forest lanes.

A label belt with `visible` false removes its lane. With an empty upload and `visible` true, the
town label belt removes its lane, while the height and road label belts leave the lane they already
have. Every label belt first calls `ensure_text_atlas`, which bakes the ASCII atlas once from
`crate::overlay::symbology::text_metrics::atlas`. `upload_marquee` removes both marquee lanes when
hidden or empty, and otherwise draws the fill in RGB (173, 198, 255) at alpha 40 and the outline at
alpha 200. `max_texture_dimension_2d` and `adapter_max_texture_dimension_2d` report the device's
and the adapter's largest 2D texture.

## Boundaries

- Depends on: `crate::frame` (the engine, `bindings`, the packet types and the text atlas type),
  `crate::overlay::lanes` (lane roles and role ids), `crate::overlay::symbology` (the baked text
  atlas and the glyph shift), `crate::world::scene::ANCHOR`, and `website-graphics-engine`
  (`draw::polygons`, `draw::lines`, `draw::geometry::LineVertex` and `layout::pack`).
- Used by:
  - inside the crate: `crate::streaming::loaders::world_loader` (terrain strips and polygons),
    `crate::world::terrain::relief` and `crate::world::environment::vegetation` (hairlines),
    `crate::world::environment::locations` (the three label lanes),
    `crate::world::terrain::satellite` (the texture limits), `crate::overlay::symbology` (the text
    atlas), and `crate::frame` and `crate::diagnostics::readback` (`TextAtlasGpu` and
    `text_uniform_bytes`);
  - the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s input handlers, canvas
    mount and document host under `apps/website/frontend/src/v2/apps/editor/` (the marquee, the
    connection and squad lines), and the debug benches under
    `apps/website/frontend/src/v2/apps/debug/` (building interiors).
- Rules: every lane change goes through `upsert_lane` or `remove_lane`, which mark the frame damaged
  (`every_lane_mutation_marks_the_frame_damaged` in
  `apps/website/map-engine/src/frame/tests/damage_discipline.rs`); `connections_bind` uploads
  its lane and never touches the pick bridge of the [slot](/documentation_v2/glossary/n_to_z.md#slot)
  icons (`connections_bind_body_uploads_its_lane_and_skips_the_pick_bridge`), and the overlay's
  draw-order suites in `apps/website/map-engine/src/overlay/tests/tests/` read these files by path,
  so a moved or renamed file breaks them.
