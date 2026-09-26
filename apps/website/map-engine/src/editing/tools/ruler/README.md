# Ruler tool

The headless half of the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s ruler:
a session-local polyline, what each leg measures (distance, bearing, rise and slope), its screen
projection, and the tool mode that decides what a left click means. A measurement is never
[mission](/documentation_v2/glossary/g_to_m.md#mission) content.

## Contents

```text
apps/website/map-engine/src/editing/tools/ruler/
├── chain.rs          `RulerChain`: append, end, de-duplicate the tail, escalating escape, readout
├── host_registry.rs  `RULER_CHAIN`, the host-installed chain cell the drawer reads
├── leg.rs            `RulerPoint` and `Leg`: distance, bearing, rise, slope and their formatters
├── mod.rs            the module tree; re-exports the tool's vocabulary flat
├── projection.rs     legs and vertices projected to screen space, keyed by world position
├── tests/            unit tests for each file, and the no-document-write guard
└── tool_mode.rs      `EditorTool` (select, ruler, line of sight) and `should_begin_ruler`
```

## How it works

```text
EditorTool::Ruler or LoS, button 0 ──► should_begin_ruler = true ──► point-capture gesture
                                                                          │ release
                                                                          ▼
RulerChain: press(x, y, z) appends, drawing = true
            double_click ends the chain and keeps it (dedup_tail drops the coincident vertex)
            escape: while drawing, stop drawing (a lone vertex clears); placed, clear the points
```

`EditorTool` is the one enum the toolbar and the pointer handlers share: `Ruler` and `LoS` both
capture points (`captures_points`), and `should_begin_ruler` opens the shared point-capture gesture
for button 0 only, so the middle and right buttons keep pan and the context menu. The host routes
a captured click to the chain or to the line-of-sight capture by the live tool.

A chain stores committed vertices only; the rubber-band leg to the cursor is the overlay's. `legs`
builds every `Leg` once, so the on-map labels and the status readout (`status_readout`,
`Σ 1.24 km · last 412 m · 073.2° · +8 m (2%)`) read the same figures. Bearing is degrees clockwise
from grid north (world +Y), printed `NNN.N°`; rise and slope are `None` when either vertex has no
DEM height, and the label then drops the clause. `project_legs` and `project_vertices` take an
injected world-to-pixel projector and key each node by its world endpoints quantised to 0.1 m
(`world_key`), so two legs with the same label never share a node.

## Boundaries

- Depends on: nothing outside the folder; the host injects the projector and the click-time DEM
  height.
- Used by:
  - the Mission Creator's ruler overlay
    (`apps/website/frontend/src/v2/apps/editor/input/tools/ruler_tool.rs`), which installs the
    chain into `RULER_CHAIN`, and the canvas mount
    (`apps/website/frontend/src/v2/apps/editor/mission_editor/canvas_mount.rs`), which creates it;
  - the pointer gestures (`apps/website/frontend/src/v2/apps/editor/input/pointer_gestures.rs` and
    `apps/website/frontend/src/v2/apps/editor/input/pointer_gestures/pointer_down.rs`), the editor
    page (`apps/website/frontend/src/v2/apps/editor/mission_editor.rs`) and the toolbelt
    (`apps/website/frontend/src/v2/apps/editor/ui/docks/toolbelt/`), which read `EditorTool`.
- Rules:
  - nothing here names the document or its mutators (`the_ruler_never_writes_the_document` in
    `tests/session_local.rs`);
  - only button 0 under a point-capture tool opens the gesture
    (`should_begin_ruler_button_and_tool_gating` in `tests/tool_mode.rs`);
  - a drawn node is keyed by world coordinates, not label text
    (`label_keys_are_world_coords_not_text` in `tests/projection.rs`).

## Related documentation

- [Mission Creator feature inventory: bottom toolbelt](/documentation_v2/website/frontend/apps/editor/feature_inventory/bottom_toolbelt.md) — the Ruler as the mission maker uses it.
