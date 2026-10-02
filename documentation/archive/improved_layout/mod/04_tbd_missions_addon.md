**Status:** archived — see [the restructure program](/documentation/restructure/README.md)

# Standalone tbd-missions addon

**Status:** Proposed design  
**Scope:** New addon `<apps/mod/tbd-missions>` and scenario content migration  
**Context:** TBD Reforger platform monorepo  

Detailed specification for creating the standalone `<apps/mod/tbd-missions/>` addon and extracting
all scenario mission headers (`Missions/`) and world subscenes (`worlds/`) out of `tbd-framework`.

---

## 1. Motivation: Library vs. Scenario Content Separation

In the Enfusion engine ecosystem, coupling core game mechanics with specific scenario assets inside
a single addon causes several major architectural issues:
1. **Unnecessary mod update cycles**: Any scenario edit, terrain subscene tweak, or layer adjustment
   forces a rebuild and republish of the entire framework mod, invalidating client caches.
2. **Circular dependency hazard**: If a world scene references framework prefabs while the framework
   references that world scene in its test configs, the boundary between library code and scenario
   data becomes blurred.
3. **Violated Single Responsibility Principle**: `tbd-framework` is a gameplay engine and UI runtime;
   it should not be burdened with gigabytes of world geometry, layer entities, or scenario headers.

By extracting scenarios into a dedicated `tbd-missions` addon, we establish a clean separation of
concerns following industry best practices.

---

## 2. Directory Layout of `tbd-missions/`

```text
apps/mod/tbd-missions/
├── README.md                  # Addon purpose, scenario authoring & CAD export guide
├── addon.gproj                # Enfusion project definition (depends on TBD_Framework)
├── resourceDatabase.rdb       # Workbench resource index for mission assets
│
├── Missions/                  # Playable mission headers (.conf and .meta)
│   ├── README.md              # Header configuration standards
│   ├── TBD_Dev_POC.conf       # Development POC mission header (SCR_MissionHeader)
│   └── TBD_Dev_POC.conf.meta  # Enfusion resource metadata (GUID: 69A85365FC09E2CA)
│
└── worlds/                    # World subscenes, layer folders & Eden presets
    ├── README.md              # Subscene layer organization guide
    ├── Eden/                  # World-specific terrain configs
    │   └── README.md
    ├── TBD_Dev_POC.ent        # Development subscene root entity file
    ├── TBD_Dev_POC.ent.meta   # Subscene metadata (GUID: F652B97A6F497348)
    └── TBD_Dev_POC_Layers/    # Layer entity files (GameMode, Spawns, Triggers)
        ├── README.md
        ├── default.layer
        ├── gamemode.layer
        └── spawns.layer
```

---

## 3. Project Configuration: `addon.gproj`

The `addon.gproj` file for `tbd-missions` explicitly declares its dependency on `TBD_Framework` and
the vanilla base game:

```text
GameProject {
 ID "TBD_Missions"
 GUID "E5F6A7B8C9012345"
 TITLE "TBD Missions"
 Dependencies {
  "58D0FB3206B6F859"   // Arma Reforger base game
  "B2C3D4E5F6A78901"   // TBD_Framework
 }
 Configurations {
  GameProjectConfig PC {
  }
  GameProjectConfig HEADLESS {
  }
 }
}
```

### Dependency Properties:
- **Clean DAG**: `tbd-missions` $\rightarrow$ `tbd-framework` $\rightarrow$ vanilla.
- **Inherited Assets**: `tbd-missions` can reference all prefabs (`Prefabs/`), UI layouts (`UI/`),
  and script components defined in `tbd-framework`.
- **Zero Reverse Dependency**: `tbd-framework` has **zero** dependencies on `tbd-missions`.

---

## 4. Content Extracted from `tbd-framework`

### 4.1. `Missions/` Directory
- **Files moved**:
  - `apps/mod/tbd-framework/Missions/TBD_Dev_POC.conf` $\rightarrow$ `<apps/mod/tbd-missions/Missions/>`
  - `apps/mod/tbd-framework/Missions/TBD_Dev_POC.conf.meta` $\rightarrow$ `<apps/mod/tbd-missions/Missions/>`
