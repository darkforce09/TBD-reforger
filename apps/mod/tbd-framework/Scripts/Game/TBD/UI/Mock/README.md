# UI/Mock

Static mock datasets for offline UI development, screen styling, and layout verification.

### Roles & Responsibilities
- `TBD_BriefingMock.c` (2026-09-14): builds the `TBD_BriefingCatalog` behind the Briefing from the
  mockups verbatim — 7 nets, 2 objectives (Everon coordinates for Locate), 4 + 4 rules, the lore
  paragraph, 5 parameters, friendly / enemy assets with vanilla vehicle prefabs on record (BTR-70,
  BRDM-2, Ural-4320, UAZ-469 / M1025, M923A1, M151A2) and the mockup's loadout rows, uniform cards
  whose dolls are the registry's faction riflemen, 4 plans.
- `TBD_PlayersMock.c` (2026-09-14): builds the `TBD_PlayersCatalog` behind the PLAYERS modal — the
  mockup's named rows plus generated ones up to 36 / 40 and 48 / 50, 4 spectators, 6 unslotted (94).
- `TBD_LobbyMock.c`: Builds the `TBD_LobbyCatalog` behind the Lobby from the Stitch pre-game mockup
  (lobby_sidebar / orbat_panel / slot_kit_inspector) — BLUFOR 92 vs OPFOR 95 + Spectators 10, the
  drawn squads / roles / holders / tags, four kits keyed by role. Each kit also carries its WIRE half —
  kit alias `kit:sov_rifleman` + a real `TBD_SlotLoadoutStruct` of GUID-pinned vanilla prefabs (read off
  golden-missions/slot-loadout-coverage.json and the `Character_USSR_*.et` prefabs) so the 3D preview
  dresses what the server would spawn; `crew` is kit-only, the Makarov has no GUID on record (handgun
  empty). Replaces `TBD_LobbyMockData` (2026-09-13); its voice channels return with the voice-panel
  pass, from that mockup.
- `TBD_MissionSelectorMock.c`: Builds the `TBD_MissionCatalog` behind the Mission Selector from
  the Stitch pre-game mockup — three terrains, nine missions, five modes, the PVP Test 1 inspector
  (versions, modset, factions, objectives), identity `Mission Maker` / `ADMIN`. Replaces the
  retired `TBD_MissionSelectorData.c` mock (2026-09-12).

### The swap point
Screens never hold a mock class. They read `TBD_MissionCatalog.Get()`
(`Session/MissionSelector/TBD_MissionSelectorData.c`), which lazily calls
`TBD_MissionSelectorMock.Build()` until a client cache calls `TBD_MissionCatalog.Set(...)`. The lobby, the briefing (`TBD_BriefingCatalog.Get()`) and the players modal (`TBD_PlayersCatalog.Get()`)
are the same shape: `TBD_LobbyCatalog.Get()` (`Session/Lobby/TBD_LobbyCatalog.c`) lazily calls
`TBD_LobbyMock.Build()` until the `TBD_LobbyClient` adapter calls `TBD_LobbyCatalog.Set(...)`. Wiring
the real mission library therefore touches nothing under `UI/`. Counts shown in the UI are
computed from the catalog, not stored in it.

### Call Flow & Contracts
Standalone data repositories consumed solely by UI controllers when developing screens without
active network or server dependencies.
