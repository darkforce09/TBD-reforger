# TBD Reforger — Spectator UI Specification & Visual Breakdown

**Source Reference Directory:** [`spectator_ui/`](./spectator_ui/)  
**Captured References:**
- [`spectator_ui/spectator_death_and_kill_info.png`](./spectator_ui/spectator_death_and_kill_info.png)
- [`spectator_ui/spectator_mines_playercount.png`](./spectator_ui/spectator_mines_playercount.png)  
**System Domain:** One-Life Elimination Camera, Combat Forensics, Faction Attrition Telemetry & Hazard Tracking  
**Framework Alignment:** Reforger Enfusion Mod Framework (`apps/mod/tbd-framework`), Spectator Component (`CRF_SpectatorCamera.c`), and Mission Schema Settings (`packages/tbd-schema/`)

---

## Architecture & Executive Overview

The **Spectator UI** is the post-elimination observation, competitive caster, and referee oversight interface in the one-life tactical milsim suite. When a combatant sustains fatal trauma (`[FatalInjury:Death]`) or when a referee enters observer mode, the client immediately transitions into the spectator subsystem.

The interface is built on a **minimalist, high-clarity 3-tier layering model** designed to preserve an unobstructed view of the battlefield while providing instant situational awareness and forensic ballistics analysis:

```
┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ [TERA]western_fog                                      [FDX]Azmir[LG]                 (Living Unit     │
│ [TERA]teran          (Burning Armor Smoke Plumes)      [LG]Eben                        Nameplates)     │
│                                                                                                        │
│                                            ▲ (Mine 1)          killer: [1xDB]         (3D Spatial      │
│ [KNBM]PANIDA       [1] [FOX]Beast (Truck)   ▲ (Mine 2)         [XOF]Ales               Killer Beacon)  │
│ [OWR] ZDRA                                   ▲ (Mine 3)                                                │
│                                                                Kills: 3                                │
│                                              (ACE3):           gHosT                                   │
│                                           Total Kills: 3       [L-13] Kolin           (Victims List)   │
│                                 Wounded: ace_frag_huge [HF]    [L-13]ProrocK                           │
│                                 Kill: gHosT - [Death]                                                  │
│                                 Kill: [L-13] Kolin - [Death]                                           │
│                                 Kill: [L-13]ProrocK - [Death]                                          │
│                                 Wounded: 5.45x39mm 7N22                                                │
│                                 Killer: [XOF]Ales - [Death]                                            │
├──────────────────────────┬───────┬──────┬──────┬──────────────┬──────┬──────┬──────────────────────────┤
│ [LG]Yuki_Hattori         │ 23:32 │  34  │  28  │ Kills to win:│  20  │  26  │                          │
│ (Target calls / team)    │(Timer)│ (OPF)│(BLU) │ (Cutoff Tag) │(OPF) │(BLU) │  (Unobstructed Margin)   │
└──────────────────────────┴───────┴──────┴──────┴──────────────┴──────┴──────┴──────────────────────────┘
```

### Layering Hierarchy
1. **Layer 0 — 3D World Viewport:** Full-scene in-engine rendering of terrain, structures, vegetation, and atmospheric particle effects (e.g., dense black smoke columns from destroyed armor, muzzle flashes, grenade dust).
2. **Layer 1 — 3D Spatial World Billboards:** Real-time entity markers projected from world coordinates onto the 2D screen:
   - Faction-colored nameplates for all living soldiers.
   - Vehicle occupant tags (`[OccupantCount] [Squad]Player`).
   - 3D spatial killer beacons (`killer: [1xDB] / [XOF]Ales`).
   - Ground-anchored explosive hazard markers (`▲`) for buried anti-tank and anti-personnel mines.
3. **Layer 2 — Center Screen Forensics HUD:** Central, semi-translucent event stream detailing damage, fragmentation, weapon calibers, and combatant kills.
4. **Layer 3 — Bottom Docked Global Telemetry Rail:** Fixed-height status strip anchoring target profile, match clock, team alive headcounts, and dynamic attrition defeat thresholds.

---

