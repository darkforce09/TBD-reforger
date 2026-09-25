# TBD framework scripts

Every game script of the TBD framework [mod](/documentation_v2/glossary.md#mod), in six
folders: the platform bridge, shared utilities, the round rules, the in-world systems, the
player and admin session flows, and the UI library. Together they load the
[mission](/documentation_v2/glossary.md#mission) the platform deploys to a dedicated server and
run the [event](/documentation_v2/glossary.md#event) from lobby to debrief.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/
├── API/       the platform bridge: game runtime session, fleet commands, identity links, results
├── Core/      shared utilities: the structured log, the prefab registry, player chat, SHA-256
├── Gamemode/  the round rules: stage machine, safe start, objectives, tasks and end conditions
├── Session/   the player and admin flows: mission selection, lobby, briefing, spectator, post-game
├── Systems/   in-world machinery: mission load, spawns, loadouts, zones, AI, audio, markers, radio
└── UI/        the shared view layer: menu framework, components, HUD and mock catalogs
```

## How it works

```text
            API/ ◀──HTTP──▶ platform API (deployment, artifact, roster, runtime session, fleet)
             │
Systems/ ── loads the mission, stands slot bodies, runs zones, AI, audio, markers, radio
             │
Gamemode/ ── TBD_FrameworkManager: LOADING ▶ LOBBY ▶ BRIEFING ▶ SAFE_START ▶ LIVE ▶ END ▶ DEBRIEF
             │ replicated stage
Session/ ── lobby, briefing, spectator, admin and post-game flows ──▶ UI/ screens and HUD
             │
Core/ ── used by all: TBD_Log, TBD_Registry, TBD_PlayerChat, hashing
```

`TBD_FrameworkManager` in `Gamemode/`, a component of
`apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, is where a round starts: it loads the
mission through `Systems/`, runs the stage machine, and every other folder reads its stage. Every
modded vanilla class checks `TBD_FrameworkManager.IsFrameworkWorld()` first, so a vanilla [mission
header](/documentation_v2/glossary.md#mission-header) with the addon loaded runs none of the
framework.

Most features in `Session/` and `Systems/` split their code by role:

| File | Runs on | Holds |
|---|---|---|
| `TBD_<X>Data.c` | both | plain model classes and wire structs |
| `TBD_<X>Service.c` | server | the payloads built from the mission document |
| `TBD_<X>Controller.c` | both | the RPC pairs on a `modded class SCR_PlayerController` |
| `TBD_<X>Client.c` | client | the received cache and the `ScriptInvoker`s screens bind to |
| `TBD_<X>Component.c` | server | the `SCR_BaseGameModeComponent` that hosts the feature's lifecycle |

A screen lives in its feature's `UI/` subfolder under `Session/`, reads the feature's client
cache or catalog, and builds from the shared library in `UI/`. `Gamemode/` decides the round;
`Systems/` provides the tools it asks, and decides no outcome.

Three facts hold across the folders: clients never hold the mission document, so every payload a
player sees is built on the server; `JsonLoadContext` allocates a nested block even when its key
is absent, so presence is tested with a sentinel; statics outlive a world inside one process, so
each folder clears its static state when a new world starts.

## Authority

- Server: the mission document, the stage machine, spawning, loadouts, zones, objectives, AI,
  admin commands and every call to the platform [API](/documentation_v2/glossary.md#api).
- Client: the screens, the HUD, the spectator camera, and the pull loops that ask the server for
  markers, radio nets, the briefing and the HUD snapshots.
- Owner: every server-to-client answer is an owner RPC to the asking or addressed player only.
- RPCs: all on a `modded class SCR_PlayerController`, in `Session/`, `Systems/` and `UI/Hud/`;
  every one Reliable except one Unreliable spectator request to the Server. Each folder's README
  lists its own.
- Replicated properties: only in `Gamemode/`: the stage, the authored spectator and night-vision
  settings, the end banner and debrief board, and the safe start countdown.

## Boundaries

- Depends on: vanilla Arma Reforger's script API; the platform API over HTTP, with the server's
  `mod_runtime` [machine credential](/documentation_v2/glossary.md#machine-credential); the wire
  shapes in `contracts_v2/definitions/`; the layouts in `apps/mod/tbd-framework/UI/`, the prefabs
  in `apps/mod/tbd-framework/Prefabs/`, the configs in `apps/mod/tbd-framework/Configs/` and
  `Data/registry.json` in `apps/mod/tbd-framework/Data/`.
- Used by: `apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, which attaches the manager
  components; the menu presets in `apps/mod/tbd-framework/Configs/System/chimeraMenus.conf`; the
  layouts under `apps/mod/tbd-framework/UI/layouts/` that attach handlers by class; and
  `cargo xtask mod compile` and `cargo xtask mod world-boot`, which compile and boot them. No
  other addon in `apps/mod/` names these classes.
- Rules: every class the addon adds carries the `TBD_` prefix; wire structs keep the JSON keys'
  spelling and carry a `@contract` tag (`cargo xtask schema citations`); RPC and replicated members
  carry `@rpc` and `@replicated` tags and context-dependent methods an `@authority` tag; a new game
  mode component is added to `TBD_GameMode.et` and to the manager's roll-call, which `cargo xtask
  mod world-boot` checks; a new script file needs a
  [Workbench](/documentation_v2/glossary.md#workbench) restart to appear there; lines added to a
  script stay ASCII, and `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [Mod design](/documentation_v2/mod/tbd-framework/mod_design.md) — what the framework is for, its
  non-negotiables and the [Enfusion](/documentation_v2/glossary.md#enfusion) facts it relies on
- [Mod UI documentation](/documentation_v2/mod/tbd-framework/UI/README.md) — the specification of
  each in-game screen
- [Mod slice workflow](/documentation_v2/runbooks/mod_slice_workflow.md) — how mod work runs through
  Workbench and the gates
