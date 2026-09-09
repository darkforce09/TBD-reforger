# TBD Reforger — Mission Selection UI Specification & Visual Breakdown

**Source File:** [`mission_selection.png`](./mission_selection.png)  
**Panel Title:** `CREATE GAME`  
**Host Identity Context:** `Mission Maker`  
**Scope:** Complete textual representation, visual layout, interactive controls, information architecture, and operational mechanics of the multiplayer mission selection and hosting interface.

---

## 1. Screen Title, Headers, and Navigation Context

* **Main Title Banner (Top Left):** `CREATE GAME` — Rendered in bold black uppercase on an edge-to-edge solid amber/gold header bar (`#D49B2A`).
* **Host Profile / Context Indicator (Top Right):** `Mission Maker` — Rendered in white bold text on the right edge of the amber header bar, identifying the host player's profile and administrative authority.
* **Navigation Context:** Serves as the multiplayer server hosting, scenario discovery, and match configuration screen. It bridges server creation / the server browser with the role assignment and slotting lobby. The host selects the map/terrain, selects a scenario, verifies telemetry and rules, configures server options, or accesses the workshop before committing to launch into the lobby.

---

## 2. Structural Architecture & Visual Layout

The UI uses a modular, dark translucent HUD overlay docked over an in-engine 3D backdrop (blurred airfield/hangar vista). The interface is structured into four primary quadrants flanked by global top and bottom control rails:

