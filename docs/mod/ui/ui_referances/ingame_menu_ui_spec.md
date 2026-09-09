# TBD Reforger — In-Game Menu UI Specification & Visual Breakdown

**Source Reference Directory:** [`ingame_menu/`](./ingame_menu/)  
**Scope:** Complete textual representation, control inventory, and operational logic for all 15 reference captures of the in-game menu and administrative toolset.

---

## Architecture Overview

The in-game menu suite consists of two distinct tiers:
1. **Player Pause / Home Menu Tier (`home_menu*`, `player_options`):** A lightweight overlay accessible by all connected players via `Esc`. Dynamically adapts between pre-start warmup (readiness voting) and live combat.
2. **Administrative / Referee Suite (`admin_*`):** A modal command center accessible to game masters and administrators (`WOG Admin Menu`), featuring a global status bar, an 8-module vertical navigation dock, and deep operational controls.

```
                             [ Player presses ESC ]
                                       │
                         ┌─────────────┴─────────────┐
                         ▼                           ▼
               [ Pre-Start Phase ]           [ Live Combat Phase ]
             (home_menu_before_start)       (home_menu_after_start)
             - OPTIONS                      - OPTIONS
             - ADMIN PANEL                  - ADMIN PANEL
             - CONSOLE                      - CONSOLE
             - CLOSE                        - CLOSE
             - MY TEAM IS READY!            - FIX UNIFORM BUG
             - MY TEAM IS NOT READY!
             - FIX UNIFORM BUG
                         │
             ┌───────────┴───────────────────────────┐
             ▼                                       ▼
     [ Player Options ]                     [ WOG Admin Suite ]
    (player_options.png)                     (8 Tabbed Modules)
    - Terrain / Presets                      1. Mission (admin_home)
    - Earplug Muting                         2. Radio (admin_radio)
    - Terrain Saving                         3. Teleport (admin_teleport)
    - Nickname Highlight                     4. Chat (admin_chat)
    - Freeze Beep Alert                      5. Spawner (admin_spawner_*)
                                             6. Heal & Repair (admin_heal_repair)
                                             7. Kick & Ban (admin_kick_ban)
                                             8. Server & Other (admin_server_and_other)
```

---

# Section 1: Player Home Menu & Options

## 1.1 Root Home Menu (`home_menu.png`)

**Image:** `docs/mod/ui/ui_referances/ingame_menu/home_menu.png`  
**Panel Title:** `Main menu`  
**Context:** Top-level pause overlay during live gameplay. Displays active selection state on the `OPTIONS` row.

### UI Layout & Placement
- **Left Navigation Stack:** Anchored in the upper-mid-left screen quadrant (~20% left margin, vertically centered). A vertical stack with a title header bar and four uniform rectangular buttons.
- **Right Action Area:** Docked in the lower-mid-right screen quadrant (~70% left margin). A single standalone button for emergency recovery.
- **Backdrop:** Translucent view into the active 3D world.

### Control Inventory
| Control / Label | Element Type | Visual State | Purpose & Action |
| :--- | :--- | :--- | :--- |
| **`Main menu`** | Title Bar | Dark navy/slate background (`#212832`), olive/khaki text (`#9da06d`), sentence case. | Identifies the root pause menu. Non-clickable. |
| **`OPTIONS`** | Menu Button | **Active / Focused:** Solid bright white background (`#ffffff`), centered black bold text (`#000000`). | Opens the client-side **Player Options** dialog (`player_options.png`). |
| **`ADMIN PANEL`** | Menu Button | Default / Idle: Dark slate background (`#1a222c`), olive/khaki text (`#9da06d`). | Opens the referee **WOG Admin Menu** (`admin_home.png`). Gated by admin privileges. |
| **`CONSOLE`** | Menu Button | Default / Idle: Dark slate background (`#1a222c`), olive/khaki text (`#9da06d`). | Opens the engine/script debug console for diagnostics and command execution. |
| **`CLOSE`** | Menu Button | Default / Idle: Dark slate background (`#1a222c`), olive/khaki text (`#9da06d`). | Dismisses the menu overlay and restores full player camera and input control. |
| **`FIX UNIFORM BUG`** | Standalone Action Button | Dark charcoal container, high-visibility crimson red text (`#9e1b1b`). | Executes client-side inventory reconciliation: strips and re-equips character containers while preserving inventory items to fix desynced or invisible clothing models. |

