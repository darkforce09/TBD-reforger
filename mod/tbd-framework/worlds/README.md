# Framework worlds

The framework's two worlds: TBD Dev POC, vanilla Everon (the Eden world) as a parent sub-scene,
and TBD Dev POC Arland, vanilla Arland as its parent; each adds a layer that places the TBD game
mode. The [mission headers](/documentation/glossary/g_to_m.md#mission-header)
`Missions/TBD_Dev_POC.conf` and `Missions/TBD_Dev_POC_Arland.conf` boot them, and the
[mission](/documentation/glossary/g_to_m.md#mission) itself arrives from the platform at runtime, so
one development and test world serves every mission on its terrain.

## Contents

```text
mod/tbd-framework/worlds/
├── Eden/                        Workbench editor data for the vanilla Eden world
├── TBD_Dev_POC.ent              the TBD Dev POC world: a sub-scene of vanilla Eden
├── TBD_Dev_POC.ent.meta         the world's resource GUID, `{F652B97A6F497348}`
├── TBD_Dev_POC_Arland.ent       the TBD Dev POC Arland world: a sub-scene of vanilla Arland
├── TBD_Dev_POC_Arland.ent.meta  the Arland world's resource GUID, `{C664C066F1476634}`
├── TBD_Dev_POC_Arland_Layers/   the Arland world's layer: the same game mode and managers, Arland's AI world
└── TBD_Dev_POC_Layers/          the world's layer: the TBD game mode and three vanilla managers
```

## How it works

```text
Missions/TBD_Dev_POC.conf ──World──▶ worlds/TBD_Dev_POC.ent ──Parent──▶ worlds/Eden/Eden.ent
                                          │ layers                  (vanilla Everon, game data)
                                          ▼
               worlds/TBD_Dev_POC_Layers/default.layer ──▶ TBD_GameMode, FactionManager,
                                                           LoadoutManager, AIWorld
```

`TBD_Dev_POC_Arland.ent` is built the same way over `{A9806AF617972E97}worlds/Arland/Arland.ent`:
its layer places the same four entities, with the Arland AI world
`{01DC74137CFDDB6A}Prefabs/AI/SCR_AIWorld_Arland.et`, at `2048 0 2048`, the centre of the 4,096 m
terrain, where the Everon layer uses `6400 0 6400`.

`TBD_Dev_POC.ent` holds nothing but its parent: Everon's terrain, buildings and vegetation come from
the game's own Eden world, and the layer adds the TBD game mode, whose components load the deployed
mission and run the round. Everything a mission authors
([slots](/documentation/glossary/n_to_z.md#slot), zones, objectives, entities) is built at runtime from
the mission document, never saved into the world. The world places no `RadioManagerEntity`, so
`TBD_RadioComponent` in `mod/tbd-framework/Scripts/Game/TBD/Systems/Radio/` logs the radio
backbone as missing and falls back to its script-side channel table.

## Format

- File type: `TBD_Dev_POC.ent` is an [Enfusion](/documentation/glossary/a_to_f.md#enfusion) world, plain
  text, a `SubScene` block whose `Parent` names the vanilla world by resource; its layers sit in the
  sibling `TBD_Dev_POC_Layers/` folder.
- Resource GUID: `TBD_Dev_POC.ent.meta` holds `Name "{F652B97A6F497348}worlds/TBD_Dev_POC.ent"`
  and one `ENTResourceClass` per platform configuration, and `TBD_Dev_POC_Arland.ent.meta` holds
  `Name "{C664C066F1476634}worlds/TBD_Dev_POC_Arland.ent"`; each mission header names its world by
  that GUID, so it never changes.
- Naming: a world is `<Name>.ent` with a `<Name>_Layers/` folder beside it;
  [Workbench](/documentation/glossary/n_to_z.md#workbench)'s per-world editor data sits under
  `<world>/.EditorData/`.
- Adding a world: create it in Workbench inside this addon, which writes the `.ent`, its `.meta`
  and the layer folder; commit them together with a mission header in
  `mod/tbd-framework/Missions/` that names the world.

## Referenced by

- `mod/tbd-framework/Missions/TBD_Dev_POC.conf`, by resource GUID:
  `World "{F652B97A6F497348}worlds/TBD_Dev_POC.ent"`.
- `mod/tbd-framework/Missions/TBD_Dev_POC_Arland.conf`, by resource GUID:
  `World "{C664C066F1476634}worlds/TBD_Dev_POC_Arland.ent"`.
- `cargo xtask mod spawn-determinism`, whose world argument defaults to `worlds/TBD_Dev_POC.ent`
  (`tools/commands/mod_operations/src/mod_dispatch.rs`).
- Through the mission header, `cargo xtask mod world-boot`, `cargo xtask mod playtest`, the
  dedicated-server profiles, the deploy settings and the
  [fleet scenario](/documentation/glossary/a_to_f.md#fleet-scenario) seeds.

## Boundaries

- Depends on: the vanilla Eden world, `{853E92315D1D9EFE}worlds/Eden/Eden.ent`, and the vanilla
  Arland world, `{A9806AF617972E97}worlds/Arland/Arland.ent`, from the game's data; `mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et` through the layer.
- Used by: the mission header and every server that boots it, as listed above.
- Rules: the world holds no mission content, which comes from the platform at runtime; a world and
  its `.meta` are committed together and its GUID stays stable; `cargo xtask mod world-boot` boots
  the Everon world, the one its dedicated-server profile names, headless and fails when a game mode component does not instantiate.

## Related documentation

- [Two-client playtest](/documentation/runbooks/two_client_playtest/README.md) — booting the dev
  world with two clients
- [Spawn determinism](/documentation/runbooks/spawn_determinism.md) — the repeated boots of this
  world that check slot spawning