```
┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ [ CREATE GAME ]                                                                    [ Mission Maker ]  │ <- Top Bar (Amber)
├───────────────────┬─────────────────────┬─────────────────────────────────────────────────────────────┤
│ MISSION SETTINGS: │ MAPS: 53            │ Missions: 30     [ Search Input... ] [Q] [SHOW ALL MISSIONS]│
│ • Min Players: 1  │ - !Virtual Reality  │ - <<New - 3D Editor>>                                       │
│ • Max Players: 48 │ - [Altis] (Active)  │ - COOP 04 Firing From Vehicles                              │
│ • Type: Warlords  │ - Anizay            │ - COOP 12 Combat Patrol                                     │
│ • Respawn: BASE   │ - Beketov           │ - End Game 16 Kavala                                        │
│                   │ - Bukovina          │ - RHS BECTI 32 - Altis                                      │
│ SERVER PRESET:    │ - ...               │ - [SC 48 Warlords (Whole Island)] (Selected)                │
│   Regular         │                     │ - Support 04 Rodopoli                                       │
├───────────────────┤                     │ - Vanguard 50 Syrta                                         │
│ [SUMMARY] [CHAT]  │                     │ - Zeus 16+2 Master Altis (NATO)                             │
│ SC 48 Warlords    │                     │                                                             │
│ by Bohemia        │                     │                                                             │
│ [ Hero Graphic ]  │                     │                                                             │
│ Briefing synopsis │                     │                                                             │
├───────────────────┴─────────────────────┴─────────────────────────────────────────────────────────────┤
│ [ BACK ]                                               [ GAME OPTIONS ] [ STEAM WORKSHOP ] [ PLAY ]   │ <- Bottom Bar
└───────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

### Visual Regions & Docking:
1. **Top Header Rail:** Full-width high-contrast banner establishing operational mode (`CREATE GAME`) and host profile.
2. **Top-Left Telemetry Panel:** Compact card displaying scenario constraints (player capacity, game mode, respawn model) and server difficulty.
3. **Bottom-Left Detail & Comms Dock:** Tabbed container hosting either the scenario synopsis (title, author attribution, hero branding graphic, overview) or the pre-lobby multiplayer chat.
4. **Upper-Center-Left Column (Map Library):** Vertical scrollable list displaying installed maps/terrains with terrain type icons.
5. **Upper-Center-Right Column (Mission Library):** Two-column-wide scrollable scenario roster for the active map, equipped with keyword search and global filter toggles.
6. **Bottom Control Rail (Footer):** Navigation and commitment dock with navigation back, server settings, external workshop discovery, and game launch execution.

---

## 3. Interactive Controls Inventory

| Control Element | Label / Text | Control Type | Visual State | Purpose & Functional Logic |
| :--- | :--- | :--- | :--- | :--- |
| **Top Header Bar** | `CREATE GAME` | Status / Title | Static text (black on amber) | Identifies hosting mode. Non-interactive. |
| **User Identity** | `Mission Maker` | Profile indicator | Static text (white on amber) | Confirms active hosting administrator identity. |
| **Map Selection Row** | `Altis` (under `MAPS: 53`) | Selectable List Item | **Selected / Active** (White background pill, black text) | Filters the mission list to scenarios built for Altis. Refreshes count and metadata. |
| **Unselected Map Rows** | `!Virtual Reality`, `Anizay`, `Beketov`, `Bukovina`, etc. | Selectable List Items | Idle / Inactive (Translucent dark, white text, terrain icon) | Switching maps repopulates the mission library column with matching missions. |
| **Map Scroll Arrows** | `▲` (up), `▼` (down) | Scroll Indicators | Active / Interactive | Scrolls through the 53 detected terrain packages. |
| **Mission Search Bar** | Text input field | Text Input | Empty / Active focus | Real-time text filter to query missions by name, mode, or tags. |
| **Search Icon** | Magnifying glass icon (`Q`) | Button / Indicator | Dark bordered box | Commits or clears query search. |
| **Show All Missions** | `SHOW ALL MISSIONS` | Toggle Button | Default / Inactive (white text on dark slate) | When toggled on, ignores the selected map filter and displays all scenarios across all terrains. |
| **New 3D Editor** | `<<New - 3D Editor>>` | Action List Item | Accent (Bright green text) | Shortcut to launch the 3D Eden Editor on the selected map directly from the multiplayer session. |
| **Active Mission Row** | `SC 48 Warlords (Whole Island)` | Selectable List Item | **Selected** (Semi-transparent light-gray selection bar) | Loads scenario telemetry, synopsis text, and hero graphic into the left panels. |
| **Unselected Mission Rows** | `COOP 12 Combat Patrol`, `Escape 10 Altis`, `Zeus 16+2...` | Selectable List Items | Inactive (White text on dark background) | Selecting changes active scenario and refreshes detail panes. |
| **Mission Summary Tab** | `MISSION SUMMARY` | Sub-Panel Tab | **Active** (Solid white rectangular background, black uppercase text) | Displays mission synopsis, author, and preview artwork. |
| **Chat Tab** | `CHAT` | Sub-Panel Tab | Inactive (Dark translucent background, white text) | Displays live lobby chat log to communicate with connected players before starting. |
| **Back Button** | `BACK` | Push Button | Bottom-left docked, dark button with thin border | Cancels hosting and returns to the server browser or main menu. |
| **Game Options Button** | `GAME OPTIONS` | Push Button | Bottom-right area, dark button with thin border | Opens server configuration modal (passwords, ports, difficulty overrides, admin rules). |
| **Workshop Button** | `WORKSHOP` (with Steam icon) | Push Button | Highlighted (Teal/cyan background, black Steam logo, white text) | Opens Steam Workshop overlay for discovering, subscribing, and downloading new missions. |
| **Play Button** | `PLAY` | Push Button | Far bottom-right, dark button with thin border | Commits selection, binds mission payload, and advances all players into the slotting lobby. |

---

## 4. Information Architecture & Displayed Features

### 4.1 Mission Telemetry & Server Difficulty Panel (Top-Left)
* **`MISSION SETTINGS:`**
  * **`MIN. PLAYERS:`** `1` — Minimum player threshold required or recommended to start.
  * **`MAX. PLAYERS:`** `48` — Hard player capacity capped by mission slot design.
  * **`TYPE:`** `Warlords` — Game mode classification (e.g., Warlords, COOP, End Game, Zeus, Support, Vanguard, TVT).
  * **`RESPAWN:`** `BASE` — Defined respawn model (Base, Respawn on leader, Instant, None/1-Life).
* **`SERVER DIFFICULTY PRESET:`**
  * **`Regular`** — Active difficulty profile (Recruit, Regular, Veteran, Custom) governing crosshairs, 3rd-person camera, friendly tags, and AI skill.

### 4.2 Terrain / Map Library Column (`MAPS: 53`)
* **Total Terrains:** `53` detected maps.
* **Special Pinning:** Prefix conventions like `!Virtual Reality` use punctuation to sort foundational testing maps to the very top.
* **Terrain Icons:** Small graphic thumbnails next to each map name convey geographical biome (desert, temperate forest, winter, Mediterranean island).

### 4.3 Mission Library Column (`Missions: 30`)
* **Total Scenarios:** `30` missions found matching the active terrain filter (`Altis`).
* **Naming Conventions & Prefix Standards:**
  * `COOP [slots] [Name]` (e.g., `COOP 04 Firing From Vehicles`, `COOP 12 Combat Patrol`)
  * `End Game [slots] [Location]` (e.g., `End Game 16 Kavala`)
  * `Escape [slots] [Island]` (e.g., `Escape 10 Altis`)
  * `RHS [Mod/Mode] [Slots]` (e.g., `RHS BECTI 32 - Altis`, `RHS Co 1-10 Insurgency Action`)
  * `SC [slots] Warlords ([Sub-area])` (Sector Control Warlords variations: 16, 32, 48, 64 slots)
  * `Support [slots] [Location]` (e.g., `Support 04 Rodopoli`)
  * `Vanguard [slots] [Location]` (e.g., `Vanguard 50 Syrta`)
  * `Zeus [players+gamemaster] [Subtype]` (e.g., `Zeus 10+1 Defend Syrta`, `Zeus 16+2 Master Altis (NATO)`)

### 4.4 Mission Summary Panel (Bottom-Left)
* **Scenario Title:** `SC 48 Warlords (Whole Island)`
* **Author / Attribution:** `by Bohemia Interactive`
* **Hero Graphic / Artwork:** High-resolution branded mission artwork featuring the stylized "WARLORDS" logotype (NATO star in 'A', tactical hex grid in 'O'), yellow Bohemia crest lion, overlaid across a tactical topographic contour map.
* **Briefing Synopsis:** `"Capture sectors. Through them, advance to the enemy base and raid it. Parameters allow you to change various rules within the mission."` — Summarizes core objective loop and alerts host to configurable parameters.

---

## 5. Operational Mechanics for TBD Reforger

1. **Terrain-First Drilldown:**
   * Selecting a terrain immediately scopes down the mission list to compatible scenarios.
   * `SHOW ALL MISSIONS` allows overriding this filter to inspect unassigned or multi-terrain missions.
2. **Instant Keyword Querying:**
   * Text search filters scenario titles by substring (e.g., "Warlords", "COOP", "Zeus", "RHS") in real time.
3. **Pre-Lobby Communication:**
   * Host can switch between `MISSION SUMMARY` and `CHAT` to poll connected players on preferred game modes before committing to a map change.
4. **Mission Creator & 3D Editor Handoff:**
   * `<<New - 3D Editor>>` provides a direct authoring shortcut to open the 3D editor on the selected map from the hosting workflow.
5. **Execution & Session Binding:**
   * `GAME OPTIONS` configures server passwords, port overrides, and engine rules.
   * Clicking `PLAY` commits the scenario payload and transitions all connected clients into the role assignment and ORBAT slotting screen.