---

## 1.2 Home Menu — Pre-Start / Warmup Phase (`home_menu_before_start.png`)

**Image:** `docs/mod/ui/ui_referances/ingame_menu/home_menu_before_start.png`  
**Panel Title:** `Main menu`  
**Context:** In-game pause menu during the pre-match preparation / briefing / hard-freeze phase.

### Visual Differences from Live Play
- The right action column contains a **Team Readiness Cluster** positioned directly above the `FIX UNIFORM BUG` button.

### Additional Pre-Start Controls
| Control / Label | Element Type | Visual State | Purpose & Action |
| :--- | :--- | :--- | :--- |
| **`MY TEAM IS READY!`** | Action Button | Dark navy container, muted olive/khaki uppercase text (`#8f9661`). | Signals squad/faction readiness to the server administrator during freeze time. |
| **`MY TEAM IS NOT READY!`** | Action Button | Dark navy container, two-line text (`MY TEAM IS NOT` / `READY!`) in olive/khaki text. | Revokes team readiness or requests an administrative hold on match start. |
| **`FIX UNIFORM BUG`** | Standalone Button | Dark navy container, crimson red text (`#c02020`). Separated below readiness controls. | Re-equips uniform and vest gear without reconnecting. |

---

## 1.3 Home Menu — Post-Start Phase (`home_menu_after_start.png`)

**Image:** `docs/mod/ui/ui_referances/ingame_menu/home_menu_after_start.png`  
**Panel Title:** `Main menu`  
**Context:** In-game pause menu immediately following match start (freeze time dropped, live combat started).

### Visual & Functional Behavior
- **Dynamic Element Stripping:** The readiness cluster (`MY TEAM IS READY!` / `MY TEAM IS NOT READY!`) is unmounted from the viewport, leaving only `FIX UNIFORM BUG` in the secondary action area.
- Prevents accidental readiness inputs while combat is in progress.

---

## 1.4 Player Options Modal (`player_options.png`)

**Image:** `docs/mod/ui/ui_referances/ingame_menu/player_options.png`  
**Panel Title:** `Options`  
**Context:** Client-side graphics, sound, and interface preferences dialog.

### UI Layout & Placement
- Compact vertical dialog (~360px wide) in the upper-left screen quadrant.
- **Header:** Ochre/amber-gold horizontal title banner (`#c68b20`) with white text.
- **Body:** Dark slate/navy background (`#1b2533`).
- **Footer:** Docked `CLOSE` button beneath the bottom-right corner.

### Control Inventory
| Control / Label | Section | Element Type | Value / State | Functionality |
| :--- | :--- | :--- | :--- | :--- |
| **`Options`** | Header | Title Banner | White text on amber gold | Identifies the modal. Non-clickable. |
| **`View distance`** | View distance | Section Label | White text | Header for draw distance settings. |
| **`Terrain`** | View distance | Stepper / Slider | `<` [ Black track ] `>` | Adjusts maximum terrain mesh rendering distance. |
| **`Preset 1`** | View distance | Stepper / Slider | `<` [ Grey filled bar ] `>` | View distance preset 1 (typically Infantry / on-foot). |
| **`Preset 2`** | View distance | Stepper / Slider | `<` [ Grey filled bar ] `>` | View distance preset 2 (typically Ground Vehicles). |
| **`Preset 3`** | View distance | Stepper / Slider | `<` [ Grey filled bar ] `>` | View distance preset 3 (typically Aircraft / flight). |
| **`Sound settings`** | Audio | Section Label | White text | Header for volume & earplug controls. |
| **`Muting`** | Audio | Stepper + Number | `<` [ Track ] `>` `0.01` | Sets audio attenuation factor for earplug mode (`0.01` = 1% volume, 99% reduction). |
| **`Save terrain settings`** | Preferences | Checkbox | **Unchecked** `[ ]` | When enabled, persists view distance settings to client profile across sessions. |
| **`Highlight nickname`** | Preferences | Checkbox | **Checked** `[■]` | Emphasizes the player's own name on overhead nameplates, killfeeds, and rosters. |
| **`Beep after freeze time`**| Preferences | Checkbox | **Unchecked** `[ ]` | Plays an audible alert chime when freeze time / safe start expires and match goes live. |
| **`CLOSE`** | Footer | Action Button | Dark button, white text | Closes the options dialog and commits configured settings. |

