# Frontend Architecture Hub (`src/v2`)

This directory contains the modernized, domain-driven architecture for the TBD Reforger platform frontend.

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
│   ├── mission_hub/                   <-- Hub 3: Mission Library, Overview Dossier, Scenario Creator
│   ├── field_tools/                   <-- Hub 4: Mortar Ballistics Calculator, Debug Viewers
│   ├── doctrine_and_info/             <-- Hub 5: Wiki Knowledgebase, Vehicle Index, Modpacks
│   └── administration/                <-- Hub 6: Event Admin, Server Control, Personnel, Approvals, CMS
│
├── map_engine/                        <-- SHARED MAP CAPABILITY (Zero UI opinions)
│   ├── renderer/                      <-- wgpu RenderEngine bridge & RAF loop
│   ├── camera/                        <-- Viewport math, pan/zoom, screen-to-world
│   ├── terrain/                       <-- DEM hillshade, satellite streaming, occluders
│   └── tools/                         <-- Pure tactical algorithms (Ruler, LOS, Mortar, Symbology)
│
└── apps/                              <-- THE 3 STANDALONE MAP APPLICATIONS
    ├── editor/                        <-- 1. SCENARIO CREATOR (Eden CAD workspace)
    ├── planner/                       <-- 2. MISSION PLANNER (Tactical whiteboard & briefing)
    └── aar/                           <-- 3. AFTER-ACTION REPORT (Telemetry replay player)
```

---

## Core Principles

1. **4 Clean Root Domains:** Foundations (`core`), Document Pages (`pages`), Tactical Engine (`map_engine`), and Standalone CAD/Forensics Tools (`apps`).
2. **No Abstract "Shell":** Navigation is simply the platform frame (`pages/navigation/`) that wraps standard pages.
3. **Page-as-a-Folder:** Every page route has its own folder containing a main layout file and focused panel files.
4. **Independent Map Applications:** The Editor, Planner, and AAR each own 100% of their respective UI and state, consuming the shared `map_engine/`.
