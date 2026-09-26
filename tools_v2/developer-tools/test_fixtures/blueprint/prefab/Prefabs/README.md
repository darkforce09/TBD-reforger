# Blueprint synthetic prefab tree

A small synthetic [Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) prefab tree laid out as an
addon's `Prefabs/` folder: one wooden house with a door set, two glazed windows and a furniture
composition, plus the door variants the prefab resolver must read.

## Contents

```text
tools_v2/developer-tools/test_fixtures/blueprint/prefab/Prefabs/
├── Core/       the root prefabs the building, props and furniture inherit
├── Doors/      a door frame with its leaf, and plain and sliding leaf variants
├── Furniture/  a furniture composition of a table and a `$grp` of two chairs
├── Houses/     the base house and the wooden variant the tests start from
├── Props/      the table and chair the furniture composition places
└── Windows/    a window frame with two glass panes, and their bases
```

## How it works

Every prefab names its base and its children by resource path (`"{GUID}Prefabs/…/X.et"`), so the
tree resolves inside itself. `House_Wood.et` is the entry point; the resolver follows each base
chain and the walker follows each child:

```text
Houses/House_Wood.et ─▶ Houses/House_Base.et ─▶ Core/Building_Base.et
  ├─ socket_door_left_01: Doors/DoorSet.et ─▶ Doors/DoorSet_Base.et
  │    └─ socket_door_LEFT: Doors/Door_Leaf.et ─▶ Doors/Door_Base.et
  ├─ socket_win_01, socket_win_02: Windows/Window.et ─▶ Windows/Window_Base.et
  │    └─ socket_glass_001, socket_glass_002: Windows/Glass.et ─▶ Windows/Glass_Base.et
  ├─ probe: Prefabs/Core/Probe.et (not in the tree; the walker records a note)
  └─ F1: Furniture/Furniture_01.et ─▶ Core/Furniture_base.et
       ├─ T1: Props/Table.et ─▶ Core/Prop_Base.et
       └─ C1, C2: Props/Chair.et ─▶ Core/Prop_Base.et
```

The socket children and the probe come from `House_Base.et`, and the furniture composition from
`House_Wood.et`. `Doors/Door_Plain.et` and `Doors/Door_Sliding.et` sit outside the house; the
resolver test resolves them alone.

## Format

- Encoding: ASCII Enfusion prefab text (`.et`), one root entity per file, with invented IDs, GUIDs
  and resource paths; nothing here is game content. Each subfolder groups one kind of prefab.
- Schema: the `.et` grammar that `parse_et` in
  `tools_v2/developer-tools/src/blueprint/bvh/prefab_catalog/tokenize.rs` reads: class heads with
  an optional base, `components`, `SlotBoneMappings`, anonymous child lists, `$grp` groups,
  `Hierarchy` `PivotID`, `coords`, `angles` and `scale`, and door components.
- Adding a file: put it in the subfolder of its kind, reference it by `"{GUID}Prefabs/<kind>/<name>.et"`
  from the prefab that uses it, and run `cargo test -p developer-tools blueprint::prefab` and
  `cargo test -p developer-tools blueprint::batch`.

## Producers and consumers

- Producers: people; the files are written by hand.
- Consumers:
  - `resolver_walks_inheritance_sockets_and_children` in
    `tools_v2/developer-tools/src/blueprint/tests/prefab/tests.rs`;
  - `walker_places_door_set_window_and_furniture_from_fixtures` in
    `tools_v2/developer-tools/src/blueprint/tests/batch_tests.rs`;
  - `compiler_fixtures_resolve_from_root_crate_and_source_directory` in
    `tools_v2/developer-tools/src/tests/repository_paths.rs`, which checks that
    `Houses/House_Wood.et` is reachable.

## Boundaries

- Depends on: the `.et` grammar `parse_et` reads, and the `PrefabResolver` and `Walker` in
  `tools_v2/developer-tools/src/blueprint/bvh/`.
- Used by: the three tests above, through the prefab folder root one level up.
- Rules: every reference resolves inside the tree except `Prefabs/Core/Probe.et` and the component
  bases (`.ct`), which no test reads; the socket names match the synthetic models the walker test
  builds.