---

# Section 2: WOG Admin Suite Global Shell

All 8 administrative screens share a standardized frame and navigation container:

### Header Bar (Gold Banner)
- **Title (Left):** Context title in white text (`WOG Admin Menu: <Subsystem>`).
- **In-Game Clock (Center):** Mission simulation clock in `HH:MM` format (e.g., `11:30`).
- **Server Health (Right):** Real-time server performance indicator (`Server FPS: ###`).
- **Close Button (Far Right):** White `✕` button to dismiss the admin dialog.

### Left Vertical Navigation Rail
An 8-icon sidebar providing one-click switching across admin modules:
1. **Flag Icon:** Mission & Match Control (`admin_home.png`).
2. **Radio / Handset Icon:** Radio & Communications (`admin_radio.png`).
3. **Person in Concentric Rings:** Teleportation & Player Positioning (`admin_teleport.png`).
4. **Speech Bubbles Icon:** Admin Chat & Announcements (`admin_chat.png`).
5. **Open Crate Icon:** Asset & Vehicle Spawner (`admin_spawner*.png`).
6. **Person with Plus Signs:** Medical, Heal & Vehicle Repair (`admin_heal_repair.png`).
7. **Prohibited / Circle-Slash Icon:** Moderation, Kick & Ban (`admin_kick_ban.png`).
8. **Gear & Wrench Icon:** Server Settings & Weather Control (`admin_server_and_other.png`).

---

# Section 3: WOG Admin Menu — Mission & Match Control (`admin_home.png`)

**Image:** `docs/mod/ui/ui_referances/ingame_menu/admin_home.png`  
**Panel Title:** `WOG Admin Menu: Mission`  
**Active Tab:** Flag icon (highlighted in bright green).  
**Context:** Central event management dashboard for mission timers, safe-start rules, faction status, and round termination.

### UI Layout & Placement
- Center column divided into 5 distinct functional panels:
  1. Broadcast Announcement Bar
  2. Granular Mission Phase Timers
  3. Safe-Start (HFT) Toggles & Spectator Camera Shortcuts
  4. Faction Live Headcount & Win Matrix
  5. Mission End Execution & Reason Input

