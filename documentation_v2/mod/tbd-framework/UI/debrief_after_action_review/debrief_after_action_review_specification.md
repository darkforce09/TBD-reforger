# TBD Reforger — Debrief / AAR Scoreboard UI Functional Reference Specification

**System Domain:** Post-Match Debriefing, Combat Statistics, Server Telemetry Snapshot, AAR Review  
**Framework Alignment:** Reforger Enfusion Mod Framework (`apps/mod/tbd-framework`), Gamemode Lifecycle (`TBD_FrameworkManager.c`), End-of-Round Reporter (`TBD_ResultsReporter.c`), UI Shell & Theme (`TBD_UITheme.c`, `TBD_UILayouts.c`)  
**Stage Lifecycle:** `LOADING → LOBBY → BRIEFING → SAFE_START → LIVE → END → DEBRIEF`

---

## 1. Executive Summary & Purpose

The **Debrief / AAR Scoreboard** screen is the post-match combat debriefing and statistics interface in the TBD Reforger milsim suite. It is automatically presented to all connected players (active combatants, eliminated spectators, and referees) upon entering the `DEBRIEF` game stage following the decisive `END` outcome banner.

### Core Objectives
1. **Authoritative Combat Audit:** Display server-authoritative combat performance records (kills, casualties, friendly fire / teamkills, and objective completion credits) free from client-side dispute.
2. **Squad & Faction Perspective:** Present combat statistics structured by side (BLUFOR vs. OPFOR vs. INDFOR) and grouped by squad/fireteam ORBAT assignments.
3. **Outcome Contextualization:** Clearly display the definitive match outcome (winner, decisive victory trigger, elapsed mission time).
4. **Transition Pacing & Review:** Provide an uninterrupted spectator and client review window governed by a synchronized server countdown timer before recycling back to the `LOBBY` or rotating to the next scheduled scenario.

---

## 2. Structural Architecture & Visual Layout

The Debrief UI operates as an edge-to-edge modal overlay attached to the Enfusion UI Workspace. Rendered against a dark translucent scrim (`TBD_UITheme.SCRIM`, 72% Midnight Navy opacity) over the 3D world view, it organizes telemetry into five coordinated functional sectors:

