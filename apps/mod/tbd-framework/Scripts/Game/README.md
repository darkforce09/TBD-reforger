# Game script module

The addon's [Enfusion](/documentation_v2/glossary.md#enfusion) game script module: every script here
compiles into the game itself, on dedicated servers, clients and in
[Workbench](/documentation_v2/glossary.md#workbench) play mode. The framework's scripts all sit
under one `TBD/` namespace folder.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/
└── TBD/  every framework script: platform bridge, core, round rules, systems, session flows, UI
```

## How it works

Enfusion builds a script module from each named folder under an addon's `Scripts/`, and a
dedicated server compiles only the `Game` module. Script discovery is a directory scan, so a new
or moved `.c` file needs no resource database entry, though Workbench builds its script list when
it loads the project and needs a restart to see a new file. Scripts from every loaded addon share
one namespace, which is why every class the framework adds carries the `TBD_` prefix and sits
under `TBD/`.

## Authority

- Server: the round, the [mission](/documentation_v2/glossary.md#mission) and every platform call,
  as the scripts under `TBD/` decide it.
- Client: the screens and HUD under `TBD/`.
- Owner: server answers go to the asking player's controller only.
- RPCs: declared under `TBD/`; that README summarises them.
- Replicated properties: declared under `TBD/Gamemode/`.

## Boundaries

- Depends on: vanilla Arma Reforger's game script module, the addon's one dependency.
- Used by: the dedicated servers and clients that load `apps/mod/tbd-framework/addon.gproj`, and
  `cargo xtask mod compile`, which compiles this module headless.
- Rules: only the `Game` module lives in this addon, and `cargo xtask mod compile` exits 1 when a
  `Scripts/WorkbenchGame/` folder appears here; lines added to a script stay ASCII.
