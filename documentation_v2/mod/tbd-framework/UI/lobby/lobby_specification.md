# TBD Reforger — Lobby & Role Assignment UI Specification & Visual Breakdown

**Source File:** [`lobby_screen.png`](./lobby_screen.png)  
**Panel Title:** `ROLE ASSIGNMENT`  
**Host Context Indicator:** `Mission Maker`  
**Scenario Name:** `wog_187_chollima_on_the_wing_10`  
**Active Faction:** `BLUFOR` (US Army, Attacking)

---

## 1. Executive Summary & Screen Purpose

The **Role Assignment** screen (commonly known as the **Lobby** or **Slotting Screen**) is the primary pre-match staging interface in the milsim framework. It bridges scenario selection (`CREATE GAME` / Server Browser) with live deployment (`Briefing` -> `Safestart` -> Combat).

Connected clients select their side (BLUFOR vs. OPFOR), navigate a hierarchical Order of Battle (ORBAT), claim designated combat roles within squads and vehicle crews, monitor client latency/ping, toggle AI bot occupancy, and wait for the match host or referee to lock the lobby and commit the roster into the briefing phase.

---

## 2. Screen Headers, Profile Badges & Scenario Telemetry

### 2.1 Top Global Header Bar
* **Main Title Banner (Top Left):** `ROLE ASSIGNMENT` — Rendered in bold, uppercase black sans-serif typography (`#000000`) docked edge-to-edge on a solid amber/gold header rail (`#D49B2A`).
* **Host Identity / Administrative Authority (Top Right):** `Mission Maker` — Rendered in bold white sans-serif text (`#FFFFFF`), confirming that the client possesses administrative hosting controls (e.g., `LOCK` and `DISABLE AI`).

### 2.2 Scenario Telemetry & Player Count Strip (Sub-Header)
Located immediately beneath the amber header rail on a translucent dark slate container (`#0A0A0A` @ ~85% opacity):

| Telemetry Field | Displayed Value | Description & Purpose |
| :--- | :--- | :--- |
| **`Mission:`** | `wog_187_chollima_on_the_wing_10` | Standard community scenario filename: 187 total player slots; scenario title *"Chollima on the Wing"* v1.0. |
| **`Map:`** | `Chernarus Autumn` | Island / terrain environment (autumn vegetation variant of Chernarus). |
| **`Description:`** | `US Army (синие, атака) vs КНДР (красные, оборона)` | Bilingual matchup synopsis: US Army (*Blue / BLUFOR, Attacking*) vs DPRK / North Korea (*Red / OPFOR, Defending*). |
| **`Listed Players:`** | `1` | Real-time counter of total clients currently connected inside the lobby. |

---

## 3. UI Layout & Structural Architecture

The interface uses a 16:9 full-screen dark HUD overlay docked over an in-engine 2D satellite/topographical map of Chernarus (showing coastline contours and terrain elevation in dark blues and greens).

