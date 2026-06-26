# Mission Creator & Planner Design Blueprint

## Core Philosophy
The Mission Creator must solve the ultimate UX oxymoron: **Granular, God-like Control vs. Apple-tier Simplicity.** An 80-year-old grandma should be able to click and drag elements to build a functioning combined-arms operation. 

The architecture relies on a strict **JSON Data Contract**. The Website Mission Creator outputs a highly detailed JSON file. The Go backend parses it, and the Arma Reforger Mod consumes it to generate both the 3D world and the immersive Briefing UI automatically.

## Technical Design Constraints
1. **The Map Engine (2D + Heightmaps):** A flat 2D tile map is insufficient for spawning units. The web mission creator's map engine must be built using Topographic/Satellite tiles paired with a **Z-Axis Heightmap matrix**. This is critical: if we place an AT-ST on a hill in the 2D web UI, the JSON must accurately export the Z-coordinate (elevation) so the vehicle doesn't spawn underground or floating in the air.
2. **Phase 1 Focus (Web JSON Generator):** The current scope is entirely focused on the web interface producing the perfect, strictly-defined JSON schema. The Arma Enfusion (C++) scripts to read the JSON and render the 3D Uniforms tab, Vehicle Inventories, and Squad Layouts will be developed in a later phase.
3. **Frictionless Website-to-Game Sync (Opt-In Loading):** Players will not download files to their PC or paste long hash codes to load plans. The sync is invisible but opt-in. When a player draws a plan on the website, it saves to their Discord Identity in the database. When they load into the Arma briefing room, there is a dedicated **"Load Web Plan"** button on the map screen. Clicking this fetches their personal markers (or a Commander's shared markers) via the API and plots them. This prevents the map from becoming an unreadable mess if 10 squad leaders all have their auto-loading plans overlapping.

---

## 1. The Website Mission Creator (The Forge)
This is where Mission Makers build the operation.

### Automated vs. Manual Data
To ensure simplicity, the creator automatically generates stats based on what you drag onto the map.
- **Auto-Generated:** Thermals presence, view distance limits, time of day, weather, vehicle asset lists, squad layouts, crate inventories, safe start zones, and spawn points.
- **Manually Entered:** Lore/Briefing text, specific Attack/Defend objectives per faction, custom rules, and additional notes.

### No-Code Logic & Objectives (The FNF Model)
Mission makers should never have to write code to create a functioning objective. The Asset Palette includes a **Logic & Events** tab.
- **Drag-and-Drop Objectives:** If you want BLUFOR to attack a town, you drag an "Attack Objective" zone over the town and assign it to BLUFOR. The underlying JSON and Go backend handle all the complex game scripting automatically.
- **Zero Scripting Required:** Triggers, spawn zones, and win conditions are placed visually and configured via simple dropdowns in the Contextual Inspector.

### Collaborative Visual Version Control (Git Integration)
Because the mission is saved as a strict JSON data contract, we can build true visual version control for collaborative mission making.
- **Visual Diffing:** If Player A saves v1.0, and Player B saves v1.1, the website can show a visual diff directly on the 2D map.
- Instead of reading raw Git code (`- Abrams`, `+ BTR`), the map literally highlights deleted assets in **red ghosts** and newly added assets in **green**.
- **Team Workflow:** Multiple creators can work on a massive campaign, visually reviewing each other's additions without ever looking at a line of code.

### UI Layout: The "macOS" Arma Editor

> **Authoritative UX is now `Design_Docs/Mission_Creator_Architecture/agent_execution.md`
> + `ux_spec.md`** (the **Arma 3 Eden Editor docked shell**, modernized with Aegis
> glass). The product intent below still holds; two interaction details were superseded by the
> locked Decisions log — see the **bold** notes. The `Mission_Creator_Mock_Up/**/code.html` and
> `macOS_Blueprints/**` HTML mockups are **historical explorations only** — do not execute against them.

The visual layout merges the classic layout of the Arma Eden Editor with the premium, native feel of macOS (frosted glass, rounded corners, and split-pane simplicity):
- **Full-Bleed Map:** The interactive, topographical 2D map takes up 100% of the screen background. The editor runs **fullscreen** — the platform Sidebar/TopNav are hidden on `/missions/:id/edit`.
- **Top Bar (The Mac Menu Bar):** A thin, frosted-glass header containing global parameters (Eden-style Time of Day slider/scrub, Weather toggles, Lore/Briefing editor).
- **Left Sidebar (The Finder/Outliner):** A translucent sidebar **docked flush left** — the hierarchical tree view (like Mac Finder) of everything placed on the map (ORBAT on top, Editor Layers folders below).
- **Right Sidebar (The Asset Palette):** A frosted-glass panel **docked flush right** with categorized drag-and-drop assets (Factions, Vehicles, Markers, Triggers). **The Asset Palette stays visible at all times — it does NOT swap to an inspector on selection.**
- **Contextual editing (Attributes Modal):** **Superseded — double-clicking a unit opens the Aegis-glass Attributes Modal** (Transform / Identity / States / Arsenal tabs), not a popover/bottom panel. Single-click only selects; this keeps the always-on Asset Palette intact.
- **Polygon Tools:** Clean, minimalist toolbar icons to draw the **Safe Start Area** directly onto the map.

---

## 2. The Loadout Forge (Web Arsenal)
Creating loadouts in-game (even with Ace Arsenal) is incredibly tedious. The web creator will feature a dedicated Loadout Forge designed for mass standardization with deep customizability.

### Modular Standardization (The FNF Model + Templates)
- **Pre-Built Factions:** When a mission maker drops a predefined faction (e.g., US Army 2005) onto the map, the system automatically assigns fully standardized kits to each slot.
- **Custom Mass-Application:** For custom factions, mission makers do not have to manually edit every slot. They can apply modular templates to entire squads or factions instantly:
  - *"Apply Standard Medical Loadout to Alpha Squad"*
  - *"Apply Woodland Camo Uniform to entire Faction"*
  - *"Apply Base Rifle Template to all Rifleman slots"*
- **Automated Heavy Lifting:** By mass-applying these standard templates, the creator automates the boring stuff (bandages, base uniforms) and only spends time doing the fine details (e.g., clicking on the Squad Leader to give him a unique radio or helmet).

### Deep Customizability
- **Granular Edits:** Despite the automated standardization, the mission maker can click into any specific slot (e.g., Alpha 1-1 Rifleman) to override the standard kit.
- **The "Vibe" / Visual Inventory:** Text lists are boring, but inventory Tetris is exhausting. The Loadout Forge will NOT feel like the clunky Arma Reforger minigame. It will be a flat, dead-simple, highly visual 2D grid displaying the icons of the primary weapon, vest, helmet, and uniform. You get the visual coolness without the micromanagement.
- **Seamless Mod Support (Zero-Click Updates):** Uploading zip files is too complex for an 80-year-old grandma. The mod support must be 100% automated. When the Arma Game Server updates its modpack, the server itself uses the Go REST API to securely push the new weapon/gear data (classnames and 2D UI icons) directly to the website's database. The mission maker simply opens the website and all modded gear is magically there. Zero manual file handling required.

---

## 3. The Website Mission Planner (Tactical Board)
This is where Squad Leaders and Commanders plan the operation *before* the server launches.
- Uses the same detailed 2D map as the Creator.
- Allows players to draw tactical markers (lines, arrows, phase lines, obj markers) similar to SWT markers.
- **Identity Sync:** Because the player's Discord is linked to their Arma ID, they can click "Save Plan to Profile."
- When they load into the Arma server, a custom menu allows them to click "Import Web Plan," instantly plotting their website markers onto their in-game map.

---

## 4. The In-Game Briefing UI (Pre-Deployment Phase)
Once players load into the Arma server, they are locked in the Safe Start zone. They open the Briefing UI (top-left menu). The Arma Mod reads the JSON file generated by the website and populates these tabs:

### Information Tabs
- **Lore & Briefing:** The narrative background of the op.
- **Mission Parameters:** View distance, thermals status, time of day, weather, time limit.
- **Objectives:** Faction-specific (e.g., BLUFOR sees "Attack X", OPFOR sees "Defend X").
- **Rules & Notes:** Custom stipulations added by the mission maker.

### Asset & Logistics Tabs
- **Uniforms:** Visual renders of the OpFor and BluFor faction uniforms to prevent friendly fire.
- **Vehicle Assets:** Friendly and Enemy tabs showing vehicle icons, exact names, and total counts.
- **Vehicle & Crate Inventory:** Shows what weapons/ammo are inside. Clicking the vehicle name pans the map to its exact location in the Safe Start zone.

### Personnel Tabs
- **Squad Layout (Personal):** Granular, itemized inventory specifically for *your* squad's slots.
- **Squads (Global):** A high-level roster showing who is slotted into which squad.

### The Map Screen
- Shows the Safe Start area polygon.
- Shows friendly spawn points and vehicle parking.
- Displays a prominent timer: *Time elapsed in Briefing*.
- **Hardcore Rule:** No crutches (C-Tab, Shack-Tac, Squad Radar). Players must manually plot their positions.

---

## 5. Future Scope (Roadmap)
> [!TIP]
> These are logged so the foundation supports them, but will be built later.
- **AI Integration:** Using AI to auto-generate balanced squad compositions, write immersive lore, or suggest objective placements based on terrain.
- **Simplified 3D:** Upgrading the web's 2D map to a lightweight 3D topological view.
- **DCS-Style AAR (After Action Report):** Upgrading from simple 2D OCAP replays to a high-fidelity, high-data-throughput telemetry viewer that accurately replays engagements, trajectories, and movements.