# Section 1: Elimination Forensics & Killer Diagnostics

**Source Image:** `docs/mod/ui/ui_referances/spectator_ui/spectator_death_and_kill_info.png`  
**Spectated Player:** `[LG]Yuki_Hattori` (Green / BLUFOR / INDFOR)  
**Killer:** `[XOF]Ales` (Red / OPFOR)  
**Camera View:** High-angle tactical free camera overlooking rolling fields, farmsteads, and crossroads.

---

## 1.1 Forensic Damage Stream & Trauma Audit

Positioned directly at screen center, this typography stream provides an authoritative audit of the combatant's final engagement sequence:

```text
(ACE3):
Total Kills: 3
Wounded: ace_frag_huge from [HF]
Kill: gHosT - [FatalInjury:Death]
Kill: [L-13] Kolin - [FatalInjury:Death]
Kill: [L-13]ProrocK - [FatalInjury:Death]
Wounded: rhs_B_545x39_7N22_Ball from [XOF]Ales
Wounded: rhs_B_545x39_7N22_Ball from [XOF]Ales
Wounded: ace_frag_small HD from [HE]
Wounded: rhs_B_545x39_7N22_Ball from [XOF]Ales
Killer: [XOF]Ales - [FatalInjury:Death]
```

### Sequence Breakdown
1. **Engine Authority Tag:** `(ACE3):` rendered in bold amber/gold (`#FFCC00`), indicating server-side advanced ballistics and medical logging.
2. **Pre-Death Accomplishments:** `Total Kills: 3` — Confirms 3 confirmed kills achieved during this life.
3. **Initial Trauma (Heavy Fragmentation):**
   - `Wounded: ace_frag_huge from [HF]`
   - Player was struck by large high-explosive shrapnel from a heavy artillery or vehicle shell (`[HF]` = Heavy Fragmentation), inflicting major bleeding and pain.
4. **Triple Elimination Spree:**
   - Rapid-fire neutralization of three enemy combatants: `gHosT`, `[L-13] Kolin`, and `[L-13]ProrocK`, each receiving fatal trauma (`[FatalInjury:Death]`).
5. **Direct Ambush & Bullet Penetration:**
   - Consecutive penetrating hits from Russian 5.45x39mm 7N22 armor-piercing ammunition fired by enemy rifleman `[XOF]Ales` (`rhs_B_545x39_7N22_Ball`).
6. **Secondary Fragmentation:**
   - Caught by a high-damage grenade fragment (`ace_frag_small HD from [HE]`).
7. **Terminal Fatal Blow:**
   - Third 7N22 bullet impact from `[XOF]Ales` resulting in immediate cardiac arrest and lethal trauma: `Killer: [XOF]Ales - [FatalInjury:Death]`.

---

## 1.2 3D Spatial Killer Beacon & Victim Billboards

* **3D Killer Spatial Beacon:**
  - Located directly over enemy combatant `[XOF]Ales`.
  - Top Line: `killer: [1xDB]` (Bright yellow text for `killer:`, green tag for `[1xDB]`).
  - Bottom Line: `[XOF]Ales` in bold white.
  - **Purpose:** Instantly resolves "where was I shot from?" by projecting the killer's exact physical concealment point (e.g. hedgerow, tree base, building window).
* **3D Victim Summary Billboard:**
  - Floats adjacent to the engagement sector:
    - Header: `Kills: 3` (amber/gold).
    - Roster: `gHosT`, `[L-13] Kolin`, `[L-13]ProrocK`.
  - Confirms the exact identities of neutralized hostile players.
* **Atmospheric Visual Cues:**
  - Three massive black smoke plumes billowing on the western flank signify destroyed armored vehicles (tanks/IFVs), visually communicating prior armor engagements.

---

# Section 2: Faction Attrition Telemetry & Explosive Hazard Tracking

**Source Image:** `docs/mod/ui/ui_referances/spectator_ui/spectator_mines_playercount.png`  
**Spectated Player:** `[L - 13] Vikhr` (Red / OPFOR)  
**Camera View:** Elevated chase view over a rural road corridor with approaching vehicular traffic.

---

