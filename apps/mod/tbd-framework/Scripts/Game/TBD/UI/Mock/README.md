# UI/Mock

Static mock datasets for offline UI development, screen styling, and layout verification.

### Roles & Responsibilities
- `TBD_LobbyMock.c`: Builds the `TBD_LobbyCatalog` behind the Lobby from the Stitch pre-game mockup
  (lobby_sidebar / orbat_panel / slot_kit_inspector) — BLUFOR 92 vs OPFOR 95 + Spectators 10, the
  drawn squads / roles / holders / tags, four kits keyed by role. Replaces `TBD_LobbyMockData`
  (2026-09-13); its voice channels return with the voice-panel pass, from that mockup.
- `TBD_MissionSelectorMock.c`: Builds the `TBD_MissionCatalog` behind the Mission Selector from
  the Stitch pre-game mockup — three terrains, nine missions, five modes, the PVP Test 1 inspector
  (versions, modset, factions, objectives), identity `Mission Maker` / `ADMIN`. Replaces the
  retired `TBD_MissionSelectorData.c` mock (2026-09-12).

### The swap point
Screens never hold a mock class. They read `TBD_MissionCatalog.Get()`
(`Session/MissionSelector/TBD_MissionSelectorData.c`), which lazily calls
`TBD_MissionSelectorMock.Build()` until a client cache calls `TBD_MissionCatalog.Set(...)`. The lobby
is the same shape: `TBD_LobbyCatalog.Get()` (`Session/Lobby/TBD_LobbyCatalog.c`) lazily calls
`TBD_LobbyMock.Build()` until the `TBD_LobbyClient` adapter calls `TBD_LobbyCatalog.Set(...)`. Wiring
the real mission library therefore touches nothing under `UI/`. Counts shown in the UI are
computed from the catalog, not stored in it.

### Call Flow & Contracts
Standalone data repositories consumed solely by UI controllers when developing screens without
active network or server dependencies.
