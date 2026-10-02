**Status:** archived — see [the restructure program](/documentation/restructure/README.md)

# Upstream references consolidation

**Status:** Proposed design  
**Scope:** Upstream reference archives in `<apps/mod/References/>` and related tooling  
**Context:** TBD Reforger platform monorepo  

Detailed specification for consolidating upstream reference directories (`crf_framework` and
`vanilla_reference`) into a dedicated `<apps/mod/References/>` container, including gitignore updates,
licensing isolation, and tooling gate adaptations.

---

## 1. Background & Rationale

During the development of the TBD Reforger platform, two external codebases serve as oracle lanes:
1. **`crf_framework`**: The Coalition Reforger Framework upstream repository, used as a functional
   reference for mission mechanics, slotting patterns, and game mode logic.
2. **`vanilla_reference`**: Unpacked Enforce Script sources from Bohemia Interactive's base Arma
   Reforger game, used as an API lookup and engine behavior reference.

Previously, these two archives resided directly under `apps/mod/` (`apps/mod/crf_framework` and
`apps/mod/vanilla_reference`). While excluded from version control via `.gitignore`, this placement
exposes several risks:
- **Visual and cognitive clutter**: Reference folders mix with first-party shipping addons.
- **Accidental Workbench loading**: Workbench can inadvertently detect root-level directories as addons
  if an `addon.gproj` is detected, creating GUID conflicts or schema pollution.
- **Tooling path fragmentation**: Verification gates, slice worktree automation, and fetch scripts
  each hardcode separate paths to these folders.

Consolidating them under `<apps/mod/References/>` provides a unified, gated boundary for all non-shipping
reference materials.

---

## 2. Directory Structure of `References/`

```text
apps/mod/References/
├── README.md                  # Reference policy, licensing rules & oracle lane guide
├── crf_framework/             # Cloned Coalition Reforger Framework repository (gitignored)
└── vanilla_reference/         # Unpacked Arma Reforger base game scripts (gitignored)
```

### 2.1. Invariants & Licensing Rules
- **Strictly Read-Only**: Code within `References/` is never modified by TBD development sessions.
- **Never Opened in Workbench**: Reference folders must never be loaded as project roots in Bohemia's
  Workbench tools.
- **Never Shipped**: No files, scripts, or assets from `References/` may ever be packaged into `.pak`
  archives or deployed to game servers.
- **No Upstream Code Leaks**: All first-party code in `tbd-framework` must be authored cleanly.
  `cargo xtask verify no-crf-leak` scans production addons to guarantee zero GPL-licensed or upstream
  code snippets leak into TBD repositories.

---

## 3. Impact Analysis & Necessary Updates

### 3.1. `.gitignore` Updates
The root `.gitignore` currently excludes the old paths directly under `/apps/mod/`. These entries
must be updated to reflect the new `References/` container:

```diff
  # ==============================================================================
  # 7. Mod Suite (apps/mod)
  # ==============================================================================
  # Oracle lanes & reference sources (NO trailing slashes: in worktrees these are symlinks,
  # and trailing slashes would allow symlinks to be staged)
- /apps/mod/crf_framework
- /apps/mod/vanilla_reference
+ /apps/mod/References/crf_framework
+ /apps/mod/References/vanilla_reference
  /apps/mod/playable_selector
  /apps/mod/Tbd_framework
```

### 3.2. Verification Gate: Upstream Code Leaks
`tools_v2/xtask/src/verifications/licensing/upstream_code_leaks.rs` scans production code to ensure
no upstream code has been copied:
- The path constants pointing to the reference roots must be updated to inspect
  `<apps/mod/References/crf_framework>` when generating comparative symbol fingerprints.
- Test fixtures in `tools_v2/xtask/src/verifications/licensing/tests/` must be validated against the
  new path structure.

### 3.3. Vanilla Source & API Fetch Tooling
`tools_v2/xtask/src/commands/fetch/vanilla_source.rs` and `vanilla_api.rs`:
- These commands automate extracting vanilla scripts from the installed Steam game files.
- Destination directories must be updated from `apps/mod/vanilla_reference` to
  `<apps/mod/References/vanilla_reference>`.

### 3.4. Slice Worktree Automation
`tools_v2/xtask/src/commands/platform/slice_worktree/git_plain.rs`:
- When spawning isolated git worktrees for refactor slices, the worktree driver symlinks reference
  folders to avoid re-cloning large trees.
- The symlink path logic must be updated to target `<apps/mod/References/>`.

### 3.5. Developer Tools & Carve Tooling
`tools_v2/developer-tools/src/enfusion_tooling/carve.rs` and `cli.rs`:
- CLI paths querying vanilla AST or symbol indexes must resolve against `<apps/mod/References/>`.

### 3.6. Documentation Citations
Cross-references across `CLAUDE.md`, `documentation_v2/mod/README.md`, `documentation_v2/runbooks/`,
and `documentation_v2/standards/` will be updated to cite `<apps/mod/References/>`.

---

## 4. Specification for `<apps/mod/References/README.md>`

When `<apps/mod/References/>` is created, it will contain a standard-compliant `README.md`:

```markdown
# Upstream references

Upstream source archives and oracle lanes for the TBD Reforger mod suite. Contains no production
code. All subdirectories are gitignored reference sources.

## Contents

```text
apps/mod/References/
├── crf_framework/             Coalition Reforger Framework reference source
└── vanilla_reference/         Unpacked Bohemia Interactive Enfusion scripts
```

## How it works

This directory provides read-only reference material for mod developers and AI agents investigating
Enfusion API contracts, vanilla entity behaviors, and community framework patterns. No files in this
directory are built, packaged, or shipped.

## Boundaries

- Depends on: external upstream source repositories and local Steam installations.
- Used by: developer reference tools, AST carvers, and license verification gates.
- Rules:
  - Reference files are strictly read-only and never staged in git.
  - Never open this folder or its children in Arma Reforger Workbench.
  - No reference code or GUIDs may be copied into shipping addons (`cargo xtask verify no-crf-leak`).
```

---

## 5. Related Documentation

- [Master overview](01_master_overview.md) — mod reorganization synthesis.
- [Mod root structure](02_mod_root_structure.md) — top-level directory layout.
- [Tooling and CI impact](07_tooling_and_ci_impact.md) — detailed xtask gate adaptations.
- [Migration roadmap](08_migration_roadmap.md) — step-by-step rollout sequence.