## 2.1 Bottom Telemetry Rail (Global Status Bar)

Docked horizontally along the bottom viewport edge (Y: 95% – 100%), the telemetry ribbon provides continuous strategic data:

```
┌──────────────────────────┬───────┬──────┬──────┬─────────┬──────────────┬──────┬──────┬─────────────────┐
│ [L - 13] Vikhr           │ 35:39 │  31  │  16  │         │ Kills to win:│  11  │  26  │                 │
│ (Active Target - OPFOR)  │(Timer)│(BLU) │(OPF) │ (Spacer)│ (Label)      │(BLU) │(OPF) │ (Clear Margin)  │
└──────────────────────────┴───────┴──────┴──────┴─────────┴──────────────┴──────┴──────┴─────────────────┘
```

### Telemetry Elements Inventory
| Control / Tile | Visual Styling | Displayed Value | Meaning & Behavior |
| :--- | :--- | :--- | :--- |
| **Spectated Target Pill** | Solid Dark Crimson (`#8B0000`) or Solid Green (`#006B1B`), white bold text | `[L - 13] Vikhr` / `[LG]Yuki_Hattori` | Name and squad tag of currently spectated player. Background color dynamically reflects faction (Red for OPFOR, Blue for BLUFOR, Green for INDFOR). |
| **Match Clock** | Solid Black container (`#000000`), white tabular numerals | `35:39` (or `23:32`) | Mission combat chronometer: elapsed round time or time remaining until match expiration. |
| **Faction 1 Alive Count** | Solid Blue (`#0055AA`) or Solid Red (`#A00000`) | `31` (BLUFOR) / `34` (Red OPFOR) | Total living combatants remaining on Faction 1. |
| **Faction 2 Alive Count** | Solid Red (`#B00000`) or Solid Green (`#006B1B`) | `16` (OPFOR) / `28` (Green BLUFOR) | Total living combatants remaining on Faction 2. |
| **Spacer** | Translucent slate void | Empty | Visual breathing space between headcounts and victory calculations. |
| **Threshold Tag** | Light tan / khaki text (`#D8D8B0`) on dark slate | `Kills to win:` | Explanatory header for the dynamic victory casualty thresholds. |
| **Faction 1 Kills to Win** | Solid Blue (`#0055AA`) or Solid Red (`#A00000`) | `11` (BLUFOR) / `20` (Red OPFOR) | Casualties Faction 1 must inflict on Faction 2 to trigger victory by attrition. |
| **Faction 2 Kills to Win** | Solid Red (`#B00000`) or Solid Green (`#006B1B`) | `26` (OPFOR) / `26` (Green BLUFOR) | Casualties Faction 2 must inflict on Faction 1 to trigger victory by attrition. |

---

## 2.2 Mathematical Engine: Dynamic Attrition Victory Thresholds

The framework continuously computes dynamic surrender thresholds:
- **Living Population in `spectator_mines_playercount.png`:**
  - BLUFOR Alive = $31$
  - OPFOR Alive = $16$
  - Total Living Combatants = $47$
- **Defeat Threshold Derivation:**
  $$\text{OPFOR Alive (16)} - \text{BLUFOR Kills to Win (11)} = 5 \text{ surviving combatants}$$
  $$\text{BLUFOR Alive (31)} - \text{OPFOR Kills to Win (26)} = 5 \text{ surviving combatants}$$
- When a faction is reduced to $5$ surviving players, the mission triggers an immediate decisive victory for the opposing side due to combat ineffectiveness.

---

## 2.3 3D Explosive Hazard Tracking (Mines Visualization)

Spectators and referees possess full vision of hidden explosive hazards, whereas live combatants face strict fog of war:

* **In-World Vehicle Tag:**
  - `[1] [FOX]Beast` rendered in bright blue (`#2A7FFF`) over a moving 6x6 camouflaged military cargo truck.
  - `[1]` = vehicle occupant count; `[FOX]` = Foxhound squad/clan; `Beast` = driver callsign.