### Control Inventory
| Group | Element / Label | Control Type | State | Purpose & Action |
| :--- | :--- | :--- | :--- | :--- |
| **Broadcast** | `Announcement Text` | Text Input Field | Outlined text box | Entry field for composing a server-wide administrative alert. |
| **Broadcast** | `Make Announcement` | Action Button | Dark button, white text | Transmits the entered text as a prominent on-screen announcement to all players. |
| **Hold Fire Timer** | `01:29` Readout | Monospace Display | Centered box | Displays remaining safe-start / Hold Fire Time (HFT). |
| **Hold Fire Timer** | `[-5 min]`, `[-1 min]` | Quick-Adjust Buttons | Dark slate buttons | Decrements safe-start countdown by 5 or 1 minutes. |
| **Hold Fire Timer** | `[+1 min]`, `[+5 min]` | Quick-Adjust Buttons | Dark slate buttons | Increments safe-start countdown by 1 or 5 minutes. |
| **Round Limit** | `120:00` Readout | Monospace Display | Centered box | Displays total scenario duration limit (in minutes:seconds). |
| **Round Limit** | `[-5 min]`, `[-1 min]` | Quick-Adjust Buttons | Cyan-tinted buttons (`#255872`) | Decrements total round limit by 5 or 1 minutes. |
| **Round Limit** | `[+1 min]`, `[+5 min]` | Quick-Adjust Buttons | Cyan-tinted buttons (`#255872`) | Increments total round limit by 1 or 5 minutes. |
| **Safe-Start Mode** | `HFT ON` | State Toggle | **Active / Selected** (Charcoal) | Enforces safe-start: weapons disabled, damage turned off, movement restricted. |
| **Safe-Start Mode** | `HFT OFF` | State Toggle | Inactive (Dim grey) | Terminates safe-start immediately; enables live combat. |
| **Admin Camera** | Walking Pedestrian Icon | Action Icon Button | White vector icon | Exits spectator free camera and returns admin to their physical character. |
| **Admin Camera** | Crosshair with Cursor Icon | Action Icon Button | White vector icon | Activates click-to-teleport inspection camera at cursor target. |
| **Faction: EAST** | Counter: `0` / Button: `EAST` | Counter + Action Button | Counter dark red (`#4b1f23`); Button bright red (`#b81818`) | Live player count on EAST (OPFOR); button manually declares EAST victory. |
| **Faction: WEST** | Counter: `1` / Button: `WEST` | Counter + Action Button | Counter dark blue (`#1e4a64`); Button steel blue (`#2d779a`) | Live player count on WEST (BLUFOR); button manually declares WEST victory. |
| **Faction: GUER** | Counter: `0` / Button: `GUER` | Counter + Action Button | Counter dark green (`#194d2d`); Button forest green (`#1b5029`) | Live player count on GUER (INDFOR); button manually declares GUER victory. |
| **Faction: CIV** | Counter: `0` / Button: `CIV` | Counter + Action Button | Counter slate (`#5b6470`); Button light platinum (`#c4c9cf`) | Live player count on CIV; button declares neutral/draw outcome. |
| **Termination** | `End Mission Reason` | Text Input Field | Outlined text box | Entry field for the official victory/termination rationale (logged on AAR). |
| **Termination** | `End Mission` | Action Button | **Disabled** (Dim grey) | Commits mission end, locks gameplay, and displays debriefing screens. (Enables when reason/winner is specified). |

---

# Section 4: WOG Admin Menu — Radio & Comms Control (`admin_radio.png`)

**Image:** `docs/mod/ui/ui_referances/ingame_menu/admin_radio.png`  
**Panel Title:** `WOG Admin Menu: Radio`  
**Active Tab:** Radio icon (highlighted in green with tooltip `"Radio"`).  
**Context:** Administrator radio frequency override and directional voice broadcast panel.

### UI Layout & Placement
- **Left Rail:** 8-tab admin dock.
- **Center Column (~35% width):** Player search field and scrollable player list.
- **Right Column (~55% width):** Four stacked rectangular action buttons for voice transmission routing.

### Control Inventory
| Control / Label | Type | Visual State | Purpose & Action |
| :--- | :--- | :--- | :--- |
| **Search Input `🔍`** | Text Input | Outlined text box | Filters the connected player list below by name. |
| **`Mission Maker ★`** | List Item | Light blue text, white star `★` | Connected player entry. Star indicates admin/mission maker. Clicking selects target. |
| **`SR to Player`** | Action Button | Dark rectangular button, white text | Transmits admin voice directly onto the selected player's Short Range (SR) radio net. |
| **`LR to Player`** | Action Button | Dark rectangular button, white text | Transmits admin voice directly onto the selected player's Long Range (LR) backpack radio net. |
| **`SR to All`** | Action Button | Dark rectangular button, white text | Broadcasts admin voice simultaneously across all Short Range radio channels on the server. |
| **`LR to Leaders`** | Action Button | Dark rectangular button, white text | Broadcasts admin voice exclusively across all Long Range Squad Leader and Command channels. |

---

# Section 5: WOG Admin Menu — Teleportation & Positioning (`admin_teleport.png`)

**Image:** `docs/mod/ui/ui_referances/ingame_menu/admin_teleport.png`  
**Panel Title:** `WOG Admin Menu: Teleport`  
**Active Tab:** Person with rings icon (highlighted with tooltip `"Teleport"`).  
**Context:** Player and vehicle relocation, administrative summoning, coordinate teleportation, and invulnerability toggle.

