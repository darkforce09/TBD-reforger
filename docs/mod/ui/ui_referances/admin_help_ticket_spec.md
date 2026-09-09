# TBD Reforger — Admin Help Ticket UI Specification & Functional Reference

**System Domain:** In-Game Player Support, Terrain Recovery, Bug Remediation & Referee Dispatch  
**Target Platform:** Reforger Enfusion Mod Framework (`apps/mod/tbd-framework`)  
**Backend & Replication:** `CRF_AdminMenuManager.c`, `CRF_PlayerRplToAuthorityManager.c`, `CRF_RplBroadcastManager.c`

---

## 1. Purpose & System Overview

The **Admin Help Ticket** is an in-game modal dialog enabling players in competitive milsim scenarios to quickly report game-breaking issues directly to connected referees and administrators without leaving the simulation.

### Operational Focus:
- **Collision & Terrain Extraction:** Rapid recovery for players clipped into rocks, buildings, or subsurface terrain meshes.
- **Client Desync & Medical Remediation:** Quick reporting for persistent uniform/vest model invisibility, broken bleeding/unconscious loops, or inventory glitches.
- **Spectator Cam Restoration:** One-life spectator camera recovery when players are trapped in black screens or desynced post-death.
- **Rule Violation & Match Administration:** Discreet signaling of safe-start breaches, asset theft, or intentional teamkilling with verified spatial coordinates.

---

## 2. UI Layout & Visual Wireframe

The dialog appears centered on screen over a darkened backdrop, pausing background game input while active.

```
┌───────────────────────────────────────────────────────────────────────────────┐
│ [!] REQUEST REFEREE ASSISTANCE                                           [ ✕ ] │
├───────────────────────────────────────────────────────────────────────────────┤
│                                                                               │
│  ISSUE CATEGORY                                                               │
│  ┌─────────────────────────────────────────────────────────────────────────┐  │
│  │ [▾] Stuck in Terrain / Wedged in Geometry                               │  │
│  └─────────────────────────────────────────────────────────────────────────┘  │
│      Options: Stuck in Terrain | Medical / Uniform Glitch                     │
│               Spectator Bug   | Rule Violation / Admin Request                │
│                                                                               │
│  INCIDENT DESCRIPTION                                                         │
│  ┌─────────────────────────────────────────────────────────────────────────┐  │
│  │ Fell through floor of warehouse near hangar. Legs broken, cannot move.  │  │
│  │ Please extract to safe ground outside.                                  │  │
│  │                                                                         │  │
│  │                                                               78 / 256  │  │
│  └─────────────────────────────────────────────────────────────────────────┘  │
│                                                                               │
│  TELEMETRY & LOCATION METADATA (Auto-Attached)                                │
│  ┌─────────────────────────────────────────────────────────────────────────┐  │
│  │  Player: [1stID] Miller (BLUFOR)           Grid: 042-088 (Elev: 42m)    │  │
│  │  State: Alive / Injured (Bleeding)         Vehicle: None (On Foot)      │  │
│  │  Time: 12:44:19 UTC                        Active Admins Online: 2      │  │
│  └─────────────────────────────────────────────────────────────────────────┘  │
│                                                                               │
│  [ CANCEL ]                                                [ SUBMIT TICKET ]  │
└───────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. UI Components & Control Inventory

| Widget Name | Element Type | Purpose & Behavior |
| :--- | :--- | :--- |
| **`HeaderBanner`** | Header Panel | Displays modal title `REQUEST REFEREE ASSISTANCE`. Non-interactive. |
| **`CloseButton`** | Push Button (`✕`) | Dismisses the dialog without dispatching ticket (`Esc`). |
| **`CategorySelector`** | Dropdown / ComboBox | 4 selectable categories: `Stuck in Terrain`, `Medical / Uniform Glitch`, `Spectator Bug`, `Rule Violation / Admin Request`. |
| **`DescriptionInput`** | Multiline Edit Box | Freeform description box. Max limit: 256 characters. |
| **`CharCounter`** | Text Label | Real-time counter: `<Current> / 256`. |
| **`MetadataPanel`** | Readout Card | Displays auto-attached telemetry: player callsign, faction, 6-digit military grid, elevation, life state, and active admin count. |
| **`CancelButton`** | Action Button | Aborts submission, closes menu, releases mouse lock. |
| **`SubmitButton`** | Action Button | Dispatches payload to server authority via reliable RPC. Disabled if active ticket exists or cooldown active. |

---

## 4. Functional Lifecycle & Two-Way Alerts

1. **Invocation:**
   - Accessible via pause menu (`Esc` → `HELP TICKET`) or dedicated keybind (`F8`).
   - Usable while alive or in spectator mode.
2. **Server Dispatch & Referee Alerts:**
   - Broadcasts alert to all connected referees in their admin panel (`admin_chat`):
     `"[TICKET #12] <Player> (<Faction> @ <Grid>) [Stuck in Terrain]: <Description>"`
   - Plays an administrative chime on referee clients.
3. **Referee Acknowledgment:**
   - When a referee claims the ticket, the player receives an on-screen toast:
     `"TICKET ACKNOWLEDGED: Referee <AdminName> is reviewing your request."`
4. **Action Execution:**
   - **Terrain Unstick:** Referee clicks unstick; player is teleported to adjacent clear ground with safe collision offset.
   - **Medical Heal:** Referee clicks heal; removes injuries and revives player.
5. **Ticket Closure:**
   - Referee closes ticket upon resolution; player receives confirmation banner.
   - 60-second client cooldown prevents ticket spamming.
