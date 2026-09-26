# Pre-game screen mock data

The mock catalogs behind the rebuilt pre-game screens: fixed datasets taken from the Stitch mockups
that the [mission](/documentation_v2/glossary/g_to_m.md#mission) selector, lobby, briefing and players
screens render when no live catalog has been handed to them.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/UI/Mock/
├── TBD_BriefingMock.c         TBD_BriefingMock: briefing nets, objectives, rules, assets and uniforms
├── TBD_LobbyMock.c            TBD_LobbyMock: the lobby's sides, squads, slots and kits
├── TBD_MissionSelectorMock.c  TBD_MissionSelectorMock: the selector's terrains, modes and missions
└── TBD_PlayersMock.c          TBD_PlayersMock: the players modal's lanes, spectators and unslotted
```

## How it works

Each class has one `static Build()` that returns a filled catalog. A screen never names a mock
class: it reads its catalog's `Get()`, which calls `Build()` once when no catalog has been set,
and a catalog's `Set(...)` replaces it (null brings the mock back on the next `Get()`). No script
in the addon calls a catalog's `Set(...)` today, so these datasets are what the screens show.

| Mock | Catalog, read through `Get()` | Dataset |
|---|---|---|
| `TBD_MissionSelectorMock` | `TBD_MissionCatalog` in `Session/MissionSelector/TBD_MissionSelectorData.c` | three terrains (Everon, Arland, Kolguyev), five modes, nine missions, the `PVP Test 1` inspector |
| `TBD_LobbyMock` | `TBD_LobbyCatalog` in `Session/Lobby/TBD_LobbyCatalog.c` | BLUFOR (92 seats), OPFOR (95) and spectators (10), the drawn squads and holders, four kits keyed by role |
| `TBD_BriefingMock` | `TBD_BriefingCatalog` in `Session/Briefing/TBD_BriefingCatalog.c` | seven radio nets, objectives, rules, lore, parameters, both sides' assets and uniforms, plans |
| `TBD_PlayersMock` | `TBD_PlayersCatalog` in `Session/Players/TBD_PlayersCatalog.c` | BLUFOR 36 of 40, OPFOR 48 of 50, four spectators, six unslotted |

The catalog paths are under `apps/mod/tbd-framework/Scripts/Game/TBD/`. Counts a screen shows
(seats taken, missions available, badges) are computed by the screen from the rows, never stored
in a catalog. The previews render real vanilla prefabs: the briefing's vehicles are GUID-pinned
vanilla prefabs standing in for the mockup's names, and each lobby kit except `crew` carries a
`TBD_SlotLoadoutStruct` of GUID-pinned vanilla items, read off
`contracts_v2/fixtures/missions/valid/slot-loadout-coverage.json` and the vanilla
`Character_USSR_*.et` prefabs, so the 3D doll wears what the server would spawn. The players mock
names the mockup's first nine players per side and generates the rest up to the lane count.

## Authority

- Server: nothing.
- Client: everything; the catalogs are built in the local UI and never replicate.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: the catalog and row classes in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/` (`TBD_MissionCatalog`, `TBD_LobbyCatalog`,
  `TBD_BriefingCatalog`, `TBD_PlayersCatalog` and their rows); `TBD_SlotLoadoutStruct` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/`; `TBD_SessionIdentity` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Common/`; `TBD_UILayouts` and `TBD_EUITint` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/`.
- Used by: the four catalogs' `Get()`, in the files the table names; through them, the mission
  selector, lobby, briefing and players screens under
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/`.
- Rules: a screen reads its catalog, never a mock class, so a live catalog replaces a mock without
  touching `UI/`; a mock holds rows, never a count the screen can compute; a prefab a mock names is
  GUID-pinned and exists in the vanilla data; lines added to a script stay ASCII, and
  `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [Mission selection specification](/documentation_v2/mod/tbd-framework/UI/mission_selection/mission_selection_specification.md)
  — the design the selector mock reproduces
- [Lobby specification](/documentation_v2/mod/tbd-framework/UI/lobby/lobby_specification.md) — the
  design the lobby mock reproduces
- [Briefing specification](/documentation_v2/mod/tbd-framework/UI/briefing/briefing_specification.md)
  — the design the briefing and players mocks reproduce
