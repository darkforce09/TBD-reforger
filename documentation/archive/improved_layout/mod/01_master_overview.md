**Status:** archived — see [the restructure program](/documentation/restructure/README.md)

# Mod suite reorganization master overview

**Status:** Proposed design  
**Scope:** `apps/mod` and related tooling infrastructure  
**Context:** TBD Reforger platform monorepo  

The master architectural overview and combined synthesis of the reorganization plan for the
`apps/mod` Enfusion game mod suite.

---

## 1. Executive Summary

The `apps/mod` directory contains the Enfusion game mod ecosystem powering the TBD Reforger platform.
It includes `tbd-framework` (the shipping runtime loaded by dedicated game servers and players),
`tbd-export` (Workbench export plugins generating terrain, equipment, and vehicle catalogs), and
`tbd-emcp` (the Enfusion MCP automation Net API bridge).

While the underlying Enforce Script codebase has been extensively modularized through programs like
T-1092, several legacy directory patterns remain that compromise modularity, create unnecessary path
friction, and blend distinct architectural responsibilities:

1. **Uncontained upstream reference folders**: Upstream source archives (`crf_framework` and
   `vanilla_reference`) sit directly at the root of `apps/mod/`. Although properly gitignored and
   screened by `cargo xtask verify no-crf-leak`, having them at root clutters the addon workspace and
   complicates repository-wide path filters.
2. **Coupled scenario and world content in `tbd-framework`**: `tbd-framework` currently bundles both
   core runtime code (`Scripts/`, `Prefabs/`, `UI/`, `Configs/`, `Data/`) and specific scenario content
   (`Missions/` and `worlds/`). In standard Enfusion addon architecture, reusable libraries and engine
   frameworks should be cleanly separated from scenario packages.
3. **Redundant directory nesting in Enforce Scripts**: All gameplay scripts are nested under
   `tbd-framework/Scripts/Game/TBD/`. Because Enforce Script classes already use global namespace prefixes
   (`TBD_`) and Enfusion's compilation scope is determined by `Scripts/Game/`, the `TBD/` folder level
   is redundant and adds gratuitous path depth.
4. **Abstract objective decomposition obscures gameplay**: The current `Gamemode/Objectives/` subsystem
   is split along technical abstraction layers (`Model/`, `Registry/`, `Runtime/`, `Tasks/`). This makes
   it difficult for developers and scenario authors to discover, extend, or understand specific objective
   types (such as Capture, Destroy, HoldUntil, and HVT).

This reorganization resolves these issues comprehensively: establishing a dedicated `References/`
container, extracting scenario content into an independent `tbd-missions` addon, flattening scripts to
`Scripts/Game/`, and organizing `Objectives/` into clean engine plumbing (`Engine/`) and pluggable
objective types (`Types/`).

---

## 2. Master Directory Matrix (Before vs. After)

### 2.1. Complete Subsystem Comparison

