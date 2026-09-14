# Canvas Viewport Container (`ui/canvas/`)

## Purpose
Frames the interactive 2D map and mounts floating canvas HUD elements.

## Responsibilities
- Mounts the HTML `<canvas>` element for the `canvas_engine`.
- Observes container resize events and notifies the render engine.
- Mounts the floating overlay layer (`overlays.rs`) for tool gizmos, ruler lines, and sticky notes.
