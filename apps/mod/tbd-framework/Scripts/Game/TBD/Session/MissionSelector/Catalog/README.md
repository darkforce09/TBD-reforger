# Mission Selector catalog

What the Mission Selector screen reads: the terrains, the mode tags, and each mission's hero,
modset, summary and per-faction rows, behind one read surface. It serves mock data until a client
cache installs a real catalog.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/Catalog/
├── TBD_MissionCatalog.c         `TBD_MissionCatalog`: the catalog in force, lookups and counts
├── TBD_MissionFactionSummary.c  one faction's role, slots, vehicles and objectives
├── TBD_MissionMode.c            one mode tag with its label and chip tint
├── TBD_MissionSummary.c         one mission, with its versions and required mods
└── TBD_TerrainInfo.c            one terrain with its icon and hero art
```

## How it works

`TBD_MissionCatalog.Get()` returns the catalog in force, built on first use by
`TBD_MissionSelectorMock` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Mock/`; `Set()` replaces
it, and `Set(null)` restores the mock on the next `Get()`. No script calls `Set()`, so the screen
shows mock missions. The screens hold the catalog and never a raw array: `GetMissions` and
`CountMissions` filter by terrain key, `CountMode` counts a mode on a terrain, and `FindMission`,
`FindTerrain`, `FindMode`, `ModeTint` and `ModeLabel` resolve keys, falling back to null, `NEUTRAL`
or the key itself. `TBD_MissionSummary.GetSlotTotal` sums the faction rows and falls back to
`m_iSlots` when there are none. The models run on both sides and hold no wire code.

## Authority

- Server: nothing.
- Client: everything; the catalog is read by the local player's Mission Selector screen.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_MissionSelectorMock` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Mock/`;
  `TBD_EUITint`, `TBD_SessionIdentity` and the icon and layout keys in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/`.
- Used by: the screen and its panels in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/UI/`; `TBD_SessionSelection` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/`; the mock that builds it.
- Rules: the screens read missions only through `TBD_MissionCatalog.Get()`; the arrays are never
  null; lines added stay ASCII and `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [Mission selection specification](/documentation_v2/mod/tbd-framework/UI/mission_selection/mission_selection_specification.md)
  — the screen as built, its data, design target, open work and decisions