- **GUID Preservation**: The `.conf.meta` retains its exact GUID (`{69A85365FC09E2CA}`). Because
  Enfusion resolves resources by GUID, dedicated servers and configs referencing this header continue
  functioning seamlessly once `tbd-missions` is loaded.

### 4.2. `worlds/` Directory
- **Files moved**:
  - `apps/mod/tbd-framework/worlds/TBD_Dev_POC.ent` $\rightarrow$ `<apps/mod/tbd-missions/worlds/>`
  - `apps/mod/tbd-framework/worlds/TBD_Dev_POC.ent.meta` $\rightarrow$ `<apps/mod/tbd-missions/worlds/>`
  - `apps/mod/tbd-framework/worlds/TBD_Dev_POC_Layers/*` $\rightarrow$ `<apps/mod/tbd-missions/worlds/TBD_Dev_POC_Layers/>`
  - `apps/mod/tbd-framework/worlds/Eden/*` $\rightarrow$ `<apps/mod/tbd-missions/worlds/Eden/>`
- **GUID Preservation**: `TBD_Dev_POC.ent.meta` retains its GUID (`{F652B97A6F497348}`).
  Internal entity references pointing to framework prefabs (`{...}Prefabs/GameMode/...`) resolve
  via the `TBD_Framework` dependency.

---

## 5. Relationship with Web Platform & CAD Mission Creator

The TBD Reforger web platform features a CAD Mission Creator (`apps/website/frontend/src/v2/apps/editor/`):
1. **Dynamic Mission Deployment**: The web platform deploys scenarios as validated JSON contracts
   (`contracts_v2/definitions/mission.schema.json`).
2. **Framework Boot Role**: Dedicated servers boot a generic container world (`TBD_Dev_POC.ent` in
   `tbd-missions`), and the framework's HTTP client dynamically queries the backend API to fetch,
   verify, and materialize the specific mission JSON payload.
3. **Curated Scenarios**: Scenarios with custom terrain modifications, persistent base structures, or
   pre-placed static props are authored as `.ent` subscenes directly inside `tbd-missions/worlds/`.

---

## 6. Server Loading Model & Profile Adaptation

### 6.1. Headless Dedicated Server Boot
When booting a dedicated server headless (via `cargo xtask mod world-boot` or `mod playtest`), the
server must load both addons:
```bash
ArmaReforgerServer -addonsDir=/path/to/addons -addonIds=B2C3D4E5F6A78901,E5F6A7B8C9012345
```
Or when using the loose addon symlink tree in the test sandbox:
```text
<temp_run_dir>/addons/
├── tbd-framework/  -> symlinked to apps/mod/tbd-framework
└── tbd-missions/   -> symlinked to apps/mod/tbd-missions
```

### 6.2. Server Profile: `tbd-dev-server.config.json`
`tools_v2/xtask/dedicated_server_profiles/tbd-dev-server.config.json` specifies:
```json
"scenarioId": "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf"
```
Because `TBD_Dev_POC.conf` retains its GUID (`69A85365FC09E2CA`), Enfusion locates the scenario header
within `tbd-missions` as long as both addons are registered in the server's addon search path.

---

## 7. Retained Framework Boundaries

Following this extraction, `tbd-framework` is completely clean of scenario files and retains only:
- `Configs/`: Input actions, menu presets (`chimeraMenus.conf`).
- `Data/`: Core prefab alias spawn registry (`registry.json`).
- `Prefabs/`: Reusable game entities, managers, controllers.
- `Scripts/`: Core Enforce Script engines (flattened to `Scripts/Game/`).
- `UI/`: Design system layouts, textures, fonts.

---

## 8. Related Documentation

- [Master overview](01_master_overview.md) — mod suite synthesis and DAG architecture.
- [Mod root structure](02_mod_root_structure.md) — peer addon boundaries.
- [Tooling and CI impact](07_tooling_and_ci_impact.md) — server boot and xtask updates.
- [Migration roadmap](08_migration_roadmap.md) — step-by-step rollout plan.
