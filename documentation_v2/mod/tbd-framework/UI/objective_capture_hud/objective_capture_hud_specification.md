# TBD Reforger — Objective & Capture HUD Specification

**System Domain:** In-Game Mission Objectives, Real-Time Capture Progression, Ownership & Contest Telemetry  
**Framework Alignment:** Reforger Enfusion Mod Framework (`apps/mod/tbd-framework`), `TBD_ObjectiveHud` (`Scripts/Game/TBD/UI/Hud/TBD_ObjectiveHud.c`), Layout `{7BD1A70000000A01}` (`UI/layouts/TBD_ObjectiveHud.layout`), and Server Runner (`TBD_ObjectivesComponent.c`)  
**Visual & Theming Standards:** Aegis Design System (`TBD_UITheme.c`)

---

## 1. Purpose & Core Principles

The **Objective & Capture HUD** provides combatants with an in-game, real-time tactical summary of mission objectives and dynamic capture zone status directly within the gameplay viewport during the `LIVE` match stage.

### Primary Objectives
1. **Unobtrusive Combat Awareness:** Deliver immediate clarity on objective status (held, contested, progress %) without obstructing the player's primary engagement and aiming sectors.
2. **Asymmetric / Role-Framed Clarity:** Objectives reflect per-side contextualization (Attacker vs. Defender vs. Neutral framing) derived authoritatively from server-side slot assignments.
3. **Context-Sensitive Immersion:** General objective cards remain visible persistently while deployed, but detailed capture progress bars and contest alerts activate dynamically when the player enters an objective's physical boundary.

---

## 2. Structural Architecture & Layout Anatomy

The HUD is implemented as a lightweight, glass-morphism overlay anchored to the **top-left** of the screen.

```
┌────────────────────────────────────────────────────────┐
│ OBJECTIVES                                             │ <- Title Bar (Font 16, ON_SURFACE)
├────────────────────────────────────────────────────────┤
│ [+] North Zone          OURS                           │ <- Objective Card / Row
│ [!] South Zone          held by OPFOR -- CONTESTED     │ <- Contested Status Row
│ [#] Radar Station       intact 1/2                     │ <- Destroy / Alternate Objective
│                                                        │
├────────────────────────────────────────────────────────┤
│ South Zone  65%                                        │ <- Zone Context Label
│ [████████████████████████████░░░░░░░░░░] [!] CONTESTED │ <- Capture Progress Bar & Warning
└────────────────────────────────────────────────────────┘
```

---

## 3. UI Elements & Data Specifications

### 3.1 Objective Summary Cards / List
Each active mission objective is rendered as a distinct line card within the list:

| Element | Description & Formatting | Examples |
| :--- | :--- | :--- |
| **Status Glyph (`Icon`)** | 1-character bracketed visual tag denoting current state | `[+]` Friendly / Secured<br>`[-]` Hostile / Enemy Held<br>`[o]` Neutral / Uncaptured<br>`[!]` Contested (Both sides present)<br>`[#]` Destroy Target<br>`[H]` Hold Until Clock |
| **Objective Title** | Name of the objective zone, resolved to the player's side | `North Outpost`<br>`Command Bunker`<br>`Ammo Depot` |
| **Ownership Status** | Faction ownership state evaluated against the local viewer | `OURS` (Friendly controlled)<br>`held by OPFOR` (Enemy controlled)<br>`neutral` (Unclaimed) |
| **Task Detail** | Secondary contextual information appended to the row | `intact 1/2` (Destroy)<br>`hold 340s left` (Hold)<br>`hold (PAUSED)` (Hold contested) |

---

### 3.2 Capture Progress Bar & Dynamic Meter
Docked at the base of the HUD panel, the capture progress section is context-sensitive:

* **Trigger Condition:** Activates automatically when the player enters the spatial boundary of a capture zone.
* **Track & Fill:** Slate track with color fill indicating capturing team progress.
* **Numeric Readout:** Real-time text indicator formatted as `<Zone Name>  <Percent>%` (e.g., `North Zone  74%`).

---

### 3.3 Contested Zone Warning Indicator
When opposing factions have living combatants simultaneously present within the capture perimeter:

* **State Trigger:** Multiple opposing factions occupy the zone simultaneously.
* **Capture Freezing:** The capture bar freezes progress immediately (progress does not advance while contested).
* **Visual Cue — List Row:** The objective icon shifts to `[!]` and the text appends `-- CONTESTED`.
* **Visual Cue — Status Bar:** Highlights in warning colors (amber/red) with a flashing alert to signal active enemy presence inside the zone.

---

## 4. Operational Mechanics & Functional Flow

1. **Server-Authoritative Evaluation:** The server ticks objective state at 1 Hz (`TBD_ObjectivesComponent`), checking player zone containment and calculating capture math.
2. **Event-Gated Replication:** RPC updates are sent to clients only when objective state, ownership, or capture progress changes, preventing network spam.
3. **Lifecycle Gating:** The HUD is visible strictly during `TBD_EGameStage.LIVE`. It automatically unloads during `SAFE_START`, `LOBBY`, `BRIEFING`, or match `END`.