```
┌─────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ ROLE ASSIGNMENT                                                                           Mission Maker │ <- Top Rail
├──────────────────────────────────────────────────────────────────────┬──────────────────────────────────┤
│ Mission:     wog_187_chollima_on_the_wing_10                         │ Listed Players:                1 │ <- Telemetry
│ Map:         Chernarus Autumn                                        │                                  │
│ Description: US Army (синие, атака) vs КНДР (красные, оборона)       │                                  │
├─────────┬────────────────────────────────────────────────────────────┼────────────────────────┬───┬─────┤
│ Side:   │ Roles for BLUFOR:                                       [^]│ Players:             ▲ │Ping:  🔊│
│ ┌─────┐ │ A1-1 Company HQ - M113A3 MEV                               │ -----------------------+---+-----│
│ │0/92 │ │ ┌──────────────────────────────────────────────────┬───┬──┐│ Mission Maker (Host)   │ 0 │     │
│ │Blufor│ │ │ Company commander                                │[C]│👤││                        │   │     │
│ └─────┘ │ │ AI                                               │   │  ││                        │   │     │
│ ┌─────┐ │ └──────────────────────────────────────────────────┴───┴──┘│                        │   │     │
│ │0/95 │ │   2ic                                              [C] 👤̸ │                        │   │     │
│ │Opfor│ │   AI                                                       │                        │   │     │
│ └─────┘ │                                                            │                                  │
│ (Red    │ A1-2 M60A1                                                 │                                  │
│ diamond)│   Tank commander                                   [C] 👤̸ │                                  │
│         │   AI                                                       │                                  │
│         │   Driver | Eng                                     [C] 👤̸ │                                  │
│         │   AI                                                       │                                  │
│         │   Gunner                                           [C] 👤̸ │         (Dark Satellite          │
│         │   AI                                                       │          Terrain / Map           │
│         │                                                            │           Background)            │
│         │ A1-3 M60A1                                                 │                                  │
│         │   Tank commander                                   [C] 👤̸ │                                  │
│         │   AI                                                       │                                  │
│         │   Driver | Eng                                     [C] 👤̸ │                                  │
│         │   AI                                                       │                                  │
│         │   Gunner                                           [C] 👤̸ │                                  │
│         │   AI                                                       │                                  │
│         │                                                            │                                  │
│         │ A1-4 M60A1                                                 │                                  │
│         │   Tank commander                                   [C] 👤̸ │                                  │
│         │   AI                                                       │                                  │
│         │   Driver | Eng                                     [C] 👤̸ │                                  │
│         │   AI                                                    [v]│                                  │
│         ├─────────────────────────────────────────────────┬──────────┤                                  │
│         │                                                 │DISABLE AI│                                  │
├─────────┴─────────────────────────────────────────────────┴──────────┴──────────────────────────────────┤
│ [ BACK ]                                                                                 [ LOCK ] [ OK ]│ <- Bottom Rail
└─────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

### Visual Regions & Docking
1. **Top Rail (Y: 0% - 4%):** Solid amber bar housing screen mode title and host administrative profile.
2. **Metadata Strip (Y: 4% - 10%):** Translucent row with scenario name, terrain, matchup overview, and connected client count.
3. **Left Workspace — Faction & ORBAT Tree (X: 0% - 47%, Y: 10% - 94%):**
   - **Faction Rail (X: 0% - 7%):** Narrow vertical dock with faction selection buttons (`Side:`).
   - **Role Assignment Pane (X: 7% - 47%):** Scrollable nested hierarchy of squads and individual playable slots.
   - **AI Command Sub-rail:** Bottom-docked `DISABLE AI` button and vertical scroll arrows (`^` / `v`).
4. **Right Workspace — Connected Players & Tactical Map (X: 47% - 100%, Y: 10% - 94%):**
   - **Player Roster Table:** Column-sorted table of connected clients, host badges, pings, and audio indicators.
   - **Tactical Viewport / Backdrop:** In-engine 2D satellite map of the scenario theater.
5. **Bottom Control Rail (Y: 94% - 100%):** Navigation return (`BACK`) on the far left; administrative lock (`LOCK`) and readiness/launch (`OK`) on the far right.

---

## 4. Interactive Controls Inventory

| Control Element | Label / Graphic | Control Type | Visual State | Placement | Purpose & Functional Behavior |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Top Header** | `ROLE ASSIGNMENT` | Status / Title | Static text (black on amber) | Top-left rail | Identifies active lobby phase. Non-interactive. |
| **Host Profile** | `Mission Maker` | Status / Identity | Static text (white on amber) | Top-right rail | Confirms host player and administrator permissions. |
| **BLUFOR Side Button** | `0/92`<br>`Blufor` | Faction Button | **Active / Selected** (Solid blue rectangular fill `#0E4D8B`, white text) | Left rail under `Side:` | Filters ORBAT tree to BLUFOR forces. Shows `0` claimed out of `92` slots. |
| **OPFOR Side Button** | `0/95`<br>`Opfor` | Faction Button | **Inactive / Unselected** (Red diamond boundary `#990000`, dark fill, white text) | Left rail under `Side:` | Switches ORBAT tree to OPFOR forces. Shows `0` claimed out of `95` slots. (Total = 187 slots). |
| **Scroll Up** | `▲` (`^`) | Scroll Button | Active | Top-right of role tree pane | Scrolls the ORBAT slot list upwards. |
| **Scroll Down** | `▼` (`v`) | Scroll Button | Active | Bottom-right of role tree pane | Scrolls the ORBAT slot list downwards. |
| **Selected Role Row** | `Company commander`<br>`AI` | Selectable List Item | **Selected / Focused** (Full-width light-gray translucent highlight bar) | First slot under `A1-1 Company HQ` | Focused role item. Shows slot title and current occupant (`AI` in red). |
| **Unselected Role Rows** | `2ic`, `Tank commander`, etc. | Selectable List Items | Idle / Inactive (Dark background, white text, red `AI`) | Grouped under respective squad headers | Clicking focuses the slot for inspection or claiming. |
| **AI Slot Indicator** | Microchip / CPU Icon (`🔲`) | Indicator Icon | Active / Per slot | Right edge of each role row | Indicates whether the slot will spawn a bot if unoccupied at match launch. |
| **Slot Claim Icon** | Solid Silhouette (`👤`) | Status / Claim Icon | **Available / Claimable** (Solid white human silhouette) | Right edge of `Company commander` row | Signals that the slot is open and immediately claimable by a human player. |
| **Slot Restricted Icon** | Crossed-Out Silhouette (`👤̸`) | Status / Lock Icon | **Restricted / Subordinate Lock** (Silhouette with diagonal slash) | Right edge of `2ic` and crew rows | Indicates slot is locked until the unit leader slots in, or requires specific admin permissions. |
| **Disable AI Button** | `DISABLE AI` | Action Push Button | Active (Dark button with light border, white text) | Bottom-right of role tree pane | Host administrative command: globally clears AI from all unoccupied slots so bots do not spawn. |
| **Player List Header** | `Players:` | Table Header | Static | Top-left of right column | Column title for connected clients list. |
| **Ping Sort Header** | `▲` `Ping:` | Table Sort Header | Active (Ascending sort arrow `▲`) | Top-right of right column | Sorts player roster ascending by network round-trip time. |
| **Voice / VON Icon** | Speaker Icon (`🔊`) | Status / Audio Toggle | Static / Active icon | Far-right edge of player header | Indicates global lobby voice communications status or player mute control. |
| **Connected Player Row** | `Mission Maker (Host)` | Table Row Item | Selected / Host entry | First row under `Players:` | Displays player name with `(Host)` badge; ping column shows `0` ms (local host). |
| **Back Button** | `BACK` | Push Button | Active (Dark rectangle, thin light border, white text) | Bottom-left corner | Cancels role assignment and returns to Mission Selection or Server Browser. |
| **Lock Button** | `LOCK` | Push Button | Active (Dark rectangle, thin light border, white text) | Bottom-right corner (left of `OK`) | Host administrative control: locks the lobby, preventing new joins or slot switching. |
| **OK Button** | `OK` | Push Button | Inactive / Dimmed state | Bottom-right corner (far right) | Readiness commit / Launch button. Commits slot choice; once players are ready and host clicks `OK`, advances match to Briefing. |

