# Phased migration roadmap

**Status:** Proposed design  
**Scope:** Execution roadmap for `apps/mod` reorganization  
**Context:** TBD Reforger platform monorepo  

A step-by-step, zero-downtime execution roadmap for reorganizing the `apps/mod` suite across future
implementation sessions, with explicit verification gates and rollback procedures for each phase.

---

## 1. Migration Strategy & Principles

To maintain repository health, avoid breaking CI pipelines, and prevent regressions in dedicated
server execution, the reorganization is divided into five isolated, atomic phases:

```text
Phase 1: References Consolidation
      │
      ▼
Phase 2: Standalone tbd-missions Addon
      │
      ▼
Phase 3: Framework Scripts Flattening
      │
      ▼
Phase 4: Gamemode & Objectives Restructuring
      │
      ▼
Phase 5: Full Monorepo Verification & Gate Audit
```

Each phase must produce a clean git commit that passes all verification gates before the next phase
begins.

---

## 2. Phased Rollout Plan

### Phase 1: References Consolidation
- **Objective**: Move `crf_framework` and `vanilla_reference` out of the mod root into `<apps/mod/References/>`.
- **Execution Steps**:
  1. Create `<apps/mod/References/>` and author its standard-compliant `README.md`.
  2. Move `apps/mod/crf_framework/` $\rightarrow$ `<apps/mod/References/crf_framework/>`.
  3. Move `apps/mod/vanilla_reference/` $\rightarrow$ `<apps/mod/References/vanilla_reference/>`.
  4. Update `.gitignore` with the new `/apps/mod/References/` paths.
  5. Update `tools_v2/xtask/src/verifications/licensing/upstream_code_leaks.rs` search roots.
  6. Update fetch tooling in `tools_v2/xtask/src/commands/fetch/` and slice worktree scripts in
     `tools_v2/xtask/src/commands/platform/slice_worktree/git_plain.rs`.
- **Verification Gate**:
  ```bash
  cargo xtask verify no-crf-leak
  git status  # Verify no reference files are unstaged or untracked
  ```
- **Rollback**: Revert path edits and move directories back to root.

---

### Phase 2: Standalone `tbd-missions` Addon Extraction
- **Objective**: Extract scenario headers and world subscenes out of `tbd-framework` into `tbd-missions`.
- **Execution Steps**:
  1. Create directory `<apps/mod/tbd-missions/>`.
  2. Author `<apps/mod/tbd-missions/addon.gproj>` declaring dependencies on `TBD_Framework` (`B2C3D4E5F6A78901`)
     and vanilla (`58D0FB3206B6F859`).
  3. Scaffold `resourceDatabase.rdb` for `tbd-missions`.
  4. Move `apps/mod/tbd-framework/Missions/` $\rightarrow$ `<apps/mod/tbd-missions/Missions/>`.
  5. Move `apps/mod/tbd-framework/worlds/` $\rightarrow$ `<apps/mod/tbd-missions/worlds/>`.
  6. Author `README.md` files for `tbd-missions/`, `Missions/`, and `worlds/`.
  7. Update `tools_v2/xtask/src/commands/mod_ops/world_boot/execution.rs` to symlink both `tbd-framework`
     and `tbd-missions` into the headless server sandbox.
  8. Update `tools_v2/xtask/dedicated_server_profiles/tbd-dev-server.config.json` if required.
- **Verification Gate**:
  ```bash
  cargo xtask mod world-boot  # Must boot headless, load TBD_Dev_POC, and exit 0 PASS
  ```
- **Rollback**: Move `Missions/` and `worlds/` back into `tbd-framework/`, delete `tbd-missions/`.

---

### Phase 3: Framework Scripts Flattening
- **Objective**: Eliminate the redundant `TBD/` folder layer in `tbd-framework/Scripts/Game/TBD/`.
- **Execution Steps**:
  1. Move `apps/mod/tbd-framework/Scripts/Game/TBD/*` directly into `apps/mod/tbd-framework/Scripts/Game/`.
  2. Remove the empty `apps/mod/tbd-framework/Scripts/Game/TBD/` folder.
  3. Update `apps/mod/tbd-framework/Scripts/README.md` and `Scripts/Game/README.md`.
  4. Perform a search-and-replace for internal docstrings or comments citing `Scripts/Game/TBD/`.
- **Verification Gate**:
  ```bash
  cargo xtask mod compile  # Must compile all game scripts with 0 errors
  ```
- **Rollback**: Move domains back under `Scripts/Game/TBD/`.

---

### Phase 4: Gamemode & Objectives Restructuring
- **Objective**: Restructure `Gamemode/Objectives/` into generic `Engine/` and pluggable `Types/`.
- **Execution Steps**:
  1. Create `Gamemode/Objectives/Engine/` and move:
     - `Registry/` $\rightarrow$ `Engine/`
     - `Runtime/` $\rightarrow$ `Engine/`
     - `Tasks/` $\rightarrow$ `Engine/`
     - `Model/` $\rightarrow$ `Engine/Model/`
  2. Create `Gamemode/Objectives/Types/` and establish subdirectories:
     - `Capture/` (`TBD_ObjectiveCapture.c`)
     - `Destroy/` (`TBD_ObjectiveDestroy.c`, `TBD_ObjectiveDestroyTargets.c`)
     - `HoldUntil/` (`TBD_ObjectiveHoldUntil.c`)
     - `HVT/` (`TBD_ObjectiveHVT.c`)
  3. Author compliant `README.md` files for `Engine/`, `Types/`, and each objective type folder.
  4. Ensure no Enforce Script classes are renamed (preserving engine asset bindings).
  5. Check that all `.c` files remain $\le 500$ lines.
- **Verification Gate**:
  ```bash
  cargo xtask mod compile
  cargo xtask verify file-length
  cargo xtask mod world-boot
  ```
- **Rollback**: Restore the previous `Objectives/{Model, Registry, Runtime, Tasks}` layout.

---

### Phase 5: Full Monorepo Verification & Documentation Cleanup
- **Objective**: Audit all verification gates and update top-level repository indexes.
- **Execution Steps**:
  1. Update `apps/mod/README.md` to reflect the 4 peer addons and `References/`.
  2. Update `documentation_v2/mod/README.md` to link to `improved_layout/` specifications.
  3. Run the full verification suite.
- **Final Verification Gate**:
  ```bash
  cargo xtask verify markdown-placement
  cargo xtask verify readme-coverage
  cargo xtask verify link-check
  cargo xtask verify no-crf-leak
  cargo xtask mod compile
  cargo xtask mod world-boot
  ```

---

## 3. Related Documentation

- [Master overview](01_master_overview.md) — complete architectural synthesis.
- [Mod root structure](02_mod_root_structure.md) — top-level layout.
- [References consolidation](03_references_consolidation.md) — references isolation plan.
- [Standalone tbd-missions addon](04_tbd_missions_addon.md) — mission addon details.
- [Framework scripts flattening](05_framework_scripts_flattening.md) — scripts flattening details.
- [Gamemode and objectives architecture](06_gamemode_and_objectives.md) — objectives architecture.
- [Tooling and CI impact](07_tooling_and_ci_impact.md) — tooling and CI updates.