| Area | Before (Current) | After (Proposed) | Action | Rationale |
|---|---|---|---|---|
| **Upstream CRF** | `apps/mod/crf_framework/` | `<apps/mod/References/crf_framework/>` | **RELOCATE** | Cleans mod root; isolates upstream reference source. |
| **Vanilla ref** | `apps/mod/vanilla_reference/` | `<apps/mod/References/vanilla_reference/>` | **RELOCATE** | Groups reference archives in a single designated container. |
| **References root** | *Non-existent* | `<apps/mod/References/>` | **NEW** | Provides unified home and `README.md` for gitignored references. |
| **Scenario headers**| `tbd-framework/Missions/` | `<apps/mod/tbd-missions/Missions/>` | **EXTRACT** | Separates mission definitions from engine framework runtime. |
| **World scenes** | `tbd-framework/worlds/` | `<apps/mod/tbd-missions/worlds/>` | **EXTRACT** | Separates map subscenes and Eden layers into missions addon. |
| **Missions addon** | *Non-existent* | `<apps/mod/tbd-missions/>` | **NEW ADDON** | Standalone scenario addon with clean DAG dependency on framework. |
| **Framework scripts**| `tbd-framework/Scripts/Game/TBD/` | `tbd-framework/Scripts/Game/` | **FLATTEN** | Eliminates redundant `TBD/` directory nesting. |
| **Objectives Model**| `Gamemode/Objectives/Model/` | `Gamemode/Objectives/Engine/Model/` | **RELOCATE** | Places shared objective models with engine plumbing. |
| **Objectives Engine**| `Gamemode/Objectives/{Registry,Runtime,Tasks}/` | `Gamemode/Objectives/Engine/` | **CONSOLIDATE** | Unifies lifecycle, task state machine, and HUD publishing. |
| **Pluggable Types** | *Scattered across registry* | `Gamemode/Objectives/Types/` | **NEW MODULAR** | Discoverable per-type directories (`Capture/`, `Destroy/`, etc.). |
| **Planning Anchor** | *Non-existent* | `apps/mod/improved_layout/` | **NEW** | Monorepo gate-compliant planning anchor (`README.md` only). |
| **Framework Prefabs**| `tbd-framework/Prefabs/` | `tbd-framework/Prefabs/` | **RETAIN** | Preserves game mode, player controller, and component prefabs. |
| **Framework UI** | `tbd-framework/UI/` | `tbd-framework/UI/` | **RETAIN** | Preserves design system layouts, textures, icons, and fonts. |
| **Framework Data** | `tbd-framework/Data/` | `tbd-framework/Data/` | **RETAIN** | Preserves core alias registry (`registry.json`) for CAD tool. |
| **Export Addon** | `apps/mod/tbd-export/` | `apps/mod/tbd-export/` | **RETAIN** | Preserves Workbench terrain, equipment, and vehicle exporters. |
| **MCP Addon** | `apps/mod/tbd-emcp/` | `apps/mod/tbd-emcp/` | **RETAIN** | Preserves Workbench Net API automation bridge. |

---

## 3. High-Level Subsystem Topology

```text
apps/mod/
├── README.md                          # Mod suite overview & getting started guide
├── .mcp.json                          # MCP server registration for Workbench
├── .local-test-profile/               # Dev server runtime profile (gitignored)
├── improved_layout/                   # Code-tree planning anchor (README.md only)
│   └── README.md
│
├── References/                        # [NEW] Consolidated upstream references (gitignored)
│   ├── README.md                      # Licensing boundaries & no-leak invariants
│   ├── crf_framework/                 # Coalition Reforger Framework reference
│   └── vanilla_reference/             # Unpacked Bohemia Enfusion scripts reference
│
├── tbd-framework/                     # Core game mod runtime (shipping)
│   ├── README.md
│   ├── addon.gproj                    # GUID: B2C3D4E5F6A78901, depends on vanilla
│   ├── resourceDatabase.rdb
│   ├── Configs/                       # Input actions & chimeraMenus.conf
│   ├── Data/                          # Alias registry (registry.json)
│   ├── Prefabs/                       # GameMode, PlayerController, components
│   ├── Scripts/                       # Flattened: Scripts/Game/ (no TBD/ intermediate)
│   │   ├── README.md
│   │   └── Game/
│   │       ├── README.md
│   │       ├── API/                   # HTTP, Session, Identity, Fleet
│   │       ├── Core/                  # Logging, Math, Authority, Players
│   │       ├── Gamemode/              # Orchestrator, Stages, Objectives
│   │       ├── Session/               # Briefing, Lobby, Spectator, Admin
│   │       ├── Systems/               # Spawning, Loadouts, Radio, Markers, Zones
│   │       └── UI/                    # HUD controllers, menus, toasts
│   └── UI/                            # Layouts (.layout), styles, textures, fonts
│
├── tbd-missions/                      # [NEW ADDON] Standalone scenario & world content
│   ├── README.md                      # Mission authoring & CAD export workflow
│   ├── addon.gproj                    # GUID: E5F6A7B8C9012345, depends on TBD_Framework
│   ├── resourceDatabase.rdb           # Asset registry for missions
│   ├── Missions/                      # Mission headers (.conf, e.g., TBD_Dev_POC.conf)
│   └── worlds/                        # World subscenes (.ent), layer directories, Eden
│
├── tbd-export/                        # Workbench export plugins (Map, Equip, Vehicle)
│   ├── README.md
│   ├── addon.gproj                    # GUID: C3D4E5F6A7B89012, depends on vanilla, TBD_EMCP
│   └── ...
│
└── tbd-emcp/                          # Enfusion MCP Net API handlers
    ├── README.md
    ├── addon.gproj                    # GUID: D4E5F6A7B8C90123, depends on vanilla
    └── ...
```

---

## 4. Addon Dependency Architecture (DAG)

