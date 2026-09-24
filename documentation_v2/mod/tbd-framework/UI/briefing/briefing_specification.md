# TBD Reforger — Briefing UI Specification & Visual Breakdown

**Source Reference Directory:** [`briefing_ui/`](./briefing_ui/)  
**Scope:** Complete textual representation, visual hierarchy, control inventory, data tables, and operational mechanics for all 16 reference captures of the pre-match Briefing and Tactical Planning interface.

---

## Architecture Overview

The Briefing interface serves as the primary tactical orientation, coordination, and asset verification suite during the pre-match freeze phase (and remains accessible on demand via the tactical map during live gameplay). It utilizes a **3-tier nested master-detail architecture** floating as a semi-transparent HUD overlay above the 2D topographical tactical map canvas:

```
┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ <  wog_180_new_dawn_14                                                                 (Header Bar)   │
├─────────────────┬───────────────────────────────┬──────────────────────────────────────────────────────┤
│ Tier 1: System  │ Tier 2: Briefing Topic Tree   │ Tier 3: Content Viewport & Detail Inspector          │
│                 │ ----------------------------- │                                                      │
│  [Map]          │  [red]  Атака (Attack PID)    │  Header: Topic Title                                 │
│  [Briefing]*    │  [blue] Оборона (Defend PID)  │  Timestamp: In-Game Clock (e.g., Thu, Apr 10, 7:00)  │
│  [Players]      │ ----------------------------- │                                                      │
│  [SWT Markers]  │  [icon] Задачи (Tasks)        │  Scrollable Data Area:                               │
│  [CBA]          │  [icon] Условия (Rules)       │  - Formatted text & coordinate hyperlinks            │
│  [Radio]        │  [icon] Вводная (Lore)        │  - Vehicle silhouette cards & cargo manifests        │
│                 │ ----------------------------- │  - Full faction ORBAT with role badges               │
│                 │  [icon] Frequencies           │  - 2D weapon / gear paper-doll previews              │
│                 │  [icon] Vehicles              │  - Numerical parameter tables                        │
│                 │         Vehicle Inventory     │                                                      │
│                 │  [icon] Enemy vehicles        │                                                      │
│                 │         Squads                │                                                      │
│                 │  [icon] My Squad (Alpha 1-1)  │                                                      │
│                 │         Tasks parameters      │                                                      │
│                 │         Mission parameters    │                                                      │
└─────────────────┴───────────────────────────────┴──────────────────────────────────────────────────────┘
```

### 3-Tier Navigation Hierarchy
1. **Tier 1: Global System Navigation (Column 1):** Leftmost vertical dock providing instant switching between the raw tactical map, the mission briefing suite, player rosters, tactical marker logs, CBA settings, and radio hardware dialogs.
2. **Tier 2: Briefing Topic Tree (Column 2):** Categorized sub-navigation directory divided by dashed horizontal separators into three functional groups:
   - **Faction Identity & Visual PID:** Attacker (`Атака`) and Defender (`Оборона`) uniforms and camo patterns.
   - **Operational Directives:** Objectives (`Задачи`), Rules of Engagement & Boundaries (`Условия`), and Narrative Background (`Вводная`).
   - **Tactical Intel, Assets & Comms:** Radio frequency plan, friendly vehicles, internal vehicle cargo, enemy threat intel, full ORBAT squad lists, personal squad loadouts, task mechanics, and global match parameters.
3. **Tier 3: Content Viewport & Inspector Panel (Column 3):** Semi-opaque card rendering the active topic's detailed data, character/vehicle sprites, inventory grids, or interactive coordinates.

---

# Section 1: Navigation Shell & Root Tabs

## 1.1 Briefing Menu Shell (`briefing_menu.png`)

**Source Image:** `docs/mod/ui/ui_referances/briefing_ui/briefing_menu.png`  
**Panel Title:** Root Shell View  
**Active Tab:** `Briefing` (Default State)  
**Mission Identifier:** `wog_180_new_dawn_14`  

### UI Layout & Placement
- **Top Header Strip:** Full-width dark charcoal banner (`#181818` / ~95% opacity) containing the navigation exit button `<` and scenario identifier string `wog_180_new_dawn_14`.
- **Primary Dock (Column 1):** Docked top-left, ~12% viewport width. Dark slate container (`#202020` @ ~85% opacity) housing the 6 core framework tabs.
- **Secondary Sub-Nav Dock (Column 2):** Docked directly adjacent to Column 1, ~22% viewport width. Categorized directory of all 11 briefing pages.
- **Content Area:** When no topic is clicked, Column 3 remains unmounted or displays default operational directives, leaving the tactical map visible beneath.

