# Session/Players

Shared player data and the PLAYERS panel (2026-09-14).

### Roles & Responsibilities
- `TBD_PlayersCatalog.c`: `TBD_PlayerInfo` (name, faction, ping, tag, SLOTTED | SPECTATOR | UNSLOTTED) and the catalog (`GetSlotted(faction)`, `GetByState`, `Capacity`, totals). Mock until an adapter fills it from the authority (player manager + slot map) over one owner-scoped RPC; `Get()` / `Set()` is the swap point.
- `UI/TBD_PlayersPanel.c`: `TBD_PlayersPanel : Managed` — a briefing MODE like Markers: press Players and it pops out directly right of the primary nav, 800 wide (`Build(dock)` into the host's `WideDock`, `Destroy()`), header (title · TOTAL) and four `TBD_PlayerLane`s (BLUFOR / OPFOR tinted, Spectators / Unslotted neutral). No scrim, no window, no stacked menu (a menu pushed on top hides the one beneath and `SCR_MapEntity` closes with it — MEASURED). Layouts in `UI/layouts/Session/Shared/`.

### Call Flow & Contracts
`TBD_BriefingScreen.SetMode(PLAYERS)` shows `WideDock`, builds the panel, and destroys it on the next mode. Any dock screen with a wide dock can host it the same way. The nav badge and the top bar count read the same catalog.
