# Bottom toolbelt

The chrome along the bottom of the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s map: the floating mode toolbar
(Select, Ruler, line of sight), the full-width status bar with its read-outs and scale bar, and the
grid references along the map's top and left edges. The module root,
`crates/frontend/workspaces/mission_creator_workspace/src/ui/docks/toolbelt.rs`, declares these files, re-exports
the items below and mounts the toolbelt's tests.

## Contents

```text
crates/frontend/workspaces/mission_creator_workspace/src/ui/docks/toolbelt/
├── grid_reference.rs      `EdgeLabel`, `edge_eastings`, `edge_northings`: grid lines to edge labels
├── map_furniture.rs       `ScaleBar` and `MapGridRefs`: the scale bar and the edge grid references
└── toolbar_and_status.rs  `ModeToolbar` and `StatusBar`: the tool buttons and the read-outs
```

## How it works

The editor page mounts `ModeToolbar` centred above the bottom edge, `StatusBar` across the bottom,
36 px high (the state layer's `layout::STATUSBAR_H_PX`), and `MapGridRefs` over the map, each hidden with the rest of the
chrome. `ModeToolbar` sets the page's `tool_mode` to Select, Ruler or LoS, and a second press on
LoS switches its `LosMode` between ray and viewshed; the input layer reads both signals.

`StatusBar` shows, from left to right: X, Y and Z in metres, of the cursor ("CUR") or of the one
selected entity ("SEL"); "OBJ", the placed [slots](/documentation/glossary/n_to_z.md#slot), and "SEL",
the selection count; "SZ", the estimated save payload; "SCL", metres per screen pixel; the ruler's
total and last leg; the scale bar; the debug HUD line while Ctrl+Alt+D shows it; and an "OPEN"
button with no action. The scale arithmetic is the state layer's `scale_math`
(`crates/frontend/workspaces/mission_creator_state/src/scale_math.rs`): the scale is `m_per_px`, 2 to the
power of minus the zoom, which the render
loop in `crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/viewport.rs` publishes after a frame
whenever the read-out changes; `pick_scale_bar` takes the largest 1, 2 or 5 times a power of ten
that fits in 200 px. `MapGridRefs` projects the 1 km grid lines (`GRID_STEP_M`) through the live
camera and labels those inside the map pane with their three-digit references, keyed by position and
text.

## Boundaries

- Depends on: `camera_math` (the ortho camera); `map_coordinates::grid_reference`; `map_editing_tools` (the
  ruler's `EditorTool` and the line-of-sight `LosMode`, and in the browser build `frozen_camera`
  and `read_attrs`); `map_streaming_host::camera_snapshot` in the browser build; the state layer's `layout` for the
  insets and toggle classes and `scale_math` for the scale; `format_bytes` from `frontend_ui::byte_formatting`; `cn` and
  `MaterialIcon` from `frontend_ui`.
- Used by: `crates/frontend/workspaces/mission_creator_workspace/src/mission_editor.rs`, which mounts
  `ModeToolbar`, `StatusBar` and `MapGridRefs`;
  the tests in `crates/frontend/workspaces/mission_creator_workspace/src/ui/docks/tests/toolbelt/`,
  among them the grid reference check in `live_grid_labels.rs`.
- Rules: the toolbar holds no read-out; the ruler read-out writes nothing to the document; the
  scale bar and "SCL" agree (`the_bar_and_the_number_describe_the_same_scale`); the grid labels
  stay in the map pane (`grid_refs_are_clipped_to_the_map_pane_not_the_viewport`).

## Related documentation

- [Mission Creator feature inventory: bottom toolbelt](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/bottom_toolbelt.md) — each tool and read-out, with its status.