### Control Inventory: Global Top & Primary Navigation
| Control / Label | Element Type | Visual State | Purpose & Action |
| :--- | :--- | :--- | :--- |
| **`<` (Back Chevron)** | Navigation Button | Idle / White outline | Returns to the previous game state, collapses briefing overlay to tactical map, or returns to slotting screen. |
| **`wog_180_new_dawn_14`** | Text Label | Static / Plain white | Active mission scenario ID (`[Framework]_[Slots]_[Map]_[Version]`). |
| **`Map`** | Menu Tab | Inactive / Plain white text | Unfocuses menu overlays and switches to the unobstructed full-screen tactical 2D map. |
| **`Briefing`** | Menu Tab | **Active / Focused** (Light-gray background `#505050`) | Opens the multi-tier briefing topic tree (Column 2). |
| **`Players`** | Menu Tab | Inactive / Plain white text | Opens connected player list, faction side breakdown, and connection telemetry. |
| **`SWT Markers`** | Menu Tab | Inactive / Plain white text | Opens Sweet Markers settings, channels, and marker audit log. |
| **`CBA`** | Menu Tab | Inactive / Plain white text | Opens Community Base Addons keybinding and client options overlay. |
| **`Radio`** | Menu Tab | Inactive / Plain white text | Opens the player's personal radio interface (ACRE2 / TFAR dialog). |

---

## 1.2 Connected Players Roster & Network Diagnostics (`briefing_player_list.png`)

**Source Image:** `docs/mod/ui/ui_referances/briefing_ui/briefing_player_list.png`  
**Active Primary Tab:** `Players`  
**Active Selected Item:** `Mission Maker`  

### UI Layout & Visual Structure
- When the `Players` tab is selected in Column 1, Column 2 transitions into a **Connected Player Roster**, and Column 3 opens the **Player Telemetry & Network Diagnostics Inspector**.
- **Background:** Tactical map intersected by thin red coordinate crosshairs (`#b22222`).

### Control & Element Inventory
| Control / Label | Element Type | Visual State | Purpose & Action |
| :--- | :--- | :--- | :--- |
| **`Players:`** | Category Header | Static off-white text | Category title for connected player entries. |
| **`🔊` (Speaker Icon)** | Audio Indicator / Action | Light-gray speaker symbol | Indicates voice transmission activity or serves as a global mute/audio toggle. |
| **`■ Mission Maker`** | Selectable List Item | **Active / Selected** (Light-gray highlight bar, blue side marker) | Selects player profile to inspect. Displays real-time network performance card on the right. |
| **Faction Pip (`■`)** | Colored Indicator | Blue (BLUFOR) | Dynamically indicates faction affiliation (Blue = BLUFOR, Red = OPFOR, Green = Independent). |

### Player Network Diagnostics Card Breakdown
- **Target Header:** `Mission Maker` (Large bold white header).
- **`Ping:` `0   0   0`:** Displays minimum, average, and maximum round-trip packet latency in milliseconds (`ms`). *(All 0 on local loopback/listen server).*
- **`Bandwidth:` `16.78 gbit/s  16.78 gbit/s  16.78 gbit/s`:** Displays downstream, upstream, and peak socket bandwidth allocation.
- **`Desync:` `0`:** Network simulation frame desync counter. A non-zero value alerts administrators and squad leaders to packet loss or clock skew.

---

## 1.3 Tactical Markers Audit Log (`briefing_marker_log.png`)

**Source Image:** `docs/mod/ui/ui_referances/briefing_ui/briefing_marker_log.png`  
**Active Primary Tab:** `SWT Markers`  
**Screen Title:** `Markers log`  

### UI Layout & Operational Context
- Displays the chronological audit trail for all Sweet Tactical Markers (SWT) placed, edited, or deleted on the 2D map.
- Essential for competitive milsim refereeing to identify marker spam, coordinate leaks across side channels, or unauthorized intel drawing.

### Marker Log Data Table
| Timestamp | Channel Tag | Author Callsign | Action / Event | Marker Description & Text |
| :--- | :--- | :--- | :--- | :--- |
| **`07:00:26`** | `[S]` (Side) | `Mission Maker` | Placed | `dot: start` |
| **`07:00:28`** | `[G]` (Global) | `Mission Maker` | Placed | `dot: start 2` |
| **`07:00:30`** | `[V]` (Vehicle) | `Mission Maker` | Placed | `dot: start 3` |
| **`07:00:33`** | `[C]` (Command) | `Mission Maker` | Placed | `dot: start 4` |
| **`07:00:35`** | `[G]` (Group) | `Mission Maker` | Placed | `dot: start 5` |
| **`07:00:39`** | `[D]` (Direct) | `Mission Maker` | Placed | `dot: start 6` |

### Channel Scope Indicators
- **`[S]` Side Channel:** Visible to all friendly units within the faction.
- **`[G]` Global Channel:** Visible to all participants (administrators/referees).
- **`[V]` Vehicle Channel:** Visible exclusively to occupants of the author's vehicle.
- **`[C]` Command Channel:** Visible only to platoon leaders and squad leaders equipped with command radios.
- **`[G]` Group Channel:** Scoped to the author's immediate infantry squad / fireteam.
- **`[D]` Direct Channel:** Scoped to local immediate proximity.