In Enfusion, addons declare strict dependency graphs in their `addon.gproj` files. Cross-dependencies
or circular references cause engine load errors and crash dedicated servers. The reorganized mod suite
forms an explicit Directed Acyclic Graph (DAG):

```text
                  tbd-missions (Missions, worlds, scenarios)
                        │
                        ▼
                  tbd-framework (Logic, prefabs, UI, aliases)
                        │
                        ▼
                  Arma Reforger Vanilla (58D0FB3206B6F859)
```

```text
                  tbd-export (Workbench exporters)
                        │          │
                        │          ▼
                        │     tbd-emcp (MCP Net API handlers)
                        │          │
                        ▼          ▼
                  Arma Reforger Vanilla (58D0FB3206B6F859)
```

### Architectural Properties:
- **No tooling in the shipping runtime**: `tbd-framework` and `tbd-missions` never depend on `tbd-export`
  or `tbd-emcp`. They contain zero `Scripts/WorkbenchGame/` files, ensuring shipping builds carry no editor
  overhead.
- **Pure library separation**: `tbd-framework` contains zero `.conf` mission headers and zero `.ent` worlds.
  It acts as a pure Enforce Script library and prefab framework.
- **Pluggable mission content**: `tbd-missions` depends on `tbd-framework`. Server operators and mission
  authors can create scenarios without modifying the framework codebase.
- **Workbench tooling isolation**: `tbd-export` depends on `tbd-emcp` and vanilla, remaining completely
  decoupled from both `tbd-framework` and `tbd-missions`.

---

## 5. Core Architectural Principles & Invariants

### 5.1. Frozen Class Names (Engine Resource Binding)
In Enfusion, `.et` prefabs, `.layout` UI definitions, `.conf` mission headers, and `.ent` worlds bind to
Enforce Script classes by class name strings. Moving `.c` files into new folders does **not** change their
class identifiers in the global Enforce VM.

The following class names are strictly frozen and must remain intact during all reorganizations:
- `TBD_FrameworkManager`, `TBD_FrameworkRollCall`, `TBD_SafestartManager`, `TBD_EGameStage`
- `TBD_ObjectiveRegistry`, `TBD_ObjectiveRuleResolver`, `TBD_ObjectiveProgression`, `TBD_ObjectiveHudPublisher`
- `TBD_ObjectiveCapture`, `TBD_ObjectiveDestroy`, `TBD_ObjectiveDestroyTargets`, `TBD_ObjectiveHoldUntil`, `TBD_ObjectiveHVT`
- `TBD_TaskStateMachine`, `TBD_Task`, `TBD_TaskSchedule`, `TBD_TaskStruct`
- All UI component classes (`TBD_LobbyScreen`, `TBD_BriefingScreen`, `TBD_AdminScreen`, etc.)

### 5.2. File Length & Complexity Caps
- Every Enforce Script production file must strictly remain **at or under 500 lines** (`cargo xtask verify file-length`).
- Any new objective type or engine module exceeding 500 lines must be decomposed into focused helper classes.

### 5.3. Composable vs. Monolithic Game Modes
In traditional community frameworks like CRF, game modes are structured as monolithic 3,000+ line files
(e.g., `CRF_Rush_Game.c`). TBD Reforger uses a composable, data-driven architecture:
- Scenarios authored in the web CAD tool compose arbitrary mixtures of objective types (e.g. 1 capture zone +
  2 demolition caches + 1 holdout phase).
- The `Gamemode/Objectives/` reorganization separates the generic engine plumbing (`Engine/`) from individual
  objective behaviors (`Types/`), enabling clear unit boundaries and discoverable objective authoring.

---

## 6. Related Documentation

- [Mod root structure](02_mod_root_structure.md) — top-level directory layout and peer addons.
- [References consolidation](03_references_consolidation.md) — isolating reference source archives.
- [Standalone tbd-missions addon](04_tbd_missions_addon.md) — scenario extraction specification.
- [Framework scripts flattening](05_framework_scripts_flattening.md) — `Scripts/Game/` domain organization.
- [Gamemode and objectives architecture](06_gamemode_and_objectives.md) — composable objective architecture.
- [Tooling and CI impact](07_tooling_and_ci_impact.md) — downstream xtask and server gate updates.
- [Phased migration roadmap](08_migration_roadmap.md) — step-by-step rollout plan.
