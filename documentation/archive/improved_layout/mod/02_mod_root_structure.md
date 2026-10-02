**Status:** archived — see [the restructure program](/documentation/restructure/README.md)

# Mod suite root structure

**Status:** Proposed design  
**Scope:** `apps/mod` directory root and peer addon organization  
**Context:** TBD Reforger platform monorepo  

Detailed specification for the top-level directory structure of `apps/mod`, defining peer addon
boundaries, reference isolation, and local development configurations.

---

## 1. Motivation & Current Root Issues

The `apps/mod` directory currently houses a mixture of shipping code, development tooling, upstream
reference archives, and local runtime profiles directly at its root:

```text
apps/mod/ (Current)
├── .local-test-profile/       # Local dedicated server dev profile
├── .mcp.json                  # MCP server registration for Workbench
├── README.md                  # Mod suite overview
├── crf_framework/             # Unmanaged upstream reference directory
├── tbd-emcp/                  # Enfusion MCP Net API addon
├── tbd-export/                # Workbench export plugin addon
├── tbd-framework/             # Monolithic game mod (runtime + scenarios + worlds)
└── vanilla_reference/         # Unpacked Bohemia engine scripts reference
```

This root layout exhibits several structural weaknesses:
1. **Uncontained reference directories**: Upstream reference sources (`crf_framework` and
   `vanilla_reference`) sit alongside shipping addons. While excluded by `.gitignore`, their presence
   at the root level clutters directory browsing and complicates workspace discovery.
2. **Missing mission addon partition**: Scenario content (`Missions/`, `worlds/`) is trapped inside
   `tbd-framework`, preventing clean distribution of scenarios independently of core framework updates.
3. **No dedicated reference policy anchor**: There is no dedicated README documenting the strict
   licensing invariants and isolation rules governing upstream reference code.

---

## 2. Proposed Root Topology

The proposed root structure organizes `apps/mod` into four distinct categories:
- **Shipping Addons**: `tbd-framework` and `tbd-missions`.
- **Workbench Authoring Addons**: `tbd-export` and `tbd-emcp`.
- **Isolated Reference Archives**: `References/`.
- **Planning & Tooling Anchors**: `improved_layout/`, `.mcp.json`, and `.local-test-profile/`.

```text
apps/mod/ (Proposed)
├── README.md                  # High-level suite guide & Getting Started
├── .mcp.json                  # MCP server registration for Workbench
├── .local-test-profile/       # Local dedicated server runtime profile (gitignored)
│
├── improved_layout/           # Planning anchor (README.md only, gate-compliant)
│   └── README.md              # Points to documentation_v2/mod/improved_layout/
│
├── References/                # [NEW CONTAINER] Upstream reference archives (gitignored)
│   ├── README.md              # Licensing rules, no-crf-leak invariants, oracle lanes
│   ├── crf_framework/         # Upstream Coalition Reforger Framework reference
│   └── vanilla_reference/     # Unpacked Bohemia Enfusion vanilla reference
│
├── tbd-framework/             # Core shipping runtime game mod
│   ├── README.md
│   ├── addon.gproj            # GUID: B2C3D4E5F6A78901
│   ├── resourceDatabase.rdb
│   ├── Configs/               # Input actions and system menu configurations
│   ├── Data/                  # Core prefab alias registry (registry.json)
│   ├── Prefabs/               # GameMode, PlayerController, and component prefabs
│   ├── Scripts/               # Flattened Enforce Script hierarchy (Scripts/Game/)
│   └── UI/                    # Layouts, styles, textures, fonts
│
├── tbd-missions/              # [NEW ADDON] Standalone scenario & world content
│   ├── README.md              # Mission authoring & scenario manifest guide
│   ├── addon.gproj            # GUID: E5F6A7B8C9012345
│   ├── resourceDatabase.rdb   # Asset database for scenario resources
│   ├── Missions/              # SCR_MissionHeader configs (.conf)
│   └── worlds/                # World subscenes (.ent), layers, Eden
│
├── tbd-export/                # Workbench export plugin suite (Map, Equip, Vehicle)
│   ├── README.md
│   ├── addon.gproj            # GUID: C3D4E5F6A7B89012
│   └── ...
│
└── tbd-emcp/                  # Enfusion MCP Net API handlers
    ├── README.md
    ├── addon.gproj            # GUID: D4E5F6A7B8C90123
    └── ...
```