---

# Section 2: Narrative & Mission Directives

## 2.1 Mission Tasks & Objective Coordinates (`briefing_tasks.png`)

**Source Image:** `docs/mod/ui/ui_referances/briefing_ui/briefing_tasks.png`  
**Active Sub-Navigation:** `Задачи` (Tasks)  
**Main Panel Header:** `Задачи`  
**Timestamp:** `Thu, Apr 10, 7:00`  

### UI Layout & Interactive Hyperlink Mechanics
- Displays tactical directives with inline **interactive coordinate anchors** indicated by bullet points (`•`).
- Clicking an objective bullet centers and focuses the 2D tactical map view directly onto the objective's target grid without leaving the briefing overlay.

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ Задачи                                                                                 │
│ Thu, Apr 10, 7:00                                                                      │
│                                                                                        │
│ Захватить две зоны:                                                                    │
│  • Южная [Interactive Map Anchor -> Centers map on Southern Zone]                      │
│  • Северная [Interactive Map Anchor -> Centers map on Northern Zone]                    │
│                                                                                        │
│ Время на захват зон - 60 минут                                                         │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

### Directive Content Breakdown
1. **Primary Goal:** `Захватить две зоны:` *(Capture two zones:)*
   - **`• Южная`** *(Southern Zone):* Interactive blue/white text link triggering map pan to southern objective boundary.
   - **`• Северная`** *(Northern Zone):* Interactive blue/white text link triggering map pan to northern objective boundary.
2. **Time Limit:** `Время на захват зон - 60 минут` *(Time to capture zones: 60 minutes).*

---

## 2.2 Operational Conditions & Rules of Engagement (`briefing_conditions_rules.png`)

**Source Image:** `docs/mod/ui/ui_referances/briefing_ui/briefing_conditions_rules.png`  
**Active Sub-Navigation:** `Условия` (Conditions / Rules)  
**Main Panel Header:** `Условия`  
**Timestamp:** `Thu, Apr 10, 7:00`  

### UI Content Breakdown: Match Rules & Restrictions
The screen provides mission-specific constraints, vehicle operation restrictions, capture logic, and role limitations:

1. **General Win/Loss Conditions:**
   - `Для победы Атаки необходимо захватить обе зоны.` *(Attackers must capture both zones for victory.)*
   - `Для победы Обороны необходимо удержать хотя бы одну из зон.` *(Defenders win by holding at least one zone.)*
2. **Crew & Role Authorizations:**
   - `Экипаж может брать только инженер/экипаж.` *(Only designated Engineers / Crewmen can crew armored vehicles.)*
   - Prevents unauthorized infantry from operating heavy combat armor.
3. **Capture Zone Logistics & Restrictions:**
   - `Запрещено строить укрепления ближе 50 метров к флагштоку.` *(Fortifications prohibited within 50m of flagpoles.)*
   - `Захват зон начинается через 5 минут после старта.` *(Zone capture opens 5 minutes after freeze drops).*
4. **Logistics & Medical Deployments:**
   - `Мобильный госпиталь (GAZ-66) разворачивается не ближе 300м к зонам.` *(Mobile medical truck deployment boundary).*
   - `Время на подготовку (фризтайм) - 3 минуты.` *(Staging freeze time duration: 3 minutes).*

---

## 2.3 Scenario Lore & Operational Situation (`briefing_lore.png`)

**Source Image:** `docs/mod/ui/ui_referances/briefing_ui/briefing_lore.png`  
**Active Sub-Navigation:** `Вводная` (Situation / Lore)  
**Main Panel Header:** `Вводная`  
**Timestamp:** `Thu, Apr 10, 7:00`  

### UI Content Breakdown: Narrative Background
- High-contrast narrative text detailing operational intelligence and political context.
- **Narrative Synopsis:**
  - Setting: Early morning hours, South Zagoria region, Chernarus.
  - Operation: Joint Russian Airborne Forces (VDV) and ChDKZ separatist elements launching a dawn mechanized offensive against fortified Chernarus Defence Forces (CDF) 12th Mechanized Brigade positions.
  - Tactical Environment: Overcast dawn conditions, degraded radio communications in mountainous terrain, strategic bridges pre-sighted by artillery.

---

# Section 3: Force Identification & Uniform References

## 3.1 Attacker Uniform & Camouflage PID (`briefing_uniforms.png`)

**Source Image:** `docs/mod/ui/ui_referances/briefing_ui/briefing_uniforms.png`  
**Active Sub-Navigation:** `Атака` (Attack — Uniform Guide)  
**Main Panel Header:** `Форма Атаки` (Attacker Uniforms)  
**Timestamp:** `Thu, Apr 10, 7:00`  

