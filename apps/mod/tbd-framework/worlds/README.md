# Framework worlds

The framework's one world, TBD Dev POC: vanilla Everon (the Eden world) as a parent sub-scene,
plus a layer that places the TBD game mode. The
[mission header](/documentation_v2/glossary/g_to_m.md#mission-header) `Missions/TBD_Dev_POC.conf` boots
it, and the [mission](/documentation_v2/glossary/g_to_m.md#mission) itself arrives from the platform at
runtime, so this one development and test world serves every mission on Everon.

## Contents

```text
apps/mod/tbd-framework/worlds/
├── Eden/                 Workbench editor data for the vanilla Eden world
├── TBD_Dev_POC.ent       the TBD Dev POC world: a sub-scene of vanilla Eden
├── TBD_Dev_POC.ent.meta  the world's resource GUID, `{F652B97A6F497348}`
└── TBD_Dev_POC_Layers/   the world's layer: the TBD game mode and three vanilla managers
```

## How it works

```text
Missions/TBD_Dev_POC.conf ──World──▶ worlds/TBD_Dev_POC.ent ──Parent──▶ worlds/Eden/Eden.ent
                                          │ layers                  (vanilla Everon, game data)
                                          ▼
               worlds/TBD_Dev_POC_Layers/default.layer ──▶ TBD_GameMode, FactionManager,
                                                           LoadoutManager, AIWorld
```

`TBD_Dev_POC.ent` holds nothing but its parent: Everon's terrain, buildings and vegetation come from
the game's own Eden world, and the layer adds the TBD game mode, whose components load the deployed
mission and run the round. Everything a mission authors
([slots](/documentation_v2/glossary/n_to_z.md#slot), zones, objectives, entities) is built at runtime from
the mission document, never saved into the world. The world places no `RadioManagerEntity`, so
`TBD_RadioComponent` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Radio/` logs the radio
backbone as missing and falls back to its script-side channel table.

## Format

- File type: `TBD_Dev_POC.ent` is an [Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) world, plain
  text, a `SubScene` block whose `Parent` names the vanilla world by resource; its layers sit in the
  sibling `TBD_Dev_POC_Layers/` folder.
- Resource GUID: `TBD_Dev_POC.ent.meta` holds `Name "{F652B97A6F497348}worlds/TBD_Dev_POC.ent"`
  and one `ENTResourceClass` per platform configuration; the mission header names the world by
  that GUID, so it never changes.
- Naming: a world is `<Name>.ent` with a `<Name>_Layers/` folder beside it;
  [Workbench](/documentation_v2/glossary/n_to_z.md#workbench)'s per-world editor data sits under
  `<world>/.EditorData/`.
- Adding a world: create it in Workbench inside this addon, which writes the `.ent`, its `.meta`
  and the layer folder; commit them together with a mission header in
  `apps/mod/tbd-framework/Missions/` that names the world.

## Referenced by

- `apps/mod/tbd-framework/Missions/TBD_Dev_POC.conf`, by resource GUID:
  `World "{F652B97A6F497348}worlds/TBD_Dev_POC.ent"`.
- `cargo xtask mod spawn-determinism`, whose world argument defaults to `worlds/TBD_Dev_POC.ent`
  (`tools_v2/xtask/src/commands/mod_ops/dispatch.rs`).
- Through the mission header, `cargo xtask mod world-boot`, `cargo xtask mod playtest`, the
  dedicated-server profiles, the deploy settings and the
  [fleet scenario](/documentation_v2/glossary/a_to_f.md#fleet-scenario) seeds.

## Boundaries

- Depends on: the vanilla Eden world, `{853E92315D1D9EFE}worlds/Eden/Eden.ent`, from the game's
  data; `apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et` through the layer.
- Used by: the mission header and every server that boots it, as listed above.
- Rules: the world holds no mission content, which comes from the platform at runtime; a world and
  its `.meta` are committed together and its GUID stays stable; `cargo xtask mod world-boot` boots
  the world headless and fails when a game mode component does not instantiate.

## Related documentation

- [Two-client playtest](/documentation_v2/runbooks/two_client_playtest/README.md) — booting the dev
  world with two clients
- [Spawn determinism](/documentation_v2/runbooks/spawn_determinism.md) — the repeated boots of this
  world that check slot spawning
