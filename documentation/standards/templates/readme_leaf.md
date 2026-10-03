**Status:** live

# README template: leaf

**When to use:** a folder with no child folders besides exempt ones (`tests/`, `generated/`), such as
a Rust module that holds only source files. The [README standard](/documentation/standards/readme_standard.md)
defines every rule this template follows; the leaf kind adds no sections of its own.

## Skeleton

Copy the block and replace every `<…>` placeholder; each one says what goes there. How it works may
be left out by a folder with no child folders besides exempt ones and at most three files, whatever
its kind (README.md not counted).

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
files. A folder with no child folders besides exempt ones and at most three files, whatever its
kind, may leave the section out (README.md not counted).>

## Boundaries

- Depends on: <the modules, crates and files this folder uses, read from its imports>
- Used by: <every user outside the folder, found with git grep; "nothing" when none>
- Rules: <the invariants a change here must keep>

## Related documentation

- [<document title>](/documentation/<path to the document>) — <what it covers; leave the
  section out when no document goes deeper>
````

## Worked sample

Written from `crates/line_of_sight/interior_line_of_sight/src/`, a leaf of six source files and
a `tests/` folder: it leaves out How it works, and it has no Related documentation because no
document covers this module. The sample sits in a fenced block, so no gate reads it as a README; the
folder's own README.md is written from the same code and may differ.

````markdown
# Line of sight inside buildings

Traces and visibility rasters through the geometry of one building: whether an observer sees a
target past walls, glass panes and foliage, and which cells of each floor an observer can see.

## Contents

```text
crates/line_of_sight/interior_line_of_sight/src/
├── compound_walk.rs          observer-to-target traces through a compound building, with blocking
├── error.rs                  `Error` and `Result`: a `ViewshedCapRefused` behind one type
├── floor_wash.rs             per-floor visibility rasters around an observer, whole or in batches
├── lib.rs                    the crate root: module header, `mod` lines and re-exports
├── prelude.rs                the names most readers import
├── sight_line_evaluation.rs  crossings reduced to named hits and concealment
└── tests/                    unit tests for the compound walk and the floor wash
```

## Boundaries

- Depends on: `building_interiors` (building blueprints, compound buildings and their instances),
  `spatial_indexes` (surface kinds, traversal hits, the sidecar mesh), `terrain_line_of_sight`
  (`Visibility`, `ViewshedCapRefused`), `geometry_primitives` (rigid transforms) and `thiserror`.
- Used by: `world_line_of_sight`, whose world occluder reuses the compound walk's trace and
  concealment helpers; the map engine's visibility scheduler, whose building-wash lane runs a
  `WashJob` in budgeted steps; the debug building viewer
  (`apps/frontend/src/workspaces/debug/building_viewer.rs`); the Mission Creator's
  line-of-sight tool (`apps/frontend/src/workspaces/editor/input/tools/los_world_wasm.rs`);
  and the blueprint tooling (`tools/developer_tools/src/blueprint/bvh/construction.rs`).
- Rules: `wash_cap_check` refuses a wash radius above `MAX_WASH_RADIUS_M` (400 m)
  (`over_cap_wash_radius_is_refused_with_a_message` in `tests/floor_wash_tests.rs`); a `WashJob`
  may pause at any cell, because each cell's verdict depends only on its index, the observer, the
  eye height and the blocking test (`sliced_wash_is_bit_identical_to_the_sync_path` holds that).
````