### UI Layout & Visual Presentation
- 2-column or 3-column side-by-side character display showing full-length front-profile renders of attacking combatants.
- High-contrast character paper-doll cutouts against the dark semi-transparent briefing card.

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ Форма Атаки                                                                            │
│ Thu, Apr 10, 7:00                                                                      │
│                                                                                        │
│   ┌──────────────────────────┐             ┌──────────────────────────┐                │
│   │        [Character]       │             │        [Character]       │                │
│   │         Full Body        │             │         Full Body        │                │
│   │          Render          │             │          Render          │                │
│   │                          │             │                          │                │
│   └──────────────────────────┘             └──────────────────────────┘                │
│        ВДВ РФ (Флора / Берет)                   ЧДКЗ (Горка / Патизан)                 │
│      Russian VDV Regulars                      ChDKZ Insurgents / Irregulars           │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

### Identification Elements Displayed
1. **Primary Camouflage Pattern:** Russian VSR-93 / Flora woodland pattern field uniform and load-bearing vest.
2. **Headgear & Silhouette:** Blue airborne beret / SSh-68 steel helmet with flora cover for VDV regular infantry; civilian knit beanies / patrol caps / Gorka hoods for ChDKZ irregulars.
3. **Vest & Webbing:** 6B5 / Lifchik chest rigs with AK magazine pouches.
4. **Role Notes:** Warns friendly forces against friendly fire due to mixed regular/irregular equipment profiles on the offensive side.

---

## 3.2 Defender Uniform & Camouflage PID (`briefing_uniforms_2.png`)

**Source Image:** `docs/mod/ui/ui_referances/briefing_ui/briefing_uniforms_2.png`  
**Active Sub-Navigation:** `Оборона` (Defense — Uniform Guide)  
**Main Panel Header:** `Форма Обороны` (Defender Uniforms)  
**Timestamp:** `Thu, Apr 10, 7:00`  

### UI Layout & Visual Presentation
- Full-length front-profile renders of defending faction soldiers (Chernarus Defence Forces — CDF).

### Identification Elements Displayed
1. **Primary Camouflage Pattern:** TTsKO / "Dubok" Ukrainian-Chernarussian woodland camouflage fatigues.
2. **Headgear & Silhouette:** PASGT-style ballistic helmets with TTsKO covers and olive drab patrol caps.
3. **Vest & Loadout:** Olive drab modular tactical body armor and ALICE-style webbing harnesses.
4. **Distinctive Markings:** Bright national flag patches or high-contrast armbands (yellow/blue identifier) to aid target discrimination at long engagement distances (>300m).

---

# Section 4: Communications & Asset Intelligence

## 4.1 Radio Communications Plan & Squad Frequencies (`briefing_squad_frequencies.png`)

**Source Image:** `docs/mod/ui/ui_referances/briefing_ui/briefing_squad_frequencies.png`  
**Active Sub-Navigation:** `Frequencies` (Green antenna icon)  
**Main Panel Header:** `Frequencies`  
**Timestamp:** `Thu, Apr 10, 7:00`  

### UI Layout & Frequency Allocation Table
- The viewport displays the Long-Range (LR) command network followed by a vertically scrollable list of all Short-Range (SR) squad assignments.
- Color-coded callsigns: **Amber/Gold** (`#DCA13C`) for commanders / HQ; **Light Blue** (`#3B99FC`) for line infantry squads.

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ Frequencies                                                                          ▲ │
│ Thu, Apr 10, 7:00                                                                    █ │
│                                                                                      █ │
│ LR 76.2 MHz  (Aux: 50.6, 35.4, 71.3 MHz)                                             █ │
│                                                                                      █ │
│ A1-1 Mission Maker                                                                   █ │
│   390.7 MHz  (Aux: 133.3, 276.9, 384 MHz)                                            █ │
│ A1-2                                                                                 █ │
│   163.7 MHz  (Aux: 382.1, 346.4, 250.9 MHz)                                            █ │
│ A1-3                                                                                 █ │
│   263.5 MHz  (Aux: 260.7, 318.7, 127.2 MHz)                                            █ │
│ A1-4                                                                                 ▼ │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