### UI Layout & Placement
- **Three-Column Dual-Transfer Layout:**
  - **Left Column:** Source entity search and selectable player list.
  - **Center Column:** Teleport action buttons, invulnerability toggle, map teleport, and vehicle preservation toggle.
  - **Right Column:** Destination entity search and selectable target list.

### Control Inventory
| Control / Label | Column | Type | Visual State | Purpose & Action |
| :--- | :--- | :--- | :--- | :--- |
| **Search (Source) `🔍`** | Left | Text Input | Outlined text box | Filters candidate entities to be teleported. |
| **Source Player List** | Left | List Box | `Mission Maker ★` | Selects entity/player to be moved. |
| **`Become mortal`** | Center (Top) | Action Button | Dark rectangular button | Invulnerability (godmode) toggle. When immortal, clicking reverts to mortal. Dynamically toggles to `"Become immortal"`. |
| **`>` (Right Chevron)** | Center | Action Button | Dark square button | Teleports the entity selected in the Left List **to** the position of the entity in the Right List (`Source -> Destination`). |
| **`<` (Left Chevron)** | Center | Action Button | Dark square button | Teleports the entity selected in the Right List **to** the position of the entity in the Left List (`Destination -> Source` / "Summon"). |
| **`Map`** | Center | Action Button | Dark rectangular button | Opens the full-screen 2D tactical map to point-and-click teleport self or selected entity. |
| **Vehicle Toggle `(⌖🚚)`** | Center | Icon Toggle | Reticle with truck icon | Vehicle preservation modifier: when enabled, teleports the player's occupied vehicle with them rather than dismounting occupants. |
| **Search (Dest) `🔍`** | Right | Text Input | Outlined text box | Filters candidate destination targets. |
| **Dest Player List** | Right | List Box | `Mission Maker ★` | Selects target entity/player to receive the teleported entity. |

---

# Section 6: WOG Admin Menu — Admin Chat & Messaging (`admin_chat.png`)

**Image:** `docs/mod/ui/ui_referances/ingame_menu/admin_chat.png`  
**Panel Title:** `WOG Admin Menu: Chat`  
**Active Tab:** Speech bubbles icon (highlighted with tooltip `"Chat"`).  
**Context:** Two-way administrative text communication and direct player messaging.

### UI Layout & Placement
- **Left Column:** Recipient search bar and list of connected players/channels.
- **Right Viewport:** Conversation message history pane with vertical scrollbar.
- **Bottom Footer:** Full-width message composer and `Send` button.

### Control Inventory
| Control / Label | Type | Visual State | Purpose & Action |
| :--- | :--- | :--- | :--- |
| **Recipient Search `🔍`** | Text Input | Outlined text box | Filters recipients/channels by name. |
| **Recipient Row** | List Item | `Mission Maker` (Cyan) + `★` | Targets the selected player/channel for direct chat. |
| **Chat History** | Viewport | Dark pane with vertical scrollbar | Displays chronological chat log between admin and selected target. |
| **Message Input Field** | Text Input | Wide dark text field with white frame | Composer for typing outgoing administrative messages. |
| **`Send`** | Action Button | Dark button, muted grey text | Commits and delivers the composed message to the targeted recipient. |

---

# Section 7: WOG Admin Menu — Asset & Vehicle Spawner

**Images:** `admin_spawner.png`, `admin_spawner_1.png`, `admin_spawner_2.png`, `admin_spawner_3.png`  
**Panel Title:** `WOG Admin Menu: Spawn`  
**Active Tab:** Open crate icon (highlighted in bright green with tooltip `"Spawn"`).  
**Context:** Entity, vehicle, static weapon, and supply crate spawning catalog.

### UI Layout & Placement
- **Top-Right Mode Selector:** Segmented toggle buttons (`VEHICLES` / `ITEMS`).
- **Left Column:** Preset search filter and mission-maker category list.
- **Center Column:** Manual classname input field, `Spawn by class name` button, three double-click quick tools, `Clear vehicle cargo` checkbox, and primary `SPAWN` button.
- **Right Column:** Entity search bar and scrollable, faction-color-coded catalog browser.