```
┌───────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ [DEBRIEF & COMBAT REPORT]                                                      MISSION: Operation Red Dawn (34:12)│ <- Header Rail
├───────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
│                                                VICTORY: BLUFOR                                                    │
│                             Decisive Trigger: All Strategic Objectives Secured                                    │ <- Outcome Banner
├───────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ [ ALL FACTIONS ]  [ BLUFOR (US Army) - 24/28 ALIVE ]  [ OPFOR (Armed Forces of RU) - 0/30 ALIVE ]   [ SPLIT VIEW ]│ <- View Switcher
├───────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ SQUAD / PLAYER ROSTER                                       STATUS   KILLS ▲  DEATHS   TEAMKILLS   OBJECTIVES SCORE│
├───────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ ▼ ALPHA 1-1 (RIFLE SQUAD)                                                                                         │
│   • SGT Miller, J.          [Squad Leader] [Linked]          ALIVE       4       0         0           2       680│
│   • CPL Davis, R.           [Combat Medic] [Linked]          ALIVE       1       0         0           1       320│
│   • SPC Jones, T.           [Auto Rifleman] [Linked]         KIA         3       1         0           0       300│
│   • PFC Smith, K.           [Rifleman]                      ALIVE       0       0         1           0       -50│
│                                                                                                                   │
│ ▼ ALPHA 1-2 (WEAPONS SQUAD)                                                                                       │
│   • SSG Vance, D.           [Weapons Leader] [Linked]        ALIVE       2       0         0           1       340│
│   • CPL Kowalski, P.        [Machine Gunner] [Linked]        ALIVE       6       0         0           0       600│
│   • PFC Harris, M.          [Asst Gunner]                   KIA         1       1         0           0       100│
│                                                                                                                   │
│ ▼ BRAVO 1-1 (ARMOR CREW - M2A2)                                                                                   │
│   • 1LT Stone, B.           [Tank Commander] [Linked]        ALIVE       2       0         0           1       320│
│   • SGT Ramos, C.           [Gunner] [Linked]                ALIVE       5       0         0           0       500│
│   • SPC Clark, H.           [Driver] [Linked]                ALIVE       0       0         0           0       100│
├───────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ [CLOSE / SPECTATE WORLD]                           COUNTDOWN TO NEXT MATCH: 01:45           [RETURN TO LOBBY NOW] │ <- Footer Rail
└───────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Screen Layout Breakdown

| Sector | Dimensions & Anchor | Primary Elements | Enfusion Widget Typename |
| :--- | :--- | :--- | :--- |
| **Top Header Rail** | Full width; `Anchor: 0 0 1 0`; `Height: 52px` | Screen title, Mission name, terrain, elapsed combat time | `ImageWidget` (Surface), `TextWidget` |
| **Match Outcome Header** | Center-top banner; `Anchor: 0 0 1 0`; `Height: 88px` | Winning faction tag, Outcome headline, decisive win condition | `ImageWidget` (Highlight card), `TextWidget` |
| **Faction / View Navigation**| Full width sub-bar; `Height: 40px` | Side selector tabs (`ALL`, `BLUFOR`, `OPFOR`), `SPLIT VIEW` toggle | `ButtonWidget`, `HorizontalLayoutWidget` |
| **Scoreboard Roster Table** | Center viewport; `Anchor: 0 0 1 1`; scrollable | ORBAT squad groups, player entries, stat columns | `TBD_ListBox`, `ScrollLayoutWidget`, `TBD_ListRow` |
| **Bottom Control Footer** | Full width; `Anchor: 0 1 1 1`; `Height: 64px` | Back/spectate toggle, server reset countdown, host advance action | `ButtonWidget` (`TBD_UIButton`), `TextWidget` |

---

## 4. Visual & Functional Specifications

### 4.1 Match Outcome Header
- Displays winning faction in canonical color (BLUFOR Blue `#3B82F6`, OPFOR Red `#EF4444`, INDFOR Olive `#22C55E`).
- Clear text summary of decisive win trigger (`all_objectives_captured`, `faction_eliminated`, `time_limit`, `admin`).

### 4.2 Faction Navigation & View Switcher
- Filter tabs allow viewing `ALL COMBATANTS`, `BLUFOR` only, `OPFOR` only, or a side-by-side `SPLIT VIEW`.
- Displays real-time casualty ratios (`Alive / Total Slots`) per faction.

### 4.3 Squad & Player Roster Table
- Hierarchical tree grouped by ORBAT squads (`ALPHA 1-1`, `BRAVO 1-1`).
- Individual row attributes:
  - **Player Identity:** In-game name, clan tags, and Discord identity verification badge (`[Linked]`).
  - **Slot Role:** Specialized role title (`Squad Leader`, `Combat Medic`).
  - **Survival State:** `ALIVE` (green) or `KIA` (red, reflecting permanent one-life elimination).
  - **Metrics:** Confirmed Kills, Deaths (`0` or `1`), Friendly Fire / Teamkills (`TK`), Objective credits, and Total Score.

### 4.4 Column Sorting
- Clicking column headers (`KILLS`, `DEATHS`, `TEAMKILLS`, `OBJECTIVES`, `SCORE`) sorts rows dynamically in client memory.

### 4.5 Footer Controls & Match Lifecycle
- **Spectate World (`[CLOSE / SPECTATE WORLD]`):** Hides scoreboard so players can view the battlefield camera freely while awaiting reset. Pressing `Tab` re-opens it.
- **Server Countdown:** Synchronized countdown (`NEXT MATCH IN: 01:45`, default 120s) before returning to lobby.
- **Host Override (`[RETURN TO LOBBY NOW]`):** Enables admins and referees to bypass the timer and trigger stage transition immediately.
