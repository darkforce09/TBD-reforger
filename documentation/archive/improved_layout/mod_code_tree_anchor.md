**Status:** archived — see [the restructure program](/documentation/restructure/README.md)

# Mod suite reorganization plan

The code-tree planning anchor for the architectural cleanup and directory reorganization of
`apps/mod`. It holds no production code, only this index pointing to the detailed design
specifications in `documentation_v2/mod/improved_layout/`.

## Contents

```text
apps/mod/improved_layout/
```

## How it works

This directory anchors the mod reorganization initiative within the `apps/mod/` code tree. In
accordance with the monorepo's documentation placement invariants (enforced by
`cargo xtask verify markdown-placement`, Rule 1), code trees contain only `README.md` files. All
in-depth design proposals, architectural rationale, and execution blueprints live under
`documentation_v2/mod/improved_layout/`.

## High-level reorganization summary

```text
apps/mod/
├── README.md                  Mod suite overview and getting started guide
├── .mcp.json                  Enfusion MCP server registration for Workbench
├── .local-test-profile/       Local dedicated server runtime profile (gitignored)
├── improved_layout/           [NEW] This planning index
├── References/                [NEW FOLDER] Consolidated upstream reference code (gitignored)
│   ├── README.md              Reference isolation invariants and licensing rules
│   ├── crf_framework/         Coalition Reforger Framework reference source
│   └── vanilla_reference/     Bohemia Arma Reforger unpacked engine scripts
├── tbd-framework/             Core shipping game mod runtime
│   ├── Configs/               Input actions and menu configurations
│   ├── Data/                  Core prefab alias registry (registry.json)
│   ├── Prefabs/               GameMode, PlayerController, and component prefabs
│   ├── Scripts/               Flattened: Scripts/Game/ (eliminates redundant TBD/ nesting)
│   │   └── Game/
│   │       ├── API/           HTTP, Session, Identity, and Fleet services
│   │       ├── Core/          Logging, Math, Authority, and Players
│   │       ├── Gamemode/      Orchestrator, Stages, and Objectives (Engine/ vs Types/)
│   │       ├── Session/       Briefing, Lobby, Spectator, and Admin
│   │       ├── Systems/       Spawning, Loadouts, Radio, Markers, and Zones
│   │       └── UI/            HUD controllers, menu bindings, and toasts
│   └── UI/                    Layouts (.layout), styles, textures, and fonts
├── tbd-missions/              [NEW ADDON] Standalone scenario and world content
│   ├── Missions/              Scenario headers (.conf, e.g., TBD_Dev_POC.conf)
│   ├── worlds/                World subscenes (.ent), layer directories, Eden
│   ├── addon.gproj            Declares dependency on TBD_Framework and vanilla
│   └── resourceDatabase.rdb   Workbench asset registry for missions
├── tbd-export/                Workbench export plugin suite (Map, Equip, Vehicle)
└── tbd-emcp/                  Enfusion MCP Net API handlers for Workbench automation
```

## Boundaries

- Depends on: `documentation_v2/mod/improved_layout/` for all architectural specifications and migration roadmaps.
- Used by: mod developers, tools authors, and autonomous agents planning mod refactor slices.
- Rules:
  - Code trees hold only `README.md` (`cargo xtask verify markdown-placement`, Rule 1).
  - No production code, scripts, or assets may be committed inside `apps/mod/improved_layout/`.
  - All file moves and code modifications are governed by the approved phased roadmap.

## Related documentation

- [Mod reorganization master overview](/documentation/archive/improved_layout/mod/01_master_overview.md) —
  the primary combined synthesis of all changes across the mod suite.
- [Mod root structure](/documentation/archive/improved_layout/mod/02_mod_root_structure.md) —
  top-level directory layout, peer addon boundaries, and root clutter elimination.
- [References consolidation](/documentation/archive/improved_layout/mod/03_references_consolidation.md) —
  moving `crf_framework` and `vanilla_reference` into `References/` and updating tooling gates.
- [Standalone tbd-missions addon](/documentation/archive/improved_layout/mod/04_tbd_missions_addon.md) —
  extracting `Missions/` and `worlds/` into an independent addon with a clean DAG.
- [Framework scripts flattening](/documentation/archive/improved_layout/mod/05_framework_scripts_flattening.md) —
  removing `Scripts/Game/TBD/` redundant nesting in favor of direct `Scripts/Game/` domain paths.
- [Gamemode and objectives architecture](/documentation/archive/improved_layout/mod/06_gamemode_and_objectives.md) —
  restructuring `Gamemode/` into `Orchestrator/`, `Stages/`, and composable `Objectives/` (`Engine/` vs `Types/`).
- [Tooling and CI impact](/documentation/archive/improved_layout/mod/07_tooling_and_ci_impact.md) —
  updates required in `tools_v2/xtask` (`mod compile`, `world-boot`, `dev-bootstrap`), server configs, and CI gates.
- [Phased migration roadmap](/documentation/archive/improved_layout/mod/08_migration_roadmap.md) —
  step-by-step rollout sequence ensuring zero broken CI gates or server boot failures.
