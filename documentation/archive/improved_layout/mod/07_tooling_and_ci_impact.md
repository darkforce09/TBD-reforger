**Status:** archived — see [the restructure program](/documentation/restructure/README.md)

# Tooling and CI impact

**Status:** Proposed design  
**Scope:** `tools_v2/xtask`, server configs, and CI workflows  
**Context:** TBD Reforger platform monorepo  

Detailed specification of the tooling adaptations, server profile adjustments, and CI workflow
updates required to support the reorganized mod suite.

---

## 1. Overview of Impacted Tooling

The TBD Reforger monorepo relies on `cargo xtask` commands to build, verify, and run headless dedicated
server sessions. Because the mod suite reorganization separates `tbd-missions` and isolates `References/`,
several key tooling systems require targeted path and configuration updates:

```text
Downstream Tooling Touchpoints
├── tools_v2/xtask/
│   ├── src/commands/mod_ops/
│   │   ├── compile/               # Compiles game scripts headless
│   │   ├── world_boot/            # Boots development scenario headless
│   │   └── playtest_server/       # Runs local dedicated playtest server
│   ├── src/commands/fetch/        # Extracts vanilla game source & API
│   ├── src/verifications/
│   │   └── licensing/             # Upstream code leak verification (no-crf-leak)
│   └── dedicated_server_profiles/ # Server configuration JSONs
├── apps/fleet_host_agent/         # Staging server scenario deployment & process control
└── .github/workflows/
    └── mod-gates.yml              # CI automation for mod compilation & world boot
```

---

## 2. Adaptation Specifications

### 2.1. Headless Compilation (`cargo xtask mod compile`)
- **Current Behavior**: Compiles scripts in `apps/mod/tbd-framework` using the dedicated server binary
  and verifies zero compile errors.
- **Required Update**:
  - `tbd-framework` compile checks verify the flattened `Scripts/Game/` tree.
  - Add compile verification for `tbd-missions` to ensure its scenario configurations and entities
    compile cleanly against the framework.
  - Update preflight checks (`preflight_with_root`) to verify that `resourceDatabase.rdb` exists in both
    `tbd-framework` and `tbd-missions`.

### 2.2. Headless World Boot (`cargo xtask mod world-boot`)
- **Current Behavior**:
  - Sets up an isolated sandbox directory: `<temp_dir>/addons/tbd-framework`.
  - Reads `addon_guid` from `tbd-framework/addon.gproj`.
  - Boots the server with `scenarioId: "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf"`.
- **Required Update**:
  - The sandbox must symlink **both** addons into `<temp_dir>/addons/`:
    ```text
    <temp_dir>/addons/
    ├── tbd-framework/  -> symlinked to repo apps/mod/tbd-framework
    └── tbd-missions/   -> symlinked to repo apps/mod/tbd-missions
    ```
  - Read and validate the addon GUIDs for both `TBD_Framework` (`B2C3D4E5F6A78901`) and
    `TBD_Missions` (`E5F6A7B8C9012345`).
  - Pass both addons to the server's `-addonsDir` or addon configuration.

### 2.3. Dedicated Server Profiles
`tools_v2/xtask/dedicated_server_profiles/tbd-dev-server.config.json`:
- Contains `scenarioId`: `"{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf"`.
- When loading loose addons, the configuration or launch arguments must list both `TBD_Framework` and
  `TBD_Missions` in the active mods list, allowing the engine's resource manager to resolve the
  scenario GUID from `tbd-missions`.

### 2.4. Licensing Verification (`cargo xtask verify no-crf-leak`)
- **Location**: `tools_v2/xtask/src/verifications/licensing/upstream_code_leaks.rs`.
- **Current Behavior**: Scans `apps/mod/tbd-framework` against `apps/mod/crf_framework`.
- **Required Update**:
  - Update the reference search root to `<apps/mod/References/crf_framework>`.
  - Extend scanning scope to include `<apps/mod/tbd-missions/>` in addition to `tbd-framework`.

### 2.5. Vanilla Source Fetch Commands
- **Location**: `tools_v2/xtask/src/commands/fetch/vanilla_source.rs` and `vanilla_api.rs`.
- **Current Behavior**: Unpacks engine scripts into `apps/mod/vanilla_reference`.
- **Required Update**:
  - Direct extraction to `<apps/mod/References/vanilla_reference>`.
  - Ensure the parent `<apps/mod/References/>` directory is automatically created if missing.

### 2.6. Fleet Host Agent Integration
- **Location**: `apps/fleet_host_agent/src/command_execution/mission_deployment.rs`.
- **Current Behavior**: Manages dedicated server execution and scenario deployment on staging hosts.
- **Required Update**:
  - When staging host agents deploy scenarios, both `tbd-framework` and `tbd-missions` are deployed to
    the server's addons directory (`~/.local/share/ArmaReforgerServer/addons/`).

### 2.7. GitHub Actions CI (`.github/workflows/mod-gates.yml`)
- Ensure runner setup steps verify both `tbd-framework/resourceDatabase.rdb` and
  `tbd-missions/resourceDatabase.rdb`.
- Execute `cargo xtask mod compile` and `cargo xtask mod world-boot` with the updated multi-addon
  pipeline.

---

## 3. Tooling Verification Matrix

| Tooling Command | Target Addon(s) | Verification Condition |
|---|---|---|
| `cargo xtask mod compile` | `tbd-framework`, `tbd-missions` | Exit code 0, 0 compile errors |
| `cargo xtask mod world-boot` | `tbd-framework` + `tbd-missions` | Logs `[TBD][Mission] loaded id=`, exit 0 |
| `cargo xtask verify no-crf-leak` | `References/crf_framework` vs production | 0 matching symbol leaks |
| `cargo xtask verify markdown-placement` | Entire repository | All code trees hold only README.md |
| `cargo xtask verify readme-coverage` | All mod directories | 100% compliant README coverage |

---

## 4. Related Documentation

- [Master overview](01_master_overview.md) — mod reorganization synthesis.
- [Standalone tbd-missions addon](04_tbd_missions_addon.md) — mission addon architecture.
- [References consolidation](03_references_consolidation.md) — reference directory isolation.
- [Migration roadmap](08_migration_roadmap.md) — phased implementation sequence.
