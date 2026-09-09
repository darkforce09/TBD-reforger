# TBD Reforger — Play Area Warning UI Functional Reference Specification

**Target Component:** `TBD_PlayAreaWarning` (In-Game HUD Overlay)  
**Framework Alignment:** Reforger Enfusion Mod Framework (`apps/mod/tbd-framework`), Server Enforcer (`TBD_PlayAreaComponent.c`), Zone Registry (`TBD_ZoneRegistry.c`), and Mission Schema (`packages/tbd-schema/`)  
**Design Standard:** Aegis UI Design System (`TBD_UITheme.c`)

---

## 1. Purpose & Functional Overview

The **Play Area Warning** is an in-game HUD overlay displayed on a client's screen when their controlled character moves outside the mission's active Area of Operations (AO) boundary polygon.

### Tactical Context: One-Life Elimination
TBD Reforger matches operate under strict **One-Life** rules: death is permanent with no respawns. Because out-of-bounds penalties can be terminal (`penalty: "kill"`), boundaries require **instant, escalating visual feedback** to redirect players back into the mission area before punitive elimination occurs.

---

## 2. Visual Design & On-Screen Components

```
┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░│
│░░                                                                                              ░░│
│░░                             ┌───────────────────────────────────┐                            ░░│
│░░                             │   ⚠️  RETURN TO MISSION AREA  ⚠️   │                            ░░│
│░░                             │      OUT OF BOUNDS: [AO_North]    │                            ░░│
│░░                             ├───────────────────────────────────┤                            ░░│
│░░                             │             ⏱️  15s               │                            ░░│
│░░                             │   [███████████████████████░░░░░]  │                            ░░│
│░░                             │   ONE LIFE: ELIMINATION IN 15s    │                            ░░│
│░░                             └───────────────────────────────────┘                            ░░│
│░░                                                                                              ░░│
│░░                              (Unobstructed Combat Viewport)                                  ░░│
│░░                                                                                              ░░│
│░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░│
└──────────────────────────────────────────────────────────────────────────────────────────────────┘
 [◄── Red / Amber Pulsing Vignette Border ──►]
```

### 2.1 Visual Layers
1. **Peripheral Vignette Border:** A colored screen-edge gradient (amber during initial grace; flashing crimson red under 5 seconds). Leaves center screen transparent so players can navigate back.
2. **Warning Banner ("RETURN TO MISSION AREA"):** Centered in the upper viewport with violated zone identification (e.g., `OUT OF BOUNDS: Main Play Area`).
3. **Countdown Timer & Depletion Bar:** Prominent countdown in seconds (e.g. `15s`) with a progress bar draining from 100% to 0%.
4. **Penalty Consequence Text:** Explains consequence clearly: `"ONE LIFE: PERMANENT ELIMINATION IN [N]s"`.

---

## 3. Operational Logic & Rules

1. **Server Detection:** Evaluated server-side at 1 Hz via `TBD_PlayAreaComponent` against the active polygon boundary.
2. **Lifecycle Gating:** Active **only during `LIVE` stage**. Suppressed during `SAFE_START`, `LOBBY`, `BRIEFING`, and `END`.
3. **Instant Dismissal on Return:** The exact frame the player re-enters the valid polygon, the overlay unmounts immediately.
4. **Penalty Execution:** If the countdown reaches `0s`:
   - Under `penalty: "kill"`, the server kills the character (`CharacterDamageManager.Kill()`) and the player transitions into one-life spectator mode.
   - Under `penalty: "warn"`, the warning latches without terminal action.
