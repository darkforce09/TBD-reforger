# Left Dock Container (`ui/left/`)

## Purpose
Manages the collapsible, resizable left dock.

## Hosted Panels
- **Outliner (ORBAT Tree):** From `features/orbat`.
- **Layers & Visibility:** Map layer toggles from `canvas_engine/terrain`.
- **History / Undo Stack:** Action history list from `state`.

## Invariants
- Dock width is user-resizable with a horizontal splitter.
- Remembers collapsed/expanded state and active tab in user preferences.