### Full Communications Matrix
| Net / Callsign | Visual Color | Role / Description | Primary Frequency | Auxiliary (Backup) Channels |
| :--- | :--- | :--- | :--- | :--- |
| **`LR Command`** | White bold | Platoon / Company Command Net | **`76.2 MHz`** | `50.6, 35.4, 71.3 MHz` |
| **`A1-1 Mission Maker`** | **Amber / Gold** | Platoon HQ / Squad 1 Lead | **`390.7 MHz`** | `133.3, 276.9, 384.0 MHz` |
| **`A1-2`** | Light Blue | 1st Platoon, 2nd Squad | **`163.7 MHz`** | `382.1, 346.4, 250.9 MHz` |
| **`A1-3`** | Light Blue | 1st Platoon, 3rd Squad | **`263.5 MHz`** | `260.7, 318.7, 127.2 MHz` |
| **`A1-4`** | Light Blue | 1st Platoon, 4th Squad | **`273.8 MHz`** | `319.1, 334.3, 441.3 MHz` |
| **`A2-1`** | Light Blue | 2nd Platoon, 1st Squad | **`499.5 MHz`** | `227.4, 279.1, 391.1 MHz` |
| **`A2-2`** | Light Blue | 2nd Platoon, 2nd Squad | **`388.8 MHz`** | `457.1, 461.7, 366.1 MHz` |
| **`A2-3`** | Light Blue | 2nd Platoon, 3rd Squad | **`494.1 MHz`** | `214.7, 187.8, 443.2 MHz` |
| **`A2-4`** | Light Blue | 2nd Platoon, 4th Squad | **`327.6 MHz`** | `277.7, 233.5, 122.8 MHz` |
| **`A3-1`** | Light Blue | 3rd Platoon, 1st Squad | **`288.4 MHz`** | `225.1, 326.3, 199.9 MHz` |
| **`A3-2`** | Light Blue | 3rd Platoon, 2nd Squad | **`222.7 MHz`** | `353.2, 237.4, 354.9 MHz` |
| **`A3-3`** | Light Blue | 3rd Platoon, 3rd Squad | **`295.6 MHz`** | `436.2, 136.3, 228.7 MHz` |
| **`A3-4`** | Light Blue | 3rd Platoon, 4th Squad | **`389.3 MHz`** | `221.4, 404.7, 339.5 MHz` |
| **`A4-1`** | Light Blue | 4th Platoon, 1st Squad | **`413.2 MHz`** | `119.4, 218.3, 154.9 MHz` |
| **`A4-2`** | Light Blue | 4th Platoon, 2nd Squad | **`426.4 MHz`** | `187.9, 206.8, 371.2 MHz` |
| **`A5-1`** | Light Blue | 5th Platoon, Support / Weapons | *[Scroll down]* | *[Scroll down]* |

### Design Takeaways for TBD Reforger
- **Automatic Provisioning:** Prevents manual tuning during safe start. Frequencies are pseudo-randomly spaced across UHF bands (`100–500 MHz`) to avoid channel bleeds.
- **Tri-Auxiliary Channels:** 3 pre-assigned fallback channels per net allow rapid response to electronic warfare, enemy radio interception, or jamming.

---

## 4.2 Friendly Vehicle Manifest (`briefing_friendly_assets.png`)

**Source Image:** `docs/mod/ui/ui_referances/briefing_ui/briefing_friendly_assets.png`  
**Active Sub-Navigation:** `Vehicles` (Blue vehicle icon)  
**Main Panel Header:** `Vehicles`  
**Timestamp:** `Thu, Apr 10, 7:00`  

### UI Layout & Friendly Asset Roster
- Displays friendly motorized transport, armored fighting vehicles, logistics trucks, and support assets.
- Consistent typography: White silhouette sprite + vehicle designation + hyphen `-` + count in amber/gold (`#E59400`).

### Friendly Roster Breakdown
| Silhouette Type | Exact Designation String | Unit Count | Role & Armament |
| :--- | :--- | :---: | :--- |
| Light Utility 4x4 | `UAZ-3151- 2` | `2` | Reconnaissance & Platoon Leader Transport |
| Medium 4x4 Truck | `GAZ-66- 2` | `2` | Squad Infantry Soft-Skin Transport |
| Heavy 6x6 Truck | `Ural-4320- 3` | `3` | Platoon Transport & Heavy Logistics |
| Armored Recon Car | `BRDM-2- 1` | `1` | Scout Car with 14.5mm KPVT Heavy Machine Gun |
| Wheeled APC | `BTR-80- 4` | `4` | Amphibious Wheeled Armored Personnel Carrier (14.5mm) |
| Infantry Fighting Vehicle | `BMP-2- 2` | `2` | Tracked IFV with 30mm 2A42 Autocannon & 9M113 Konkurs ATGM |
| Main Battle Tank | `T-72B- 1` | `1` | Heavy Armor Support (125mm Smoothbore Gun) |

---

## 4.3 Vehicle Cargo Inventory & Equipment Packs (`briefiing_vehicle_inventory.png`)

**Source Image:** `docs/mod/ui/ui_referances/briefing_ui/briefiing_vehicle_inventory.png`  
**Active Sub-Navigation:** `Vehicle Inventory`  
**Main Panel Header:** `Vehicle Inventory`  
**Timestamp:** `Thu, Apr 10, 7:00`  