### Control Inventory & Active Tooltips
| Control / Label | Control Type | Visual State | Verified Behavior & Tooltip |
| :--- | :--- | :--- | :--- |
| **`VEHICLES`** | Segmented Toggle | **Active / Selected** (Bright Green `#237a37`) | Filters catalog to vehicles, armor, air assets, and static weapons. |
| **`ITEMS`** | Segmented Toggle | Inactive (Dark charcoal) | Switches catalog to equipment, weapons, ammo crates, and gear. |
| **Preset Search `🔍`** | Text Input | Outlined text box | Filters categories in the left column. |
| **`Mission Maker ★`** | Preset Row | Cyan text, white star `★` | Filters catalog to mission-maker pre-curated asset lists. |
| **Classname Input Box** | Text Input | Empty text box | Accepts raw Enfusion prefab resource GUIDs or classname strings. |
| **`Spawn by class name`** | Action Button | Dark button, white text | Instantiates and spawns the exact class string entered above. |
| **Medkit Tool `[ + ]`** | Quick Action Button | Rounded square with medical cross | **Tooltip (`admin_spawner_1.png`):** *"Double click to spawn default personal medicine kit"*. Spawns standard medical supplies at player feet. |
| **Cursor Spawn Tool `[ ⌖🚚 ]`** | Quick Action Button | Reticle with truck icon | **Tooltip (`admin_spawner_2.png`):** *"Double click to SPAWN vehicle or item at cursor target"*. Raycasts into 3D world and spawns selected asset at crosshair aim point. |
| **Cursor Delete Tool `[ ⌖🗑️ ]`** | Quick Action Button | Reticle with trash bin icon | **Tooltip (`admin_spawner_3.png`):** *"Double click to DELETE vehicle at cursor target"*. Raycasts into 3D world and deletes the entity under the admin's crosshairs. |
| **`Clear vehicle cargo`** | Checkbox | **Unchecked** `[ ]` | When enabled, strips all default inventory and ammunition from spawned vehicles upon creation. |
| **`SPAWN`** | Primary Action Button | **Disabled** (Dim grey) | Spawns selected asset at default location (enabled when catalog item is selected). |
| **Catalog Search `🔍`** | Text Input | Outlined text box | Filters the vehicle/item browser list in real time. |

### Asset Browser Faction Color Coding
- **BLUFOR (Light Blue / Cyan `#3898EC`):** `M113A3 (MEV)`, `M113A3 (Mk19/Early)`, `M60A1`, `M923 (Covered)`, `M923 (Fuel)`, `M923 (Repair)`, `Mortar 120mm`.
- **OPFOR (Red `#E53935`):** `MT-12 Rapira`, `MT-LB (ZU-23)`, `MT-LB Ambulance`, `MT-LB PKT`, `DShKM`.
- **Civilian / Generic / Crates (White `#FFFFFF`):** `old_bike`, `RHSAFRF Ammo Crate`.

---

# Section 8: WOG Admin Menu — Medical, Heal & Vehicle Repair (`admin_heal_repair.png`)

**Image:** `docs/mod/ui/ui_referances/ingame_menu/admin_heal_repair.png`  
**Panel Title:** `WOG Admin Menu: Repair`  
**Active Tab:** Person with plus signs icon (highlighted with tooltip `"Heal and repair"`).  
**Context:** Comprehensive medical healing, revives, vehicle maintenance, fueling, and physical recovery.

### Dual-Input Architecture (List Selection vs. Cursor Raycasting)
This screen introduces an administrative pattern: rectangular action buttons execute commands on the **entity selected in the left roster list**, while circular crosshair buttons `[⌖]` execute commands directly on the **world entity currently targeted by the admin's crosshairs**.

