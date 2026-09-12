# UI/Hud

In-game head-up display overlays active during live gameplay.

### Roles & Responsibilities
- `TBD_ObjectiveHud.c`: Live HUD widget displaying active mission objectives, sector capture progress bars, and contested territory indicators.
- `TBD_TaskHud.c`: Tactical HUD overlay projecting task objective icons onto player map and HUD elements.

### Call Flow & Contracts
Client-only presentation widgets. Receive state updates via `SCR_PlayerController` RPCs from `Systems/Objectives/` and update on-screen visual progress in real time.
