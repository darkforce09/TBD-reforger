# Spatial Workspace Shell (`ui/`)

This directory owns the physical layout of the Eden workspace editor.

---

## Responsibilities & Boundaries

- **OWNS:** Screen divisions, CSS Grid/Flexbox containers, splitters, dock resizing handles, tab button strips, and component slot placement.
- **FORBIDDEN:** Mathematical calculations, CRDT document mutations, validation checks, or direct business logic.

```text
ui/
├── top/       <-- Row 1: Primary Menu Bar | Row 2: Secondary Toolbelt Toolbar
├── left/      <-- Left dock container & tab switcher
├── right/     <-- Right dock container & tab switcher
├── bottom/    <-- Bottom status bar (readouts, zoom, FPS)
├── canvas/    <-- Center viewport frame & overlay mount points
└── modals/    <-- Full-screen modal host & backdrop management
```

## How It Interacts With Features
The UI shell simply imports and places widgets provided by `features/`. For example, `ui/top/primary_bar.rs` mounts `<MenuBar />` and `<SaveStatusIndicator />`, but knows nothing about how saving works or what menu items exist.