---

## 3. Peer Addon Roles & Responsibility Boundaries

### 3.1. `tbd-framework` (Core Game Runtime)
- **Role**: The shipping game mod loaded by dedicated servers and connected clients.
- **Responsibility**: Contains all reusable gameplay logic, network RPC handlers, UI presentation
  layers, mission JSON deserialization, spawning pipeline, loadout enforcement, radio frequencies,
  and composable objective engines.
- **Invariants**:
  - Contains **zero** scenario mission headers (`Missions/*.conf`) and **zero** world files (`worlds/*.ent`).
  - Contains **zero** Workbench scripts (`Scripts/WorkbenchGame/`).
  - Never depends on `tbd-export` or `tbd-emcp`.
  - Depends only on the vanilla Arma Reforger base game (`58D0FB3206B6F859`).

### 3.2. `tbd-missions` (Standalone Scenario Content)
- **Role**: Scenario and world packaging addon.
- **Responsibility**: Houses all playable scenario mission headers, terrain subscene layers, Eden
  configurations, and world entities.
- **Invariants**:
  - Contains **zero** core gameplay scripts or engine systems.
  - Declares dependency on `tbd-framework` (`B2C3D4E5F6A78901`) and vanilla (`58D0FB3206B6F859`).
  - Scenarios reference prefabs and components defined in `tbd-framework`.

### 3.3. `tbd-export` (Workbench Export Suite)
- **Role**: Editor-only tooling suite for exporting data consumed by the TBD web platform.
- **Responsibility**: Contains plugins and scripts for exporting terrain elevation/satellite data,
  equipment catalogues, vehicle definitions, and prefab alias registries.
- **Invariants**:
  - Runs exclusively within Arma Reforger Workbench.
  - Never loaded on dedicated servers or distributed to players.
  - Depends on `tbd-emcp` and vanilla; never depends on `tbd-framework`.

### 3.4. `tbd-emcp` (Enfusion MCP Bridge)
- **Role**: Net API handlers enabling AI agents and external tools to automate Workbench.
- **Responsibility**: Implements `EMCP_` handlers called by the `enfusion-mcp` Node daemon.
- **Invariants**:
  - The handlers exist once in `apps/mod/tbd-emcp/`.
  - Runs exclusively inside Workbench.
  - Depends only on vanilla.

---

## 4. Root Configuration & Dev Profiles

### 4.1. `.mcp.json`
Located at `apps/mod/.mcp.json`, this configuration registers the Enfusion MCP server for IDE and
agent sessions opened within the mod workspace. It executes `cargo xtask mcp` tooling to bridge
external commands into Workbench.

### 4.2. `.local-test-profile/`
Generated by `cargo xtask setup server-profile`, this directory holds local dedicated server
credentials, ports, and runtime logs for development testing. It is excluded by `.gitignore` and
never committed.

---

## 5. Updates to Root `apps/mod/README.md`

Upon executing the migration, the root `apps/mod/README.md` will be updated to:
1. Include `tbd-missions` in the addon directory list and dependency table.
2. Document `References/` as the consolidated home of upstream source archives.
3. Update the Getting Started command table to reflect the presence of both `tbd-framework` and
   `tbd-missions`.
4. Point to `improved_layout/` and `documentation_v2/mod/improved_layout/` for architectural details.

---

## 6. Related Documentation

- [Master overview](01_master_overview.md) — combined synthesis of all mod suite changes.
- [References consolidation](03_references_consolidation.md) — isolating reference source archives.
- [Standalone tbd-missions addon](04_tbd_missions_addon.md) — scenario extraction specification.
- [Framework scripts flattening](05_framework_scripts_flattening.md) — `Scripts/Game/` domain organization.