### UI Layout & Cargo Card Representation
- Displays internal cargo manifests container-by-container or vehicle-by-vehicle.
- Each vehicle class card features an item grid with side-profile 2D sprites and count badges (`xN`).

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ Vehicle Inventory                                                                      │
│ Thu, Apr 10, 7:00                                                                      │
│                                                                                        │
│ BMP-2 (x2)                                                                             │
│ ┌────────────────────────────────────────────────────────────────────────────────────┐ │
│ │ [Ammo Box x2]  [Tool Kit x1]  [Surgical Kit x1]  [Blood Bag x4]                    │ │
│ │ [AK-74 Mag x30] [PKM Belt x8] [RPG-7 Rocket x6]  [Smoke Grenade x12]               │ │
│ └────────────────────────────────────────────────────────────────────────────────────┘ │
│                                                                                        │
│ Ural-4320 (Ammo Truck)                                                                 │
│ ┌────────────────────────────────────────────────────────────────────────────────────┐ │
│ │ [Resupply Crate x4] [Spare Wheels x6] [Medical Crate x2]                           │ │
│ └────────────────────────────────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

### Critical Cargo Elements Displayed
1. **Medical Supplies:** Field dressing badges, tourniquets, epinephrine auto-injectors, and 500ml blood bags allocated for squad combat life savers.
2. **Heavy Munitions:** Spare RPG-7 / RPG-26 rockets, PG-7VL HEAT warheads, PKM/PKP 100-round ammunition belts, and 5.45x39mm rifle magazines.
3. **Engineering Tools:** Vehicle repair toolkits (`Tool Kit x1`), spare wheels, and entrenching tools.

---

## 4.4 Enemy Assets & Heavy Weapons Intelligence (`briefing_enemy_assets.png`)

**Source Image:** `docs/mod/ui/ui_referances/briefing_ui/briefing_enemy_assets.png`  
**Active Sub-Navigation:** `Enemy vehicles` (Red vehicle icon)  
**Main Panel Header:** `Enemy vehicles`  
**Timestamp:** `Thu, Apr 10, 7:00`  

### UI Layout & Tactical Intelligence Roster
- Main content pane displays **15 distinct enemy asset types** totaling **22 units**.
- Encompasses motorized vehicles, armored combat vehicles, attack aircraft, towed artillery, and crew-served static weapon emplacements.
- Formatting: White silhouette profile icon + designation string + hyphen `-` + count in amber/orange (`#E59400`).

### Full Enemy Asset Intelligence Table
| # | Profile Silhouette | Exact Designation String | Count | Classification & Combat Threat |
| :-: | :--- | :--- | :-: | :--- |
| **1** | Light 4x4 Off-roader | `UAZ-3151- 2` | `2` | Light utility soft-skin transport / commander scout |
| **2** | 4x4 Military Truck | `GAZ-66- 1` | `1` | Soft-skin troop / cargo carrier |
| **3** | 4x4 Shelter Truck with Antenna | `GAZ-66 (R-142N)- 1` | `1` | Command, control & signals mobile shelter vehicle |
| **4** | 6x6 Heavy Logistics Truck | `ZiL-131- 2` | `2` | Heavy cargo / munitions transport |
| **5** | 4x4 Armored Scout Car | `BRDM-2- 1` | `1` | Wheeled amphibious armored reconnaissance (14.5mm KPVT) |
| **6** | 8x8 Wheeled APC | `BTR-60PB- 1` | `1` | Amphibious armored troop transport (14.5mm KPVT) |
| **7** | Tracked Airborne APC | `BTR-D- 3` | `3` | Multi-purpose tracked airborne transporter |
| **8** | Tracked Airborne Command IFV | `BMD-1K- 1` | `1` | Command vehicle with 73mm 2A28 "Grom" smoothbore gun |
| **9** | Tracked Airborne Autocannon IFV | `BMD-2- 3` | `3` | Tracked airborne IFV with 30mm 2A42 autocannon |
| **10** | Main Battle Tank (Base) | `T-72B (obr. 1984g.)- 1` | `1` | 1984 model MBT (composite armor, 125mm 2A46M gun) |
| **11** | Main Battle Tank (ERA) | `T-72B (obr. 1985g.)- 1` | `1` | 1985 model MBT fitted with Kontakt-1 Explosive Reactive Armor |
| **12** | Attack Helicopter Gunship | `Mi-24V- 1` | `1` | Rotary-wing attack CAS (12.7mm Yak-B, S-8 rockets, Shturm ATGMs) |
| **13** | Towed 12-Tube Rocket Artillery | `Type 63- 1` | `1` | Static / towed 107mm light multiple rocket launcher |
| **14** | Tripod Automatic Grenade Launcher| `AGS-30 (6P17)- 2` | `2` | Static crew-served 30mm automatic grenade launcher |
| **15** | Tripod Anti-Tank Guided Missile | `9K111 'Fagot'- 1` | `1` | Static crew-served wire-guided ATGM system (AT-4 Spigot) |

