# TBD Reforger — Safestart HUD Functional Reference Specification

**Module:** TBD Reforger Enfusion Framework (`apps/mod/tbd-framework`)  
**Domain:** In-Game HUD, Staging Lifecycle, Mission Safety Enforcement  
**Applicable Stage:** `TBD_EGameStage.SAFE_START`  
**Backend & Authority Coupling:** `TBD_SafestartManager.c`, `TBD_PlayAreaComponent.c`, `TBD_AdminService.c`  
**UI Engine & Theme:** Enfusion UI Layout (`.layout`), `ScriptedWidgetComponent`, Aegis Design Tokens (`TBD_UITheme.c`)

---

## 1. Purpose & Core Design Philosophy

### 1.1 The Staging Safeguard Under "One Life"
In the TBD Reforger milsim ecosystem, events adhere to an uncompromising **One Life** rule. When a mission enters the tactical phase, players deploy into the world simultaneously, standing shoulder-to-shoulder to review equipment, assign fireteams, configure radios, and mount vehicles.

Without safe start, an accidental mouse click or negligent discharge (ND) instantly kills a friendly teammate and ends their entire multi-hour event with no respawn.

The **Safestart HUD** serves as the persistent, high-visibility visual anchor during this critical warmup window (`TBD_EGameStage.SAFE_START`). It operates directly in the player's primary viewport to:
1. Provide unambiguous confirmation that **weapons are safe and damage is disabled**.
2. Display a prominent **real-time countdown timer** to synchronization ("Go Live").
3. Remind personnel to remain within designated **staging perimeter boundaries**.
4. Surface **admin intervention states** (pause, time extensions, or manual release).
5. Signal the transition to combat with an unmistakable visual and auditory **"WEAPONS FREE"** release banner.

---

## 2. Visual Architecture & Layout Specification

### 2.1 Viewport Anchor & Layering
* **Position:** Top-center of the screen, docked at `Y = 24px` from the screen top margin.
* **Z-Order / Layer:** Anchored on the game HUD layer above standard player status, but below modal dialogs (pause menu, map, admin dashboard).
* **Reference Resolution:** 1920×1080 Enfusion Canvas. Layout dynamically centers horizontally.

### 2.2 Visual Layout Diagram

```text
                                  SCREEN TOP (Y: 0)
───────────────────────────────────────────────────────────────────────────────────
                                      [ 24px ]
         ┌───────────────────────────────────────────────────────────────┐
         │  [SHIELD]  SAFESTART ACTIVE                 [ ADMIN: PAUSED ] │ ◄ Status Bar / Admin Tag
         ├───────────────────────────────────────────────────────────────┤
         │                                                               │
         │                          04:32                                │ ◄ Warmup Countdown Timer
         │                                                               │
         ├───────────────────────────────────────────────────────────────┤
         │           WEAPONS COLD  •  DAMAGE OFF  •  ONE LIFE            │ ◄ Safety Warning Banner
         ├───────────────────────────────────────────────────────────────┤
         │       ▲ REMAIN INSIDE STAGING PERIMETER (120m to edge)        │ ◄ Staging Boundary Reminder
         └───────────────────────────────────────────────────────────────┘
                                   Width: 460px
```

### 2.3 Post-Safestart Release State (Flash Banner)

```text
         ┌───────────────────────────────────────────────────────────────┐
         │  [UNLOCK]  SAFE START ENDED — WEAPONS FREE                    │
         │            DAMAGE IS LIVE • CHECK TARGET IDENTIFICATION      │
         └───────────────────────────────────────────────────────────────┘
                               Accent: SUCCESS Green (#22C55E)
```

---

## 3. Component & Visual Inventory

| Component ID | UI Element | Visual Description & Tokens | Behavior & Function |
| :--- | :--- | :--- | :--- |
| **`HUD_PANEL`** | Container Card | Midnight navy glass panel (`SURFACE_GLASS`: `0xB31F2937`), subtle border (`BORDER_SUBTLE`: `0xFF374151`), rounded 4px. Size: 460×116 px. | Docks top-center. Opacity suppresses slightly during iron sight aim (ADS) to preserve line of sight. |
| **`STATUS_BADGE`** | "SAFESTART ACTIVE" | Shield icon + uppercase bold label. Tinted in tactical amber (`TACTICAL_YELLOW`: `0xFFFACC15`). | Latches active on `SAFE_START`. Changes to `SUCCESS` green (`#22C55E`) on lift. |
| **`TIMER_DISPLAY`** | Warmup Countdown | Huge, monospaced digital clock (`MM:SS`). Color: `ON_SURFACE` (`0xFFDDE2F7`). Font: 32px mono bold. | Driven by server `m_iSecondsRemaining`. Turns amber under 30s; flashes red (`ERROR`: `0xFFEF4444`) under 10s. |
| **`SAFETY_BANNER`** | Weapons Warning Subtitle | High-contrast status line: `"WEAPONS COLD • DAMAGE OFF • ONE LIFE"`. Tint: `ON_SURFACE_VARIANT` (`0xFFC4C6D0`). | Informs players that projectile sinks and character damage gates are active. |
| **`BOUNDARY_STRIP`** | Staging Perimeter Reminder | Sub-panel or banner strip: `"▲ STAY WITHIN STAGING AREA"` + optional distance readout. | Driven by `TBD_PlayAreaComponent`. Pulses amber if player steps outside staging bounds into warning grace period. |
| **`ADMIN_BADGE`** | Admin Status Indicator | Pill chip anchored top-right: `"PAUSED BY REF"` or `"TIME EXTENDED (+02:00)"`. | Hidden during normal countdown. Appears when an admin pauses the clock (`#tbd safestart pause`), overrides time, or forces go. |

---

## 4. Functional Mechanics & Technical Operation

1. **Three-Tier Safety Enforcement:**
   - **Damage Gate:** Player character damage handling is disabled (`EnableDamageHandling(false)`).
   - **Projectile Sink:** Any weapon discharge or thrown grenade is intercepted and deleted on creation.
   - **Weapon Safety:** Character safeties are forced on.
2. **Staging Boundary Reminder:** If a player wanders near or beyond the staging boundary, the banner pulses warning text alerting them to return before safe start lifts.
3. **Admin Controls:** Displays pause and extension indicators when referees manage the clock via admin commands.
4. **Transition to Live ("Weapons Free"):**
   - When the timer reaches `00:00` or an admin triggers `#tbd safestart go`, the banner flips to green (`#22C55E`) with the message `"SAFE START ENDED — WEAPONS FREE"`.
   - A distinct radio chime sound effect plays locally.
   - Damage handling is restored and verified server-side.
   - The banner persists for 6 seconds to ensure everyone sees the alert, then smoothly fades out.
