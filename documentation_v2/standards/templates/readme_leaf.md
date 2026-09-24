**Status:** live

# README template: leaf

**When to use:** a folder with no child folders besides exempt ones (`tests/`, `generated/`), such as
a Rust module that holds only source files. The [README standard](/documentation_v2/standards/readme_standard.md)
defines every rule this template follows; the leaf kind adds no sections of its own.

## Skeleton

Copy the block and replace every `<…>` placeholder; each one says what goes there. How it works may
be left out when the folder holds at most three files (README.md and exempt folders not counted).

````markdown
# <What the folder holds, in plain words: no path, no backticks>

<One to three sentences: what this folder is for.>

## Contents

```text
<repository path of the folder>/
├── <file name or glob>  <what it is for: a lowercase phrase, no closing period>
└── tests/               <what the unit tests cover>
```

## How it works

<How the files work together: the flow through them, the main types, the invariants that span
files. Leave the section out when the folder holds at most three files.>

## Boundaries

- Depends on: <the modules, crates and files this folder uses, read from its imports>
- Used by: <every user outside the folder, found with git grep; "nothing" when none>
- Rules: <the invariants a change here must keep>

## Related documentation

- [<document title>](/documentation_v2/<path to the document>) — <what it covers; leave the
  section out when no document goes deeper>
````

## Worked sample

Written from `apps/website/map-engine/src/spatial/los/interior/`, a leaf of three source files and
a `tests/` folder: it leaves out How it works, and it has no Related documentation because no
document covers this module. The sample sits in a fenced block, so no gate reads it as a README; the
folder's own README.md is written from the same code and may differ.

````markdown
# Line of sight inside buildings

Traces and visibility rasters through the geometry of one building: whether an observer sees a
target past walls, glass panes and foliage, and which cells of each floor an observer can see.

## Contents

```text
apps/website/map-engine/src/spatial/los/interior/
├── mod.rs     declares both modules, compiled only with the `io` feature
├── tests/     unit tests for the walker and the wash
├── walker.rs  observer-to-target traces through a compound building, with blocking and concealment
└── wash.rs    per-floor visibility rasters around an observer, whole or in budgeted batches
```

## Boundaries

- Depends on: `crate::spatial::bvh` (surface kinds, traversal hits, the sidecar mesh),
  `crate::spatial::los::terrain::viewshed` (`Visibility`, `ViewshedCapRefused`) and
  `crate::world::architecture` (building blueprints, compound buildings, their instances and rigid
  transforms).
- Used by: `crate::spatial::los::world`, whose world occluder reuses the walker's trace and
  concealment helpers; `crate::editing::tools::viewshed_scheduler`, whose building-wash lane runs a
  `WashJob` in budgeted steps; the debug building viewer (`apps/website/frontend/src/v2/apps/debug/building_viewer.rs`); the
  Mission Creator's line-of-sight tool
  (`apps/website/frontend/src/v2/apps/editor/input/tools/los_world_wasm.rs`); and the blueprint
  tooling (`tools_v2/developer-tools/src/blueprint/bvh/construction.rs`).
- Rules: `wash_cap_check` refuses a wash radius above `MAX_WASH_RADIUS_M` (400 m); a `WashJob` may
  pause at any cell, because each cell's verdict depends only on its index, the observer, the eye
  height and the blocking test; unit tests sit in `tests/`, declared from the file they test.
````