---

## 5. Order of Battle (ORBAT) Architecture & Displayed Units

The mission shown features a combined-arms mechanized engagement with **187 total playable slots** (BLUFOR: 92 slots; OPFOR: 95 slots). The visible BLUFOR hierarchy includes:

```
BLUFOR (0/92 Claimed)
├── A1-1 Company HQ - M113A3 MEV (Mechanized Command & Medical Section)
│   ├── [Selected] Company commander ── [AI] ── [Chip] [👤 Available]
│   └── 2ic (Executive Officer) ──────── [AI] ── [Chip] [👤̸ Restricted]
│
├── A1-2 M60A1 (Armor Section — Patton MBT #1)
│   ├── Tank commander ────────────────── [AI] ── [Chip] [👤̸ Restricted]
│   ├── Driver | Eng ──────────────────── [AI] ── [Chip] [👤̸ Restricted]
│   └── Gunner ────────────────────────── [AI] ── [Chip] [👤̸ Restricted]
│
├── A1-3 M60A1 (Armor Section — Patton MBT #2)
│   ├── Tank commander ────────────────── [AI] ── [Chip] [👤̸ Restricted]
│   ├── Driver | Eng ──────────────────── [AI] ── [Chip] [👤̸ Restricted]
│   └── Gunner ────────────────────────── [AI] ── [Chip] [👤̸ Restricted]
│
└── A1-4 M60A1 (Armor Section — Patton MBT #3)
    ├── Tank commander ────────────────── [AI] ── [Chip] [👤̸ Restricted]
    └── Driver | Eng ──────────────────── [AI] ── [Chip] [👤̸ Restricted]
        (Gunner row truncated by scroll view)
```

### Tactical Conventions
* **Callsign Taxonomy:** `[Company Letter][Platoon Number]-[Squad/Vehicle Number]` (e.g., `A1-1` = Alpha Company, 1st Platoon, Section 1).
* **Asset-Bound Squads:** Unit headers explicitly name the organic vehicle allocated to that unit (`M113A3 MEV`, `M60A1`).
* **Specialization Suffixes:** Roles like `Driver | Eng` signify that the driver holds combat engineering and repair authorizations.

---

## 6. Operational & Slotting Mechanics

1. **Faction Side Selection:**
   - Clicking `Blufor (0/92)` or `Opfor (0/95)` filters and loads that faction's ORBAT tree.
2. **Role Browsing & Claiming:**
   - Clicking an available slot (`👤`) claims it for the player. The red `AI` label is replaced with the player's username.
   - The faction counter updates in real time (e.g., `1/92 Blufor`).
3. **Leadership Gating & Subordinate Locking:**
   - Subordinate roles (`2ic`, `Gunner`, `Driver`) display the restricted icon (`👤̸`) until the unit leader (`Company commander`, `Tank commander`) is claimed. This prevents squads from deploying without leadership and communications.
4. **AI Bot Toggle (`DISABLE AI`):**
   - By default, unslotted roles spawn as AI bots (`[AI]` + chip icon). Clicking `DISABLE AI` clears all bots, ensuring only human-claimed roles spawn.
5. **Host Administrative Controls:**
   - `LOCK` freezes the lobby so players cannot switch slots during final briefings.
   - Clicking `OK` as host verifies ready states and advances all players into the **Briefing Screen**.
