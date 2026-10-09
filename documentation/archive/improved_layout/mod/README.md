**Status:** live

# Mod suite reorganization specifications

Design proposals, architectural blueprints, and execution roadmaps for restructuring the
`apps/mod` Enfusion mod suite in the TBD Reforger monorepo.

## Contents

```text
documentation/archive/improved_layout/mod/
├── 01_master_overview.md                the primary combined synthesis of all mod suite changes
├── 02_mod_root_structure.md             mod suite root layout, peer addon roles, and References/
├── 03_references_consolidation.md       upstream reference isolation, .gitignore, and licensing gates
├── 04_tbd_missions_addon.md             standalone scenario addon, DAG dependency, and asset registry
├── 05_framework_scripts_flattening.md   Scripts/Game/ domain layout and TBD/ nesting removal
├── 06_gamemode_and_objectives.md        composable objective architecture: Engine/ vs Types/
├── 07_tooling_and_ci_impact.md          downstream xtask mod ops, server profiles, and CI gates
└── 08_migration_roadmap.md              phased rollout plan ensuring zero broken CI or boot gates
```

## How it works

This folder contains the complete, systematic specification for reorganizing `apps/mod`. Each
document addresses a distinct architectural boundary, addon partition, or Enforce Script subsystem.
The proposals are strictly designed to satisfy all monorepo verification gates (`markdown-placement`,
`readme-coverage`, `file-length`, `no-crf-leak`, and `mod compile`).

The plan preserves working Enfusion game logic, respects frozen class names bound to engine assets,
and eliminates unnecessary path nesting while formalizing a clean Directed Acyclic Graph (DAG) across
all addons.

```text
               Addon Dependency DAG
tbd-missions ──▶ tbd-framework ──▶ vanilla (58D0FB3206B6F859)
tbd-export   ──▶ vanilla, tbd-emcp
tbd-emcp     ──▶ vanilla
```

## Documents

1. **[Master overview](01_master_overview.md)**: Combined synthesis of all mod suite changes,
   high-level Before vs. After directory matrix, core design principles, and dependency DAG.
2. **[Mod root structure](02_mod_root_structure.md)**: Top-level layout of `apps/mod/`,
   peer addon boundaries, `.local-test-profile` runtime isolation, and `References/` container creation.
3. **[References consolidation](03_references_consolidation.md)**: Isolating `crf_framework` and
   `vanilla_reference` under `<apps/mod/References/>`, updating `.gitignore`, licensing gates (`no-crf-leak`),
   and slice worktree symlinking.
4. **[Standalone tbd-missions addon](04_tbd_missions_addon.md)**: Extracting `Missions/` and `worlds/`
   into an independent addon with its own `addon.gproj`, GUID, and asset database, decoupled from framework code.
5. **[Framework scripts flattening](05_framework_scripts_flattening.md)**: Removing the redundant
   `TBD/` folder layer in `tbd-framework/Scripts/Game/`, resulting in clean top-level domain paths while
   preserving frozen class names.
6. **[Gamemode and objectives architecture](06_gamemode_and_objectives.md)**: Restructuring `Gamemode/`
   into match flow (`Orchestrator/`), phases (`Stages/`), and composable objectives (`Objectives/Engine/`
   plumbing vs `Objectives/Types/` pluggable capture, destroy, hold, and HVT types).
7. **[Tooling and CI impact](07_tooling_and_ci_impact.md)**: Comprehensive impact analysis on `tools/xtask`
   subcommands (`mod compile`, `mod world-boot`, `dev-bootstrap`), server configs, and CI workflows.
8. **[Migration roadmap](08_migration_roadmap.md)**: Phased rollout steps for future execution sessions
   to guarantee zero regressions, passing compilation, and green headless server boots.

## Boundaries

- Depends on:
  - `apps/mod/` source trees across `tbd-framework`, `tbd-export`, and `tbd-emcp`.
  - `tools/xtask` mod operations and licensing verification gates.
- Used by:
  - Mod developers refactoring Enforce Script subsystems.
  - Autonomous agents implementing slices of the mod reorganization.
- Rules:
  - All specifications must maintain every live document at or under 500 lines (`cargo xtask verify markdown-placement`).
  - No Enforce Script class names bound to `.et`, `.layout`, `.conf`, or `.ent` files may be renamed.
  - Every directory created during execution must carry a standard-compliant `README.md`.

## Related documentation

- [Mod documentation](/documentation/mod/README.md) — index of current mod documents.
- [Code-tree anchor](/documentation/archive/improved_layout/mod_code_tree_anchor.md) — code-tree index file.
- [Mod design authority](/documentation/mod/tbd-framework/mod_design.md) — architectural non-negotiables.
- [Mod slice workflow](/documentation/runbooks/mod_slice_workflow.md) — slice worktree procedures and gates.
