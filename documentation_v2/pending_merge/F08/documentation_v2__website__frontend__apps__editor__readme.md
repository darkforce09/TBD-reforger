# Scenario Creator CAD Workspace (`website/frontend/apps/editor/`)

The Scenario Creator is a full-featured 2D/3D CAD editing workspace for designing Arma Reforger operations, placing entities, defining mission objectives, and configuring tactical ORBATs.

## Subsystems
- `ui/`: Docking layout shell, outliner tree, asset catalog palette, properties inspector, modal dialogs (<500 LOC per file).
- `input/`: DOM Pointer/Keyboard events translated into headless Map Engine editing commands.
- `bridge/`: HTML5 Canvas mount, Device Pixel Ratio (DPR) scaling, and rAF heartbeat connector.
- `shell/`: Tab management, local autosave persistence, and workspace session preferences.
- `arsenal/`: Dedicated equipment configuration and Loadout Forge interface.
- `visual_reference/`: Colocated visual design blueprints and historical prototypes.

## Code Mapping
- Source: `apps/website/frontend/src/v2/apps/editor/`
