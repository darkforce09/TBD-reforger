# Bottom toolbelt

The chrome along the bottom of the
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s map: the floating mode toolbar
(Select, Ruler, line of sight), the full-width status bar with its read-outs and scale bar, and the
grid references along the map's top and left edges. The module root,
`apps/website/frontend/src/v2/apps/editor/ui/docks/toolbelt.rs`, declares these files, re-exports
the items below and mounts the toolbelt's tests.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/ui/docks/toolbelt/
├── bottom_toolbelt.rs     `BottomToolbelt`: the toolbar and status bar as one, mounted nowhere
├── grid_reference.rs      `EdgeLabel`, `edge_eastings`, `edge_northings`: grid lines to edge labels
├── map_furniture.rs       `ScaleBar` and `MapGridRefs`: the scale bar and the edge grid references
├── scale_math.rs          metres per pixel, the scale read-out's format, the scale bar's distance
└── toolbar_and_status.rs  `ModeToolbar` and `StatusBar`: the tool buttons and the read-outs
```

## How it works

The editor page mounts `ModeToolbar` centred above the bottom edge, `StatusBar` across the bottom,
36 px high (`STATUSBAR_H_PX`), and `MapGridRefs` over the map, each hidden with the rest of the
chrome. `ModeToolbar` sets the page's `tool_mode` to Select, Ruler or LoS, and a second press on
LoS switches its `LosMode` between ray and viewshed; the input layer reads both signals.

`StatusBar` shows, from left to right: X, Y and Z in metres, of the cursor ("CUR") or of the one
selected entity ("SEL"); "OBJ", the placed slots, and "SEL", the selection count; "SZ", the
estimated save payload; "SCL", metres per screen pixel; the ruler's total and last leg; the scale
bar; the debug HUD line while Ctrl+Alt+D shows it; and an "OPEN" button with no action. The scale
is `m_per_px`, 2 to the power of minus the zoom, which the render loop in
`apps/website/frontend/src/v2/apps/editor/bridge/viewport.rs` publishes after a frame whenever the
read-out changes; `pick_scale_bar` takes the largest 1, 2 or 5 times a power of ten that fits in
200 px. `MapGridRefs` projects the 1 km grid lines (`GRID_STEP_M`) through the live camera and
labels those inside the map pane with their three-digit references, keyed by position and text.

## Boundaries

- Depends on: `website_map_engine` (the ortho camera and `camera::grid_reference`, the ruler's
  `EditorTool` and the line-of-sight `LosMode`, and in the browser build `frozen_camera`,
  `read_attrs` and `camera_snapshot`); `crate::v2::apps::editor::shell::layout` for the insets
  and toggle classes, and `shell::mission_size::format_bytes`; `cn` and `MaterialIcon` from
  `crate::v2::core::ui`.
- Used by: `apps/website/frontend/src/v2/apps/editor/mission_editor.rs`, which mounts
  `ModeToolbar`, `StatusBar` and `MapGridRefs`;
  `apps/website/frontend/src/v2/apps/editor/bridge/viewport.rs` (`m_per_px`, `format_m_per_px`);
  `apps/website/frontend/src/v2/apps/editor/shell/layout.rs`, whose `DOCK_BOTTOM_PX` is
  `STATUSBAR_H_PX`; `apps/website/frontend/src/v2/apps/editor/shell/eden_chrome.rs`, which
  re-exports `BottomToolbelt`; the grid reference test in
  `apps/website/frontend/src/v2/apps/editor/shell/tests/exporter_grid_reference.rs`; the tests in
  `apps/website/frontend/src/v2/apps/editor/ui/docks/tests/toolbelt/`.
- Rules, held by that folder's tests: the toolbar holds no read-out
  (`mode_toolbar_holds_no_readouts_and_status_bar_holds_them` in `status_bar.rs`); the ruler
  read-out writes nothing to the document (`readout_is_display_only_no_doc_writes`); the scale bar
  and "SCL" agree (`the_bar_and_the_number_describe_the_same_scale`); the grid labels stay in the
  map pane (`grid_refs_are_clipped_to_the_map_pane_not_the_viewport`); `source.rs` there lists
  every file here for the source checks, so a new file joins that list.

## Related documentation

- [Mission Creator feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md)
  — the toolbelt's tools and read-outs, one entry each.
