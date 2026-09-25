# Export mission header

The export addon's [mission header](/documentation_v2/glossary.md#mission-header): the Enfusion
config that boots the export world with the export game mode, whose one component writes Everon's
road network to the profile folder once the world has started. It carries no
[mission](/documentation_v2/glossary.md#mission) and no player-facing content.

## Contents

```text
apps/mod/tbd-export/Missions/
└── TBD_Export_Everon.conf  the "TBD Export - Everon" header: the export world and game mode
```

## How it works

`TBD_Export_Everon.conf` is an `SCR_MissionHeader` that names the world
`{C3D4E5F6A7B80002}worlds/TBD_Export_Everon.ent`, a sub-scene of vanilla Eden (Everon) whose layer
places the export game mode prefab and Eden's AI world. The header itself sets the name
"TBD Export - Everon", the author "TBD Platform", the game mode label "TBD Export", 64 players and a
12:00 start; its description says what a run does.

```text
Missions/TBD_Export_Everon.conf ──World──▶ worlds/TBD_Export_Everon.ent ──Parent──▶ {853E92315D1D9EFE}worlds/Eden/Eden.ent
                                           worlds/TBD_Export_Everon_Layers/default.layer ──places──▶ Prefabs/Systems/TBD_Export_GameMode.et
                                                                                          └─places──▶ {70CCCF16487C927F}Prefabs/AI/SCR_AIWorld_Eden.et
```

Playing the header, in [Workbench](/documentation_v2/glossary.md#workbench) or on a server that
loads the `TBD_Export` addon, starts the game mode; its `TBD_RoadExportComponent` (in
`apps/mod/tbd-export/Scripts/Game/TBD/Export/`) runs half a second after it initialises and writes
the road files under `$profile:TBD_Export/everon/roads/`.

## Format

- File type: an Enfusion config (`.conf`), plain text `SCR_MissionHeader { … }` with the vanilla
  fields `World`, `m_sName`, `m_sAuthor`, `m_sDescription`, `m_sGameMode`, `m_iPlayerCount` and
  `m_iStartingHours`.
- Resource GUID: none is committed. The header has no `.conf.meta` beside it, so its resource GUID
  is whatever Workbench assigns when it opens the addon; nothing refers to the header by GUID.
- Naming: `TBD_Export_<Terrain>.conf`, one header per world the export addon boots.
- Adding a header: create it in Workbench inside this addon, pointing `World` at an export world in
  `apps/mod/tbd-export/worlds/`, and commit it with its `.meta` file.

## Referenced by

- Nothing in the repository names the header: no dedicated-server profile, deploy setting or
  `cargo xtask` command boots it. It is opened by hand in Workbench.

## Boundaries

- Depends on: the world `apps/mod/tbd-export/worlds/TBD_Export_Everon.ent`, and through it the
  vanilla Eden world, Eden's AI world prefab and the export game mode prefab in
  `apps/mod/tbd-export/Prefabs/Systems/`.
- Used by: nothing outside the folder.
- Rules: the header boots only the export world, never a framework world; the addon is Workbench
  tooling, so the header is never deployed to a game server
  (`cargo xtask deploy staging` excludes `apps/mod/tbd-export/`,
  `tools_v2/xtask/src/commands/deploy/staging/remote/ssh_argv.rs`).