### Control Inventory
| Control / Label | Target Mode | Type | Purpose & Action |
| :--- | :--- | :--- | :--- |
| **Asset Search `🔍`** | Roster | Text Input | Filters vehicles, static weapons, and crew names. |
| **Asset Roster** | Roster | Two-Column List | Lists tracked vehicles and driver status (e.g., `M60A1 | No driver`, `UH-1H Gunship | No driver`). |
| **Schematic Viewport** | Diagnostics | 2D/3D Wireframe Box | Renders component damage wireframe (hull, engine, tracks, optics, fuel tank). |
| **`Fuel level: - %`** | Fuel | Slider / Stepper (`<` `>`) | Scalar percentage adjustment for vehicle fuel tanks. |
| **`Set fuel level`** | Roster Selection | Action Button | Commits slider fuel percentage to the selected roster vehicle. |
| **`[⌖]` (Fuel Target)** | Crosshair Aim | Icon Button | Commits slider fuel percentage directly to the vehicle aimed at in the 3D world. |
| **`Hitpoint damage: - %`**| Damage | Slider / Stepper (`<` `>`) | Scalar percentage adjustment for vehicle component damage. |
| **`Set hitpoint damage`** | Roster Selection | Action Button | Applies damage percentage across hitpoints of the selected roster vehicle. |
| **`Full heal/repair`** | Roster Selection | Action Button | Instantly repairs all components, refuels, and restores health to 100% on selected roster entity. (If player: heals wounds and revives). |
| **`[⌖]` (Heal Target)** | Crosshair Aim | Icon Button | Instantly full-heals and repairs whatever vehicle or player is targeted by the crosshair. |
| **`Flip vehicle`** | Roster Selection | Action Button | Corrects an overturned or rolled vehicle back onto its wheels/tracks. |
| **`[⌖]` (Flip Target)** | Crosshair Aim | Icon Button | Flips the vehicle targeted by the crosshair in the 3D world. |
| **`Detach vehicle`** | Roster Selection | Action Button | Detaches any towed artillery piece, trailer, or connected cargo entity. |
| **`[⌖]` (Detach Target)**| Crosshair Aim | Icon Button | Detaches connected objects from the vehicle under the crosshair. |
| **`Lock/Unlock vehicle`**| Roster Selection | Action Button | Toggles vehicle door and seat locks (disallowing unauthorized boarding). |
| **`[⌖]` (Lock Target)** | Crosshair Aim | Icon Button | Toggles lock/unlock on the vehicle targeted by the crosshair. |
| **`Respawn player`** | Selected Player | Action Button | Forces an immediate respawn of the selected player or vehicle driver. |

---

# Section 9: WOG Admin Menu — Moderation, Kick & Ban (`admin_kick_ban.png`)

**Image:** `docs/mod/ui/ui_referances/ingame_menu/admin_kick_ban.png`  
**Panel Title:** `WOG Admin Menu: Kick & Ban`  
**Active Tab:** Circle with diagonal slash icon (highlighted with tooltip `"Kick and Ban"`).  
**Context:** Player discipline, administrative warnings, server disconnection, and permanent ban enforcement.

### UI Layout & Placement
- **Left Column (~40% width):** Player search field and selectable player list.
- **Right Column (~60% width):** Warning notification input, dispatch button, and large disciplinary execution buttons.

### Control Inventory
| Control / Label | Type | Visual State | Purpose & Action |
| :--- | :--- | :--- | :--- |
| **Player Search `🔍`** | Text Input | Outlined text box | Filters connected players by nickname. |
| **Player Row** | List Item | `Mission Maker ★` (Cyan) | Selects the player to be targeted for disciplinary actions. |
| **`Notification text`** | Text Input | Outlined text box with placeholder | Entry field for composing a warning message, kick reason, or ban reason. |
| **Dispatch Warning `[🔗]`** | Icon Button | Handcuff / Whistle icon | Dispatches the notification text directly to the selected player's screen as an administrative warning without kicking them. |
| **`KICK`** | Action Button | Muted olive drab green (`#5a6744`), white uppercase text | Immediately disconnects the selected player from the server, transmitting the notification text as the kick reason. |
| **`BAN`** | Action Button | Muted brick red / maroon (`#6f2e32`), white uppercase text | Disconnects the player and permanently registers their Bohemia UID in the server banlist with the notification text as the ban reason. |

