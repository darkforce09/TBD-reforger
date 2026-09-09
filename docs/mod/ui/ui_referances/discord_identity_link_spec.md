# TBD Reforger — Discord Identity Link Modal UI Functional Specification

**Target Platform:** Bohemia Interactive Enfusion Engine (Arma Reforger) / TBD Mod Framework  
**Theme System:** Aegis UI (`TBD_UITheme.c`)  
**Layout Reference:** `UI/layouts/TBD_IdentityLinkModal.layout`  

---

## 1. Purpose & Overview

The **Discord Identity Link Modal** is an in-engine dialog that binds a player's Bohemia Interactive account (Reforger platform UID / `arma_id`) to their TBD community web profile and Discord identity (`users.discord_id`).

### Why Linking is Required
- **Event ORBAT Slotting & Reservation:** Enforces claimed slots and leadership roles registered on the website, preventing unauthorized slot takeovers.
- **Match Telemetry & Leaderboards:** Maps in-game combat results, medical actions, objective completions, and attendance directly to the player's persistent web profile.
- **Discord Role Verification:** Propagates community standings (`@Admin`, `@Mission Maker`, `@Enlisted`, `@Recruit`) into the game session for rank badges and permissions.

---

## 2. Visual Layout & Wireframe Architecture

The modal appears as a focused, high-contrast modal dialog centered on screen over a full-bleed dark scrim, pausing background game input while active.

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│  COMMUNITY IDENTITY LINK                                                          [×]  │ <- Header Bar
├────────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                        │
│   WHY LINK YOUR ACCOUNT?                                                               │
│   Linking your Bohemia Reforger UID connects your in-game soldier to your TBD          │
│   community profile. This verifies your Discord tier, reserves your scheduled event    │
│   slots, and records your mission stats to the official leaderboards.                  │
│                                                                                        │
│   ┌────────────────────────────────────────────────────────────────────────────────┐   │
│   │                              YOUR ONE-TIME LINK CODE                           │   │
│   │                                                                                │   │
│   │                             [   T B D - 8 X 2 K   ]                            │   │
│   │                                                                                │   │
│   │                 Expires in: 09:42   •   [ COPY CODE ]   •   [ REGENERATE ]     │   │
│   └────────────────────────────────────────────────────────────────────────────────┘   │
│                                                                                        │
│   LINKING INSTRUCTIONS                                                                 │
│   Choose either option below to complete the connection:                               │
│                                                                                        │
│   • METHOD A (WEB PORTAL):                                                             │
│     1. Open https://tbd.gg/link in your browser.                                       │
│     2. Sign in with your TBD Discord account.                                          │
│     3. Enter the 6-character code above and click "Confirm Link".                      │
│                                                                                        │
│   • METHOD B (DISCORD BOT):                                                            │
│     1. Open any channel in the TBD Reforger Discord server.                            │
│     2. Type `/link code:TBD-8X2K` and press Enter.                                     │
│                                                                                        │
│   ──────────────────────────────────────────────────────────────────────────────────   │
│   CURRENT STATUS:                                                                      │
│   ● UNLINKED — No web profile associated with this Bohemia UID                         │
│                                                                                        │
│   [ (Optional) Do not show again on connect ]                           [ CLOSE (ESC) ]│ <- Footer Action Rail
└────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. UI Component Inventory

| Component ID | Label / Visual Text | Control Type | Purpose & Behavior |
| :--- | :--- | :--- | :--- |
| `TitleBar` | `COMMUNITY IDENTITY LINK` | Header Banner | Modal identification banner. Non-clickable. |
| `BtnCloseIcon` | `×` | Icon Button | Dismisses modal immediately, restoring standard gameplay or lobby input. |
| `TxtExplanation` | "Why Link Your Account?" | Text Block | Explains slot reservation, stats tracking, and attendance. |
| `BoxCodeDisplay` | `TBD-8X2K` | Monospace Token Box | Displays the 6-character unique linking token prominently. |
| `TxtTimer` | `Expires in: MM:SS` | Countdown Label | Dynamic 10-minute TTL timer. Turns red when `< 01:00`. |
| `BtnCopy` | `COPY CODE` | Action Button | Copies code string to clipboard. Displays `COPIED!` toast for 2s. |
| `BtnRegenerate` | `REGENERATE` | Utility Button | Revokes current code and generates a fresh one (rate-limited). |
| `TxtSteps` | Step-by-Step Instructions | Formatted List | Clear instructions for Web portal (`tbd.gg/link`) and Discord bot (`/link`). |
| `BadgeStatus` | Status Indicator + Text | Status Chip | Amber for `UNLINKED`, Green for `LINKED AS <User>`. |
| `RoleTagsCluster` | `[ @Role1 ] [ @Role2 ]` | Chip Container | Displays verified community permissions fetched from backend. |
| `BtnClose` | `CLOSE (ESC)` | Primary Button | Closes dialog and unblocks input. |

---

## 4. Functional Mechanics & Operation

1. **Invocation Vectors:**
   - Automatically prompts upon joining the server if unlinked.
   - Accessible on demand via pause menu (`Esc` → `LINK DISCORD`).
2. **Code Generation:**
   - 6-character uppercase alphanumeric code (e.g., `TBD-8X2K`), valid for 10 minutes.
   - Generated server-side via backend API (`POST /api/v1/ingest/link-code`).
3. **Verification Channels:**
   - Web link form (`https://tbd.gg/link`).
   - Discord slash command (`/link code:TBD-8X2K`).
4. **Real-Time Synchronization:**
   - Dialog polls server state every 3 seconds while open.
   - Upon completion, the code box flashes green and updates to show the bound Discord username and community roles.