---

# Section 5: Order of Battle (ORBAT) & Loadout Inspection

## 5.1 Friendly Faction Squad Roster & Hierarchy (`briefing_friendly_squad_list.png`)

**Source Image:** `docs/mod/ui/ui_referances/briefing_ui/briefing_friendly_squad_list.png`  
**Active Sub-Navigation:** `Squads`  
**Main Panel Header:** `Squads`  
**Timestamp:** `Thu, Apr 10, 7:00`  

### UI Layout & ORBAT Tree Structure
- Displays the complete hierarchical Order of Battle for all friendly units across the entire company / battalion.
- **Color Coding:**
  - Platoon / Squad Headers: Bold white or amber (`1-й Взвод`, `1-е Отделение`).
  - Human Connected Players: **Amber / Gold** typography (`Mission Maker`).
  - Unslotted / AI Bots: Marked with **`[AI]`** prefix in amber/gold.
- **Specialization Capabilities Badges:**
  - Specialized qualification roles carry explicit trailing badge tags:
    - **`| Med`** (Medical Specialist / Combat Life Saver): Authorized to carry surgical kits, plasma, and administer advanced trauma treatment.
    - **`| Eng`** (Field Engineer / Mechanic): Authorized to carry heavy repair toolkits, conduct field maintenance, and disarm mine hazards.

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ Squads                                                                                 │
│ Thu, Apr 10, 7:00                                                                      │
│                                                                                        │
│ 1-й Взвод (1st Platoon)                                                                │
│   1-е Отделение (1st Squad - Alpha 1-1)                                                │
│     1. Mission Maker (Командир отделения / SL)                                        │
│     2. [AI] (Старший стрелок / Senior Rifleman)                                        │
│     3. [AI] (Пулеметчик / Machinegunner)                                               │
│     4. [AI] (Помощник пулеметчика / Asst. MG)                                          │
│     5. [AI] (Гранатометчик / Grenadier RPG-7)                                          │
│     6. [AI] (Стрелок-санитар / Medic) | Med                                            │
│     7. [AI] (Сапер-инженер / Engineer) | Eng                                           │
│   2-е Отделение (2nd Squad - Alpha 1-2)                                                │
│     ...                                                                                │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 5.2 Personal Squad Roster & Visual Inventory (`briefing_personal_squad_roster_and_inventory.png`)

**Source Image:** `docs/mod/ui/ui_referances/briefing_ui/briefing_personal_squad_roster_and_inventory.png`  
**Active Sub-Navigation:** `My Squad (Alpha 1-1)` (White 3-man squad icon)  
**Main Panel Header:** `My Squad (Alpha 1-1)`  
**Timestamp:** `Thu, Apr 10, 7:00`  

### UI Layout & Inventory Paper-Doll
- Focuses specifically on the player's immediate squad (`Alpha 1-1`).
- Vertically stacked squad member loadout cards with 2D weapon renders, ammo counters, and wearable gear paper-doll previews.
- Scrollable list via right-side vertical scrollbar (`▲`, `▼`, and proportional slider thumb).

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ My Squad (Alpha 1-1)                                                                 ▲ │
│ Thu, Apr 10, 7:00                                                                    █ │
│                                                                                      █ │
│ 1: Mission Maker (Officer) AKS-74N (Plum)                                            █ │
│   [Rifle Render]  [Plum Mag x4]  [Bakelite Mag x2]  [Torch/Accessory]                 █ │
│   [Map x1]  [Compass x1]  [Watch x1]  [Radio x1]  [DAGR / Binoc x1]                  █ │
│   [Bandage x2]  [F-1 Grenade x2]  [Chemlight x1]                                     │ │
│   [Uniform: Flora]  [Vest: Flora]  [Headgear: Field Cap]  [Backpack: R-107M Radio]   │ │
│                                                                                      │ │
│ 2: [AI] (Combat Life Saver) AKS-74N (Plum)                                           │ │
│   [Rifle Render]  [Plum Mag x4]  [Bakelite Mag x2]                                   │ │
│   [Bandage x8]  [Tourniquet x4]  [Morphine x4]  [Blood Bag x2]                       │ │
│   [Uniform: Flora]  [Vest: Flora]  [Headgear: Helmet]  [Backpack: Medic Pack]        ▼ │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

