# Mission Creator: Technical Architecture & Problem Statement

## 1. Executive Summary
The "Mission Creator" is the most complex engineering challenge of the TBD Reforger platform. It is not a standard web form; it is effectively a **Visual Compiler** and a **2D Geographic Information System (GIS)**. 

The core requirement is to allow users to visually drag, drop, and edit up to 200 deeply nested entities (players, squads, weapons, ammo, waypoints) across a topographic map, and ultimately serialize that visual state into the strict, heavily nested JSON configuration files required by Arma Reforger.

## 2. Core Complexities & Difficulties

### A. The HTML DOM Limit (The 200 Slot Problem)
Standard web architecture uses HTML `<div>` nodes to render elements. If we attempt to render a massive topographic map featuring 200 player slots, vehicles, and draggable waypoints using HTML DOM nodes, the browser's reflow/repaint cycle will collapse, resulting in massive lag and a completely unusable editor.
**The Problem:** We need a video-game level rendering pipeline inside the browser.

### B. The Z-Axis (The Heightmap Problem)
In tactical mission planning (e.g., placing mortar tubes, AA batteries, or sniper teams), the visual X/Y coordinate is insufficient; the user must know the exact elevation (Z-axis). 
**The Problem:** A standard 2D image (like a PNG map) has no elevation data. We need to parse raw Digital Elevation Models (DEM) from the Enfusion engine so the user can hover their mouse and instantly read the elevation in meters.

### C. Deep Relational State (The Nesting Problem)
Arma Reforger loadouts are strictly hierarchical:
`Faction -> Squad -> Role -> Inventory Node -> Primary Weapon Slot -> M4A1 -> Optic Slot -> ACOG`
**The Problem:** Managing this deep nesting in standard React state causes cascading re-renders. If a user swaps an optic on an M4, the entire squad tree cannot re-render without causing severe UI stuttering.

### D. The Data Validation (The Registry Problem)
You cannot put an ACOG on an RPG, and you cannot put an RPG in a pistol slot.
**The Problem:** The UI cannot blindly accept drag-and-drop events. Every drop action must be validated against a massive registry of valid game items, checking slot compatibility, volume/weight limits, and faction restrictions before updating the state.

---

## 3. The Proposed Architectural Stack

To solve these problems, the Mission Creator must be architected as a **High-Performance WebGL Application** embedded inside the React platform.

### Pillar 1: The WebGL Viewport (PixiJS or React Konva)
We bypass the HTML DOM entirely for the main map editor. 
- The map, NATO icons, vehicles, and waypoints are drawn onto a massive `<canvas>` element using WebGL. 
- This offloads rendering to the user's GPU, allowing 60 FPS panning, zooming, and mass-selection of hundreds of entities.

### Pillar 2: Heightmap Data Textures
To solve the Z-axis problem, we export the 16-bit raw heightmap from the Arma Reforger tools.
- We load this heightmap into the WebGL engine as a "Data Texture."
- We employ a Raycaster: When the user's cursor moves, the engine reads the underlying pixel's greyscale value, converts it to meters, and outputs the exact elevation.
- If the map is too large for RAM, we must chop it into GIS tiles (e.g., Mapbox GL) and stream them into the browser asynchronously.

### Pillar 3: Normalized State Management (Zustand)
To solve the deep nesting problem, we use **Zustand** but enforce a strict **Normalized State** architecture.
- Instead of nesting data, we store all entities flatly in dictionaries (e.g., `weaponsById`, `unitsById`) and link them using ID arrays.
- This allows us to instantly update a single weapon attachment without re-rendering the soldier holding it.

### Pillar 4: The Master Item Registry
Before the UI can function, we require a database containing every valid item in the game/modpack (weapons, magazines, compatible optics). This acts as the single source of truth for the drag-and-drop validation logic.

### Pillar 5: The Serializer (The Compiler)
When the user clicks "Save", a dedicated web worker traverses the normalized Zustand state, maps the 2D canvas coordinates and inventory links, and serializes them into the exact, strictly typed JSON configuration string that Arma Reforger expects.

---

## 4. Prerequisites for Development
Before a single line of React or WebGL is written, the following assets must be secured:
1. **The Target JSON:** A perfect, working example of the final `loadout.json` / `mission.json` file expected by the Reforger mod.
2. **The Map Assets:** High-resolution top-down imagery of Everon/Arland to serve as the map tiles.
3. **The DEM Data:** The raw 16-bit heightmap data extracted from the Enfusion engine for elevation tracking.

Open work: [`docs/TICKET_LEAD.md`](../../docs/TICKET_LEAD.md) · full table [`docs/TICKET_REGISTRY.md`](../../docs/TICKET_REGISTRY.md)
