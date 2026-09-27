# Framework scripts flattening

**Status:** Proposed design  
**Scope:** `tbd-framework/Scripts/Game/` directory organization  
**Context:** TBD Reforger platform monorepo  

Detailed specification for eliminating the redundant `TBD/` directory nesting in
`tbd-framework/Scripts/Game/TBD/` to establish a clean, direct domain structure under
`tbd-framework/Scripts/Game/`.

---

## 1. Problem Statement: Redundant Directory Nesting

The Enforce Script source tree in `tbd-framework` currently nests all gameplay scripts four levels
deep before reaching any functional domain:

```text
apps/mod/tbd-framework/Scripts/Game/TBD/ (Current)
├── API/                       # Backend HTTP and session services
├── Core/                      # Authority, logging, math, player primitives
├── Gamemode/                  # Orchestrator, stages, objectives
├── Session/                   # Briefing, lobby, spectator, admin
├── Systems/                   # Spawning, loadouts, radio, markers, zones
└── UI/                        # HUD and menu script controllers
```

### Why this nesting is redundant:
1. **Enfusion Engine Semantics**: The Enfusion script engine partitions scripts into engine-level
   scopes: `Scripts/Game/` (runtime game logic) and `Scripts/WorkbenchGame/` (editor plugins). Any
   subfolder under `Scripts/Game/` is automatically recursively scanned and compiled into the global
   Enforce VM namespace.
2. **Global Class Prefixes**: Enforce Script does not support C++ or Rust-style nested namespace
   keywords. Every class in the codebase is already explicitly prefixed with `TBD_` (e.g.,
   `TBD_Authority`, `TBD_FrameworkManager`, `TBD_ObjectiveRegistry`).
3. **Gratuitous Path Depth**: Nesting `TBD/` inside `Game/` adds unnecessary path friction:
   `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Registry/...` is 7 levels deep before
   reaching a source file.

---

## 2. Proposed Flattened Layout

By removing `TBD/`, the functional domains become first-class children of `Scripts/Game/`:

```text
apps/mod/tbd-framework/Scripts/Game/ (Proposed)
├── README.md                  # Enforce Script standards, compilation rules & domain guide
├── API/                       # Backend HTTP, session lifecycle, fleet commands
│   ├── README.md
│   ├── FleetCommands/
│   ├── Http/
│   ├── Identity/
│   ├── Results/
│   └── RuntimeSession/
│
├── Core/                      # Runtime primitives, authority, logging, math
│   ├── README.md
│   ├── Characters/
│   ├── Factions/
│   ├── Hashing/
│   ├── Logging/
│   ├── Math/
│   ├── Players/
│   ├── Time/
│   ├── Wire/
│   ├── World/
│   ├── TBD_Authority.c
│   ├── TBD_Log.c
│   ├── TBD_PlayerChat.c
│   ├── TBD_Registry.c
│   └── TBD_RegistryPocComponent.c
│
├── Gamemode/                  # Orchestrator, stages, and composable objectives
│   ├── README.md
│   ├── Orchestrator/          # Match lifecycle & heartbeat
│   ├── Stages/                # Safestart & win condition evaluation
│   └── Objectives/            # Engine plumbing & pluggable types
│
├── Session/                   # Player lobby, briefing, admin, spectator
│   ├── README.md
│   ├── Admin/
│   ├── Briefing/
│   ├── Lobby/
│   ├── MissionSelector/
│   ├── Players/
│   ├── PostGame/
│   └── Spectator/
│
├── Systems/                   # Gameplay systems & world simulation
│   ├── README.md
│   ├── AI/
│   ├── Audio/
│   ├── Loadouts/
│   ├── Markers/
│   ├── Mission/
│   ├── Radio/
│   ├── Spawning/
│   └── Zones/
│
└── UI/                        # UI presentation controllers & widgets
    ├── README.md
    ├── Admin/
    ├── Common/
    ├── Core/
    ├── Hud/
    ├── Lobby/
    ├── Navigation/
    └── Screens/
```

---

## 3. Preserving Frozen Class Names & Asset Bindings

A critical requirement in Enfusion modding is ensuring asset references remain intact:
- `.et` prefabs (e.g., `Prefabs/GameMode/TBD_GameMode.et`) bind to script components via class name
  strings (e.g., `ComponentType: "TBD_ObjectivesComponent"`).
- `.layout` files bind to event handler classes (e.g., `HandlerClassName: "TBD_LobbyScreen"`).
- `.conf` mission headers bind to game mode and header classes.

Because Enforce Script resolves classes globally by symbol name, **moving `.c` files to different
folders does not change their symbol identity**. As long as class names are preserved exactly as
frozen during the T-1092 modularization program, zero prefab or layout re-linking is required.

### Mandatory Invariant:
No class, method, or enum symbol will be renamed during this directory flattening.

---

## 4. Documentation & Verification Updates

### 4.1. `README.md` Coverage
Each directory under `Scripts/Game/` must maintain a standard-compliant `README.md` meeting repository
standards:
- `Scripts/README.md`: Explaining the role of `Scripts/Game/` (runtime only, no `WorkbenchGame/`).
- `Scripts/Game/README.md`: Index of the 6 core domains (`API/`, `Core/`, `Gamemode/`, `Session/`,
  `Systems/`, `UI/`).
- Domain `README.md` files: Updated to describe the direct domain topology.

### 4.2. Internal Citations
Any docstrings, comment blocks, or citations referencing `Scripts/Game/TBD/...` will be updated to
reflect `Scripts/Game/...`.

### 4.3. Script Compilation Verification
The flattening will be verified with:
```bash
cargo xtask mod compile
```
This headless compiler runs the Linux dedicated server binary and confirms that all 370+ Enforce
Script files compile with zero syntax errors, zero unresolved symbols, and zero warnings.

---

## 5. Related Documentation

- [Master overview](01_master_overview.md) — mod reorganization synthesis.
- [Mod root structure](02_mod_root_structure.md) — peer addon boundaries.
- [Gamemode and objectives architecture](06_gamemode_and_objectives.md) — detailed `Gamemode/` layout.
- [Tooling and CI impact](07_tooling_and_ci_impact.md) — compilation gate requirements.
- [Migration roadmap](08_migration_roadmap.md) — step-by-step rollout plan.
