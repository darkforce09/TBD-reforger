# Right Dock Container (`ui/right/`)

## Purpose
Manages the collapsible, resizable right dock.

## Hosted Panels
- **Asset Palette:** Entity browser from `features/placement`.
- **Attributes Inspector:** Quick property editor from `features/manipulation`.
- **Compositions:** Prefab template browser from `features/placement`.

## Invariants
- Dock width is user-resizable with a horizontal splitter.
- Remembers width and active tab across sessions.
