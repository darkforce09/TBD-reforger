# 1. Mission Creator (`src/v2/editor`)

The desktop-class CAD scenario authoring application (Eden 2D Editor).

---

## Architecture
- **Consumes:** Mounts `<MapCanvas />` from `src/v2/map_engine`.
- **`ui/`**: OWNS the complete Eden workspace shell (Two-row 48px top strip, Left ORBAT dock, Right Asset Palette dock, bottom status bar, modal dialog host).
- **`features/`**: Domain authoring modules (ORBAT squad tree, Arsenal loadout builder, Entity placement, Zones, Tasks, Weather timeline).
- **`state/`**: Yrs CRDT document host (`MissionDocCore`), transaction boundary, history undo/redo, autosave persistence.
- **`tests/`**: Dedicated test suite.