### Squad Member Loadout Breakdown
| Slot # | Identity & Status | Role Specialization | Primary Weapon | Ammunition & Consumables | Navigation & Specialty Gear | Wearables & Containers |
| :---: | :--- | :--- | :--- | :--- | :--- | :--- |
| **1** | `Mission Maker` (Human) | `Officer` (Squad Leader) | `AKS-74N (Plum)` | 4x Plum Mags, 2x Bakelite Mags, 2x F-1 Grenades, 2x Bandages, 1x Chemlight | Map, Compass, Watch, Radio, **Map Board / DAGR**, **Binoculars** | Flora Field Cap, Flora Vest, **R-107M Long-Range Radio Backpack** |
| **2** | `[AI]` (Bot) | `Combat Life Saver` | `AKS-74N (Plum)` | 4x Plum Mags, 2x Bakelite Mags, 2x F-1 Grenades, 8x Bandages, Medical Kit | Map, Compass, Watch, Radio | Camo Ballistic Helmet, Flora Vest, **Medical Field Backpack** |
| **3** | `[AI]` (Bot) | `Crewman` | `AKS-74N (Plum)` | 4x Plum Mags, 2x Bakelite Mags, Binoculars | Map, Compass, Watch, Radio | Tanker Padded Helmet, Compact Vest |

---

# Section 6: Technical Match Parameters

## 6.1 Task & Capture Zone Parameters (`briefing_task_parameters.png`)

**Source Image:** `docs/mod/ui/ui_referances/briefing_ui/briefing_task_parameters.png`  
**Active Sub-Navigation:** `Tasks parameters`  
**Main Panel Header:** `Tasks parameters`  
**Timestamp:** `Thu, Apr 10, 7:00`  

### UI Content Breakdown: Zone Capture Mechanics
Displays the mathematical and logistical rules governing objective capture:

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ Tasks parameters                                                                       │
│ Thu, Apr 10, 7:00                                                                      │
│                                                                                        │
│ Capture Rules:                                                                         │
│   • Capture Radius:              75 meters                                             │
│   • Minimum Capturing Players:   2 attackers                                           │
│   • Defender Blocking Threshold: 1 defender prevents uncontested capture               │
│   • Advantage Ratio:             2:1 attacker ratio required to advance capture bar    │
│   • Capture Timer Duration:      60 seconds continuous hold                            │
│   • Zone Interruption:           Contesting freezes timer progress (no decay)          │
│   • Decisive Capture:            Permanent; cannot be recaptured once secured          │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

- **Capture Radius:** 75-meter circular zone centered on the objective flag/building.
- **Advantage Thresholds:** Minimum 2 attacking players required; presence of 1 defender halts capture unless attackers maintain a 2:1 numerical superiority.
- **Timer Mechanics:** 60-second continuous hold timer; contesting pauses the clock without instant penalty decay.

---

## 6.2 Global Mission Parameters (`briefing_mission_parameters.png`)

**Source Image:** `docs/mod/ui/ui_referances/briefing_ui/briefing_mission_parameters.png`  
**Active Sub-Navigation:** `Mission parameters`  
**Main Panel Header:** `Mission parameters`  
**Timestamp:** `Thu, Apr 10, 7:00`  

### UI Content Breakdown: Match Settings
Displays global mission simulation rules and technical boundaries:

| Parameter | Value | Operational Effect |
| :--- | :---: | :--- |
| **`View Distance`** | `2500 m` | Maximum visual rendering distance for terrain and vehicles. |
| **`Object View Distance`** | `1800 m` | Maximum visual rendering distance for infantry and foliage. |
| **`Preparation Time`** | `3 min` | Staging freeze time before match start (weapons locked, boundaries enforced). |
| **`Mission Duration`** | `2 hours (120 min)` | Maximum match time limit before automatic draw/defender victory. |
| **`Casualty Defeat Threshold`** | `10%` | If remaining faction strength falls below 10%, the faction suffers decisive loss. |
| **`Thermals (TI)`** | `Disabled` | Thermal optics disabled on vehicle gunsights and handheld scopes. |
| **`Night Vision Devices (NVG)`** | `Disabled` | Night vision goggles disabled; forces reliance on illumination flares and torches. |
| **`Weather / Fog`** | `Overcast / Light Mist`| Atmospheric conditions set for dawn combat. |

---

# Summary of UI Takeaways for TBD Reforger Mod Architecture

1. **Unified Multi-Tier Overlay:** The Briefing UI must operate as a non-blocking modal over the 2D map, allowing simultaneous map inspection, coordinate clicking, and topic reading.
2. **Interactive Map Coordinates:** Objective hyperlinks (e.g., `• Южная`) must emit coordinate click events that instruct the map camera to smoothly pan to the target location.
3. **Automated Radio Network Provisioning:** Comms frequencies should be auto-assigned across UHF/VHF bands with 3 backup channels (`Aux`) per squad to eliminate freeze-time setup errors.
4. **Visual Loadout Verification:** The personal squad roster (`My Squad`) and vehicle cargo views must render 2D weapon profile sprites and item counter badges (`xN`), allowing instant gear audits without opening inventory or spawning.
5. **Specialization Capability Tags:** ORBAT rosters should explicitly display capability badges (`| Med`, `| Eng`) so squad leaders immediately recognize qualified medics and repair engineers.