* **Minefield Hazard Markers:**
  - Three distinct ground-anchored equilateral red triangles (`▲`):
    - **Mine 1 (Left Verge):** Laid on the gravel shoulder near a dirt road junction to catch off-road turning vehicles.
    - **Mine 2 (Center Lane):** Positioned directly on the asphalt center line.
    - **Mine 3 (Right Verge):** Laid in the grassy right shoulder to prevent evasive maneuvers.
* **Spectator Drama & Referee Oversight:**
  - Creates intense suspense as spectators observe the vehicle approaching the 3-mine belt at high speed.
  - Allows referees to audit minefield placement compliance (e.g. verifying mines are not laid within illegal safezone boundaries).

---

# Section 3: Interactive Controls, Camera Modes & Keybinds

| Action / Feature | Keybind / Input | Operational Mechanics |
| :--- | :--- | :--- |
| **Planar Camera Translation** | `W`, `A`, `S`, `D` | Smoothly translates the camera forward, left, backward, and right across the terrain plane. |
| **Vertical Elevation** | `Q` / `Z` (or `Space` / `C`) | Elevates camera for bird's-eye strategic view or descends for ground-level inspection. |
| **Pitch & Yaw Rotation** | `Mouse Move` (RMB Hold) | Controls 360° heading and vertical pitch angle (-89° to +89°). |
| **Speed Multipliers** | `Shift` (Boost) / `Alt` (Slow) | Scales camera traversal speed (e.g., 5 m/s slow inspect -> 50 m/s boost across 4km² terrain). |
| **Cycle Target (Next / Prev)** | `Left Arrow` / `Right Arrow` (or `LMB` / `RMB`) | Cycles spectated player to the next living squadmate or combatant. |
| **Detach Camera (Free Cam)** | `F` | Unlinks camera tether from the focused soldier, anchoring free camera at current coordinates. |
| **Camera Mode Toggle** | `V` / `Enter` (Numpad) | Cycles through **1st-Person POV**, **3rd-Person Orbit Follow**, and **Free Tactical Camera**. |
| **Toggle 3D Nameplates** | `U` / `H` | Toggles rendering of floating 3D player callsigns and distance badges. |
| **Toggle UI / Cinematic Mode** | `Ctrl + H` / `Backspace` | Hides all 2D screen overlays (forensics log, bottom status rail) for clean streaming/recording. |
| **Tactical 2D Map Overlay** | `M` | Opens full-screen 2D top-down map with real-time troop vectors and objective perimeters. |
| **Vision Enhancement (NVG/FLIR)**| `N` | Cycles Night Vision (NVG) and White-Hot/Black-Hot thermal imaging for night scenarios. |

---

# Section 4: Operational Integrity & Mod Architecture

### 4.1 Anti-Ghosting & Voice Net Isolation
1. **Radio Revocation:** Upon death, the voice engine (ACRE2 / TFAR / Enfusion VON) automatically revokes access to living squad and command radio nets.
2. **Dead Chat Routing:** The player is placed into an isolated spectator voice channel where eliminated players can speak without leaking enemy positions.
3. **Spectator Policy Contract (`settings.spectatorPolicy`):**
   - `none`: Screen fades to black with text-only dead chat (hardcore competitive rules).
   - `own_side_delayed_60s`: Follow-cam locked strictly to own faction with a 60-second broadcast delay to prevent live callouts.
   - `free`: Unconstrained free camera across all factions (standard for community events, admin oversight, and live casting).

### 4.2 Implementation in Reforger (`apps/mod/tbd-framework`)
- **Spectator Camera Component:** Managed by `CRF_SpectatorCamera.c` / `TBD_SpectatorComponent.c` attaching to `TBD_SpectatorHost.c`.
- **Damage Capture Pipeline:** Hooks into Enfusion's `SCR_CharacterDamageManagerComponent` to log ammunition classes (`rhs_B_545x39_7N22_Ball`), fragmentation instances, and instigator character UIDs.
- **HUD Layout Integration:**
  - Bottom status rail implemented via `HorizontalLayoutWidget` in `BottomHUD.layout`.
  - 3D nameplates and hazard markers projected via `GetGame().GetWorkspace().ProjWorldToScreen()`.
