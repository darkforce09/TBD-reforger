# Frontend Architecture Hub (`src/v2`)

The domain-driven architecture for the TBD Reforger platform frontend. Outside this directory the
crate holds only `main.rs`, `router.rs` and `app_routes.rs`.

---

## High-Level Architecture Overview

```text
src/v2/
├── core/                              <-- SHARED FOUNDATIONS (API, Auth, Design System, Utils)
│
├── pages/                             <-- ALL PLATFORM PAGES & NAVIGATION
│   ├── navigation/                    <-- Persistent frame (Sidebar, TopNav, NavConfig)
│   ├── command_center/                <-- Hub 1: Dashboard, Server Intel, Announcements
│   ├── operations/                    <-- Hub 2: Operations Schedule, Event Dossier & Slotting
│   ├── mission_hub/                   <-- Hub 3: Mission Library, Overview Dossier, Create Dialog
│   ├── field_tools/                   <-- Hub 4: Mortar Ballistics Calculator
│   ├── doctrine_and_info/             <-- Hub 5: Wiki Knowledgebase, Vehicle Index, Modpacks
│   └── administration/                <-- Hub 6: Event Admin, Server Control, Personnel, Approvals, CMS
│
└── apps/                              <-- STANDALONE ENGINE WORKSPACES & CAD TOOLS
    ├── editor/                        <-- Scenario Creator (Eden CAD workspace)
    ├── planner/                       <-- Mission Planner (tactical whiteboard) — scaffold, no code
    ├── aar/                           <-- After-Action Report (telemetry replay) — scaffold, no code
    └── debug/                         <-- Engine testbenches (building viewer & world line-of-sight)
```

---

## Core Principles

1. **3 Clean Root Domains:** Foundations (`core`), Document Pages (`pages`), and Standalone
   CAD/Forensics Workspaces (`apps`).
2. **No Abstract "Shell":** Navigation is simply the platform frame (`pages/navigation/`) that
   wraps standard pages.
3. **Page-as-a-Folder:** Every page route has its own folder containing a main layout file and
   focused panel files.
4. **Independent Workspaces:** Each workspace under `apps/` owns 100% of its own UI and canvas
   mount. It imports from `core` and from the engine crates, never from `pages` and never from a
   sibling workspace. `editor/` and `debug/` are compiled and routed; `planner/` and `aar/` are
   scaffolds that `apps/mod.rs` does not yet declare.
5. **The Engines Are Workspace Crates.** The map engine and graphics engine live in
   `apps/website/map-engine` and `apps/website/graphics-engine`. There is no in-tree engine here,
   and the frontend never depends on the graphics engine directly — it hands the map engine a
   canvas handle, and the map engine speaks the graphics engine's frame vocabulary:

   ```text
   frontend ──► website-map-engine ──► website-graphics-engine
   ```

6. **State Lives Where It Survives.** The map engine owns state that survives a reload — the
   document, the entities, the undo stack, the selection, the tool command definitions. The
   frontend owns state that dies with the tab — hover, the in-flight drag, the pointer machine,
   the keybind map.

See `docs/platform/ENGINE_SPLIT_PROGRAM.md` for the crate layout these principles come from.