---

# Section 10: WOG Admin Menu — Server Lifecycle & Weather/Environment (`admin_server_and_other.png`)

**Image:** `docs/mod/ui/ui_referances/ingame_menu/admin_server_and_other.png`  
**Panel Title:** `WOG Admin Menu: Server`  
**Active Tab:** Gear and wrench icon (highlighted with tooltip `"Server and Other"`).  
**Context:** Dedicated server process management, session authentication, and live environmental simulation controls.

### UI Layout & Placement
- **Center Column:** Stack of 5 full-width primary server lifecycle action buttons.
- **Right Column:** Granular weather and diurnal cycle parameter tuning panel with steppers, input boxes, readouts, and individual commit buttons.

### Control Inventory
| Group | Element / Label | Control Type | Visual State / Value | Purpose & Action |
| :--- | :--- | :--- | :--- | :--- |
| **Server Ops** | **`Lock/Unlock server`** | Toggle Action Button | Dark button, white text | Locks server to prevent new players from joining during active missions, or unlocks it. |
| **Server Ops** | **`Missions list`** | Navigation Button | Dark button, white text | Opens the scenario library browser to queue or switch the active mission. |
| **Server Ops** | **`Restart server after mission ends`** | State Toggle Button | Dark button, white text | Queues an automated, clean server daemon restart upon round completion. |
| **Server Ops** | **`Restart server`** | Action Button | Dark button, white text | Immediately reboots the dedicated server process, terminating active connections. |
| **Server Ops** | **`Logout`** | Session Button | Dark button, white text | Revokes administrative session tokens and exits the admin interface. |
| **Environment**| `12:00` Dropdown + **`Set time of day`** | Time Input + Button | Value: `12:00` with `V` chevron | Sets the island's celestial sun/moon position to the specified hour and minute. |
| **Environment**| `Overcast intensity: 0 %` + **`Set overcast`** | Slider (`<` `>`) + Button | Value: `0 %` | Adjusts cloud density and sky coverage from 0% (clear) to 100% (overcast). |
| **Environment**| `Rain intensity: 0 %` + **`Set rain`** | Slider (`<` `>`) + Button | Value: `0 %` | Adjusts precipitation rate from 0% (dry) to 100% (downpour). |
| **Environment**| `Lightnings intensity: 2 %` + **`Set lightnings`**| Slider (`<` `>`) + Button | Value: `2 %` | Sets the frequency and intensity of lightning strikes and thunder. |
| **Environment**| `Fog intensity: 0 %` | Slider (`<` `>`) | Value: `0 %` | Sets overall fog visual opacity. |
| **Environment**| `Fog density: 0 %` | Slider (`<` `>`) | Value: `0 %` | Sets volumetric fog thickness and falloff curve. |
| **Environment**| `Fog altitude: -1000 m` | Slider (`<` `>`) | Value: `-1000 m` | Sets base elevation boundary (in meters) where fog begins. |
| **Environment**| **`Set fog`** | Action Button | Dark button, white text | Simultaneously commits all three fog parameters (intensity, density, altitude) to the weather engine. |

---

## Technical Summary of UI Principles for TBD Reforger

1. **Safety Interlocks via Double-Click:** Destructive actions (`Cursor Delete`, `Cursor Spawn`, `Quick Medkit`) enforce double-clicks to prevent accidental misclicks during live events.
2. **Dual-Targeting Paradigm:** Admin actions support both macroscopic roster targeting (from list boxes) and rapid point-and-click tactical raycasting (`[⌖]` buttons).
3. **Faction Color Uniformity:** Blue `#3898EC` for BLUFOR/US, Red `#E53935` for OPFOR/USSR, Green `#194d2d` for INDFOR/GUER, and White/Grey for Civilian and Logistics.
4. **State-Adaptive Pause Shell:** Client pause menu dynamically mounts readiness signaling (`MY TEAM IS READY!`) during pre-game freeze and cleans itself up during live combat.
5. **Self-Service Client Recovery:** The persistent `FIX UNIFORM BUG` action mitigates client-side texture and container desync without requiring administrative intervention.
