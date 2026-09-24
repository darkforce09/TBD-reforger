# Editor-authored map lanes

The overlay lanes the [Mission Creator](/documentation_v2/glossary.md#mission-creator) draws from
the [mission](/documentation_v2/glossary.md#mission) document's own rows (connections, comments and
briefing markers) and the hit tests a click on them resolves through. Each lane is a pure function
of the document's JSON and world coordinates.

## Contents

```text
apps/website/map-engine/src/editing/lanes/
├── comments.rs     `CommentPoint`: comment glyph lane, id column, drag preview and pick
├── connections.rs  `ConnSegment`: connection hairlines, vertices and the nearest-edge pick
├── markers.rs      `marker_lane_fields`: briefing marker rows parsed into four row-aligned columns
└── mod.rs          the module tree
```

## How it works

One read of the document feeds both what a lane uploads and what a click can find, so a glyph is
never drawn where it cannot be picked, or picked where nothing is drawn. Coordinates are world
metres on the map plane; a comment's and a marker's stored `{x, z}` is read as easting and northing.

- **Connections.** `connection_segments` reduces `MissionDocCore::connection_rows_json` and a map of
  entity positions to segments, skipping a self-link or an edge whose endpoint has no position
  rather than drawing it to the origin. `connection_lane_verts` packs six floats per vertex
  (`[x, y, r, g, b, a]`), tinting the one selected edge amber (`CONN_LINE_SELECTED_RGBA`).
  `pick_connection` measures point-to-segment distance and the nearest edge within the tolerance
  wins; the caller converts `CONN_PICK_PX` (6 px) to metres through the press camera.
- **Comments.** `comment_points` reads `commentsById` in full, sorted by id, so instance order does
  not depend on the JSON map's order across undo, redo or a restore. `comment_lane_xy`,
  `comment_lane_ids` and `comment_drag_lane_xy` are packed from that list, and
  `dragged_comment_points` serves both the drag preview and the move commit. `pick_comment` takes
  the nearest glyph within the tolerance; `COMMENT_PICK_PX` (4 px) restates the
  [slot](/documentation_v2/glossary.md#slot) pick radius
  `MissionDocCore::PICK_RADIUS_PX`.
- **Markers.** `marker_lane_fields` parses `briefing_marker_rows_json` once into positions, side
  tints (from `crate::overlay::symbology::roles::classify::side_rgba`), icon aliases and captions.
  The alias travels verbatim; the renderer maps it to a glyph. Malformed input yields four empty
  columns.

## Boundaries

- Depends on: `crate::overlay::symbology::roles::classify::side_rgba` for the marker tints, and
  `serde_json`; the rows come from `crate::data::store::MissionDocCore`'s JSON views, passed in by
  the caller.
- Used by: the Mission Creator's editor page
  (`apps/website/frontend/src/v2/apps/editor/mission_editor.rs`, which re-exports the lanes to its
  document helpers, render lanes and pointer gestures) and its hover bridge
  (`apps/website/frontend/src/v2/apps/editor/bridge/pointer_hover.rs`), plus the editor tests
  under `apps/website/frontend/src/v2/apps/editor/tests/`.
- Rules: `COMMENT_PICK_PX` equals the slot pick radius, pinned by
  `apps/website/frontend/src/v2/apps/editor/tests/t784_comment_glyph.rs`; nothing here caches, so
  a lane is rebuilt from the document on every read.
